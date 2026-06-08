use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::model::variable::{VarDef, VarType, VarValue};
use crate::plc::client::PlcClient;
use crate::plc::parser::parse_var;
use crate::state::{ConnectionStatus, LiveVar, SharedState};

/// Spawn the background polling thread.
///
/// The thread idles when `polling_active` is false or `status` is not `Connected`.
/// Lock is held only for the minimum duration — never across FFI calls or sleeps.
pub fn spawn_poller(state: Arc<Mutex<SharedState>>) -> JoinHandle<()> {
    thread::spawn(move || {
        #[cfg(test)]
        let mut client = PlcClient::new_mock();
        #[cfg(not(test))]
        let mut client = PlcClient::new();

        let mut error_count: u32 = 0;

        loop {
            // Phase 1: read config under a short lock.
            let snapshot = match state.lock() {
                Ok(s) => Some((
                    s.poll_interval_ms,
                    s.polling_active,
                    s.status.clone(),
                    s.var_defs.clone(),
                    s.config.clone(),
                )),
                Err(_) => None, // poisoned — exit thread
            };

            let (poll_interval_ms, polling_active, status, var_defs, config) = match snapshot {
                Some(t) => t,
                None => break,
            };

            if !polling_active {
                thread::sleep(Duration::from_millis(poll_interval_ms));
                continue;
            }

            if status != ConnectionStatus::Connected {
                thread::sleep(Duration::from_millis(poll_interval_ms));
                continue;
            }

            if var_defs.is_empty() {
                thread::sleep(Duration::from_millis(poll_interval_ms));
                continue;
            }

            let db_number = config.db_number as i32;
            let size = compute_read_size(&var_defs);

            // Phase 2: I/O — no lock held.
            match client.read_db(db_number, size) {
                Ok(bytes) => {
                    error_count = 0;
                    let now = Instant::now();

                    if let Ok(mut s) = state.lock() {
                        // Snapshot previous blink states before replacing live_vars.
                        let prev: Vec<Option<(bool, bool)>> = (0..var_defs.len())
                            .map(|i| {
                                s.live_vars.get(i).and_then(|lv| match &lv.value {
                                    VarValue::Bool { value, blink_on } => {
                                        Some((*value, *blink_on))
                                    }
                                    _ => None,
                                })
                            })
                            .collect();

                        s.live_vars = var_defs
                            .iter()
                            .enumerate()
                            .map(|(i, def)| {
                                let mut new_value = parse_var(&bytes, def);
                                apply_blink(&mut new_value, prev.get(i).copied().flatten());
                                LiveVar { def: def.clone(), value: new_value, last_updated: now }
                            })
                            .collect();
                    }
                }
                Err(_) => {
                    error_count += 1;
                    if error_count >= 3 {
                        if let Ok(mut s) = state.lock() {
                            s.status = ConnectionStatus::Error("read failed".into());
                        }
                        // Attempt reconnect — no lock held during connect.
                        if client.connect(&config).is_ok() {
                            error_count = 0;
                            if let Ok(mut s) = state.lock() {
                                s.status = ConnectionStatus::Connected;
                            }
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(poll_interval_ms));
        }
    })
}

/// Update the blink state of a Bool value based on its previous `(value, blink_on)`.
///
/// - Value changed → blink_on = true (alarm just fired)
/// - Value still true → toggle blink_on (visual heartbeat)
/// - Value false → blink_on stays false (already set by parse_var)
fn apply_blink(new_value: &mut VarValue, prev: Option<(bool, bool)>) {
    if let VarValue::Bool { value, blink_on } = new_value {
        let (prev_val, prev_blink) = prev.unwrap_or((false, false));
        if *value != prev_val {
            *blink_on = true;
        } else if *value {
            *blink_on = !prev_blink;
        }
    }
}

/// Compute the minimum DB read size covering all variable definitions.
fn compute_read_size(var_defs: &[VarDef]) -> usize {
    var_defs
        .iter()
        .map(|def| {
            let base = def.byte_offset as usize;
            let type_size = match &def.var_type {
                VarType::Bool | VarType::Byte => 1,
                VarType::Word | VarType::Int => 2,
                VarType::DWord | VarType::DInt | VarType::Real => 4,
                VarType::String { length } => *length as usize,
            };
            base + type_size
        })
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::variable::{VarDef, VarType, VarValue};
    use crate::state::{ConnectionStatus, SharedState};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn compute_read_size_single_bool() {
        let defs = vec![VarDef { name: "f".into(), var_type: VarType::Bool, byte_offset: 5, bit_offset: 0 }];
        assert_eq!(compute_read_size(&defs), 6);
    }

    #[test]
    fn compute_read_size_word_and_real() {
        let defs = vec![
            VarDef { name: "w".into(), var_type: VarType::Word, byte_offset: 0, bit_offset: 0 },
            VarDef { name: "r".into(), var_type: VarType::Real, byte_offset: 10, bit_offset: 0 },
        ];
        assert_eq!(compute_read_size(&defs), 14); // 10 + 4
    }

    #[test]
    fn apply_blink_value_changed_sets_blink_on() {
        let mut v = VarValue::Bool { value: true, blink_on: false };
        apply_blink(&mut v, Some((false, false))); // false → true
        assert_eq!(v, VarValue::Bool { value: true, blink_on: true });
    }

    #[test]
    fn apply_blink_value_true_toggles() {
        let mut v = VarValue::Bool { value: true, blink_on: false };
        apply_blink(&mut v, Some((true, true))); // stays true, was blink_on=true
        assert_eq!(v, VarValue::Bool { value: true, blink_on: false });
    }

    #[test]
    fn apply_blink_value_false_stays_off() {
        let mut v = VarValue::Bool { value: false, blink_on: false };
        apply_blink(&mut v, Some((false, true)));
        assert_eq!(v, VarValue::Bool { value: false, blink_on: false });
    }

    #[test]
    fn poller_updates_live_vars_in_mock_mode() {
        let mut s = SharedState::new();
        s.status = ConnectionStatus::Connected;
        s.polling_active = true;
        s.poll_interval_ms = 5;
        s.var_defs = vec![
            VarDef { name: "flag".into(), var_type: VarType::Bool, byte_offset: 0, bit_offset: 0 },
            VarDef { name: "raw".into(), var_type: VarType::Byte, byte_offset: 1, bit_offset: 0 },
            VarDef { name: "counter".into(), var_type: VarType::Word, byte_offset: 2, bit_offset: 0 },
        ];

        let state = Arc::new(Mutex::new(s));
        let _handle = spawn_poller(Arc::clone(&state));

        // Allow at least 3 poll cycles (3 × 5 ms + scheduling margin).
        thread::sleep(Duration::from_millis(80));

        let s = state.lock().expect("state poisoned");
        assert_eq!(s.live_vars.len(), 3, "poller must populate all live_vars");
        assert!(matches!(s.live_vars[0].value, VarValue::Bool { .. }));
        assert!(matches!(s.live_vars[1].value, VarValue::Byte(_)));
        assert!(matches!(s.live_vars[2].value, VarValue::Word(_)));
    }
}
