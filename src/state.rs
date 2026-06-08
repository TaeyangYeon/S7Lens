use std::time::Instant;

use crate::model::session::ConnectionConfig;
use crate::model::variable::{VarDef, VarValue};

/// Connectivity state of the PLC client.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// A live-monitored variable: its definition, current parsed value, and when it was last read.
pub struct LiveVar {
    pub def: VarDef,
    pub value: VarValue,
    pub last_updated: Instant,
}

/// Application-wide shared state, intended to live inside `Arc<Mutex<SharedState>>`.
///
/// The poller thread reads `config`, `var_defs`, `poll_interval_ms`, `polling_active`,
/// and `status` under a short lock, then writes `live_vars` under a separate lock
/// after completing I/O. Never hold the lock across FFI calls or `thread::sleep`.
pub struct SharedState {
    pub config: ConnectionConfig,
    pub var_defs: Vec<VarDef>,
    pub live_vars: Vec<LiveVar>,
    pub status: ConnectionStatus,
    pub poll_interval_ms: u64,
    pub polling_active: bool,
}

impl SharedState {
    /// Create a `SharedState` with sensible defaults.
    ///
    /// Polling starts inactive; the UI must set `polling_active = true` to begin.
    pub fn new() -> Self {
        let config = ConnectionConfig::default();
        let poll_interval_ms = config.poll_interval_ms;
        Self {
            config,
            var_defs: Vec::new(),
            live_vars: Vec::new(),
            status: ConnectionStatus::Disconnected,
            poll_interval_ms,
            polling_active: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::variable::{VarDef, VarType, VarValue};

    #[test]
    fn shared_state_new_defaults() {
        let s = SharedState::new();
        assert_eq!(s.status, ConnectionStatus::Disconnected);
        assert!(!s.polling_active);
        assert!(s.var_defs.is_empty());
        assert!(s.live_vars.is_empty());
        assert_eq!(s.poll_interval_ms, ConnectionConfig::default().poll_interval_ms);
    }

    #[test]
    fn live_var_construction() {
        let def = VarDef { name: "flag".into(), var_type: VarType::Bool, byte_offset: 0, bit_offset: 0 };
        let value = VarValue::Bool { value: true, blink_on: false };
        let lv = LiveVar { def: def.clone(), value: value.clone(), last_updated: Instant::now() };
        assert_eq!(lv.def, def);
        assert_eq!(lv.value, value);
    }

    #[test]
    fn connection_status_clone_and_eq() {
        let s1 = ConnectionStatus::Error("timeout".into());
        let s2 = s1.clone();
        assert_eq!(s1, s2);
        assert_ne!(s1, ConnectionStatus::Disconnected);
    }
}
