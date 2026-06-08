use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::config::{load_config, save_config, ConfigFile};
use crate::model::variable::{VarDef, VarType, VarValue};
use crate::state::{ConnectionStatus, SharedState};

/// Draft UI state for the connection panel — lives on the app struct, not SharedState.
struct ConnectionDraft {
    ip: String,
    rack: String,
    slot: String,
    db_number: String,
}

impl ConnectionDraft {
    fn from_defaults() -> Self {
        let cfg = crate::model::session::ConnectionConfig::default();
        Self {
            ip: cfg.ip,
            rack: cfg.rack.to_string(),
            slot: cfg.slot.to_string(),
            db_number: cfg.db_number.to_string(),
        }
    }
}

/// Main egui application struct.
pub struct PlcMonitorApp {
    state: Arc<Mutex<SharedState>>,
    draft: ConnectionDraft,
    /// File path shown in the config save/load input.
    config_path: String,
    /// One-line status message shown below the config buttons.
    config_status: String,
    /// Draft string for the poll interval text input.
    poll_ms_draft: String,
}

impl PlcMonitorApp {
    /// Create a new app, initialising draft inputs from `ConnectionConfig::default()`.
    pub fn new(_cc: &eframe::CreationContext<'_>, state: Arc<Mutex<SharedState>>) -> Self {
        let poll_ms = state.lock().map(|s| s.poll_interval_ms).unwrap_or(100);
        Self {
            state,
            draft: ConnectionDraft::from_defaults(),
            config_path: "config.json".to_string(),
            config_status: String::new(),
            poll_ms_draft: poll_ms.to_string(),
        }
    }

    fn render_connection_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("IP:");
            ui.text_edit_singleline(&mut self.draft.ip);
            ui.label("Rack:");
            ui.add(egui::TextEdit::singleline(&mut self.draft.rack).desired_width(40.0));
            ui.label("Slot:");
            ui.add(egui::TextEdit::singleline(&mut self.draft.slot).desired_width(40.0));
            ui.label("DB:");
            ui.add(egui::TextEdit::singleline(&mut self.draft.db_number).desired_width(60.0));
        });

        ui.horizontal(|ui| {
            if ui.button("Connect").clicked() {
                let rack: u16 = self.draft.rack.parse().unwrap_or(0);
                let slot: u16 = self.draft.slot.parse().unwrap_or(1);
                let db_number: u32 = self.draft.db_number.parse().unwrap_or(100);

                if let Ok(mut s) = self.state.lock() {
                    s.config.ip = self.draft.ip.clone();
                    s.config.rack = rack;
                    s.config.slot = slot;
                    s.config.db_number = db_number;
                    s.polling_active = true;
                    s.status = ConnectionStatus::Connecting;
                }
            }

            if ui.button("Disconnect").clicked() {
                if let Ok(mut s) = self.state.lock() {
                    s.polling_active = false;
                    s.status = ConnectionStatus::Disconnected;
                }
            }

            // Status indicator
            let status = self
                .state
                .lock()
                .map(|s| s.status.clone())
                .unwrap_or(ConnectionStatus::Disconnected);

            match &status {
                ConnectionStatus::Connected => {
                    ui.colored_label(egui::Color32::GREEN, "● Connected");
                }
                ConnectionStatus::Connecting => {
                    ui.colored_label(egui::Color32::YELLOW, "⟳ Connecting...");
                }
                ConnectionStatus::Error(msg) => {
                    ui.colored_label(egui::Color32::RED, format!("✗ {}", msg));
                }
                ConnectionStatus::Disconnected => {
                    ui.colored_label(egui::Color32::GRAY, "○ Disconnected");
                }
            }
        });
    }

    fn render_var_def_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ Add Row").clicked() {
                    if let Ok(mut s) = self.state.lock() {
                        s.var_defs.push(VarDef::default());
                    }
                }
            });
        });

        // Snapshot var_defs so we can render without holding the lock across the entire table.
        let var_defs_snapshot: Vec<VarDef> = self
            .state
            .lock()
            .map(|s| s.var_defs.clone())
            .unwrap_or_default();

        let mut to_delete: Option<usize> = None;
        // Collect mutations to apply after rendering.
        let mut mutations: Vec<(usize, VarDef)> = Vec::new();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .column(Column::initial(120.0)) // Name
            .column(Column::initial(110.0)) // Type
            .column(Column::initial(90.0))  // Byte Offset
            .column(Column::initial(70.0))  // Bit Offset
            .column(Column::initial(70.0))  // Length
            .column(Column::initial(30.0))  // Delete
            .header(20.0, |mut header| {
                header.col(|ui| { ui.strong("Name"); });
                header.col(|ui| { ui.strong("Type"); });
                header.col(|ui| { ui.strong("Byte Offset"); });
                header.col(|ui| { ui.strong("Bit Offset"); });
                header.col(|ui| { ui.strong("Length"); });
                header.col(|ui| { ui.strong(""); });
            })
            .body(|mut body| {
                for (i, def) in var_defs_snapshot.iter().enumerate() {
                    let mut def_mut = def.clone();
                    let mut changed = false;

                    body.row(22.0, |mut row| {
                        row.col(|ui| {
                            if ui.text_edit_singleline(&mut def_mut.name).changed() {
                                changed = true;
                            }
                        });

                        row.col(|ui| {
                            let selected = type_kind_label(&def_mut.var_type);
                            egui::ComboBox::from_id_salt(format!("type_{}", i))
                                .selected_text(selected)
                                .show_ui(ui, |ui| {
                                    for kind in ALL_TYPE_KINDS {
                                        if ui.selectable_label(type_kind_label(&def_mut.var_type) == *kind, *kind).clicked() {
                                            def_mut.var_type = kind_to_var_type(kind, &def_mut.var_type);
                                            changed = true;
                                        }
                                    }
                                });
                        });

                        row.col(|ui| {
                            let mut byte_off = def_mut.byte_offset;
                            if ui.add(egui::DragValue::new(&mut byte_off).range(0..=u32::MAX)).changed() {
                                def_mut.byte_offset = byte_off;
                                changed = true;
                            }
                        });

                        row.col(|ui| {
                            if matches!(def_mut.var_type, VarType::Bool) {
                                let mut bit_off = def_mut.bit_offset;
                                if ui.add(egui::DragValue::new(&mut bit_off).range(0..=7u8)).changed() {
                                    def_mut.bit_offset = bit_off;
                                    changed = true;
                                }
                            } else {
                                ui.add_enabled(false, egui::Label::new("-"));
                            }
                        });

                        row.col(|ui| {
                            if let VarType::String { length } = &mut def_mut.var_type {
                                let mut len = *length;
                                if ui.add(egui::DragValue::new(&mut len).range(1..=u32::MAX)).changed() {
                                    *length = len;
                                    changed = true;
                                }
                            } else {
                                ui.add_enabled(false, egui::Label::new("-"));
                            }
                        });

                        row.col(|ui| {
                            if ui.button("✕").clicked() {
                                to_delete = Some(i);
                            }
                        });
                    });

                    if changed {
                        mutations.push((i, def_mut));
                    }
                }
            });

        // Apply mutations and deletions under a single lock.
        if !mutations.is_empty() || to_delete.is_some() {
            if let Ok(mut s) = self.state.lock() {
                for (i, def) in mutations {
                    if let Some(existing) = s.var_defs.get_mut(i) {
                        *existing = def;
                    }
                }
                if let Some(idx) = to_delete {
                    if idx < s.var_defs.len() {
                        s.var_defs.remove(idx);
                    }
                }
            }
        }
    }

    fn render_live_monitor_panel(&mut self, ui: &mut egui::Ui) {
        // Poll controls row
        ui.horizontal(|ui| {
            ui.label("Poll ms:");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.poll_ms_draft).desired_width(60.0),
            );
            if resp.lost_focus() {
                if let Ok(ms) = self.poll_ms_draft.parse::<u64>() {
                    if let Ok(mut s) = self.state.lock() {
                        s.poll_interval_ms = ms;
                        s.config.poll_interval_ms = ms;
                    }
                }
            }

            if ui.button("▶ Start").clicked() {
                if let Ok(mut s) = self.state.lock() {
                    s.polling_active = true;
                }
            }
            if ui.button("■ Stop").clicked() {
                if let Ok(mut s) = self.state.lock() {
                    s.polling_active = false;
                }
            }
        });

        // Snapshot var_defs, live values, and status in one short lock.
        let (var_defs_snap, live_vals, status) = self
            .state
            .lock()
            .map(|s| {
                let var_defs = s.var_defs.clone();
                let live_vals: Vec<VarValue> =
                    s.live_vars.iter().map(|lv| lv.value.clone()).collect();
                let status = s.status.clone();
                (var_defs, live_vals, status)
            })
            .unwrap_or_else(|_| (Vec::new(), Vec::new(), ConnectionStatus::Disconnected));

        let connected = status == ConnectionStatus::Connected;

        TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .column(Column::initial(120.0)) // Name
            .column(Column::initial(80.0))  // Type
            .column(Column::remainder())    // Value
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Name");
                });
                header.col(|ui| {
                    ui.strong("Type");
                });
                header.col(|ui| {
                    ui.strong("Value");
                });
            })
            .body(|mut body| {
                for (i, def) in var_defs_snap.iter().enumerate() {
                    let value_opt = if connected { live_vals.get(i) } else { None };
                    body.row(22.0, |mut row| {
                        row.col(|ui| {
                            ui.label(&def.name);
                        });
                        row.col(|ui| {
                            ui.label(def.var_type.to_string());
                        });
                        row.col(|ui| {
                            match value_opt {
                                None => {
                                    ui.label("--");
                                }
                                Some(VarValue::Bool { blink_on, .. }) => {
                                    if *blink_on {
                                        ui.colored_label(egui::Color32::GREEN, "●TRUE");
                                    } else {
                                        ui.colored_label(egui::Color32::GRAY, "○FALSE");
                                    }
                                }
                                Some(v) => {
                                    ui.label(format_var_value(v));
                                }
                            }
                        });
                    });
                }
            });
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Config file:");
            ui.add(
                egui::TextEdit::singleline(&mut self.config_path).desired_width(200.0),
            );

            if ui.button("💾 Save Config").clicked() {
                let cfg_opt = self.state.lock().ok().map(|s| ConfigFile {
                    connection: s.config.clone(),
                    vars: s.var_defs.clone(),
                });
                match cfg_opt {
                    Some(cfg) => {
                        let path = std::path::Path::new(&self.config_path);
                        match save_config(path, &cfg) {
                            Ok(()) => {
                                self.config_status = format!("Saved to {}", self.config_path);
                            }
                            Err(e) => {
                                self.config_status = format!("Save error: {}", e);
                            }
                        }
                    }
                    None => {
                        self.config_status = "State lock failed".to_string();
                    }
                }
            }

            if ui.button("📂 Load Config").clicked() {
                let path_str = self.config_path.clone();
                let path = std::path::Path::new(&path_str);
                match load_config(path) {
                    Ok(cfg) => {
                        self.poll_ms_draft = cfg.connection.poll_interval_ms.to_string();
                        if let Ok(mut s) = self.state.lock() {
                            s.poll_interval_ms = cfg.connection.poll_interval_ms;
                            s.config = cfg.connection;
                            s.var_defs = cfg.vars;
                        }
                        self.config_status = format!("Loaded from {}", self.config_path);
                    }
                    Err(e) => {
                        self.config_status = format!("Load error: {}", e);
                    }
                }
            }

            if ui.button("📤 Export C# Class").clicked() {
                println!("export");
            }
        });

        if !self.config_status.is_empty() {
            ui.label(&self.config_status);
        }
    }
}

impl eframe::App for PlcMonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Schedule periodic repaints whenever Bool variables are defined,
        // so the blink_on toggle from the poller thread becomes visible.
        let has_bool = self
            .state
            .lock()
            .map(|s| s.var_defs.iter().any(|d| matches!(d.var_type, VarType::Bool)))
            .unwrap_or(false);
        if has_bool {
            ctx.request_repaint_after(Duration::from_millis(500));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::CollapsingHeader::new("🔌 Connection")
                .default_open(true)
                .show(ui, |ui| {
                    self.render_connection_panel(ui);
                });

            ui.add_space(4.0);

            egui::CollapsingHeader::new("📋 Variable Definitions")
                .default_open(true)
                .show(ui, |ui| {
                    self.render_var_def_panel(ui);
                });

            ui.add_space(4.0);

            egui::CollapsingHeader::new("📊 Live Monitor")
                .default_open(true)
                .show(ui, |ui| {
                    self.render_live_monitor_panel(ui);
                });

            ui.add_space(4.0);
            ui.separator();

            self.render_toolbar(ui);
        });
    }
}

/// Format a `VarValue` as a display string (no colour information).
///
/// For Bool: `blink_on` determines the label — callers apply colour separately.
pub fn format_var_value(value: &VarValue) -> String {
    match value {
        VarValue::Bool { blink_on, .. } => {
            if *blink_on { "●TRUE".to_string() } else { "○FALSE".to_string() }
        }
        VarValue::Byte(v) => v.to_string(),
        VarValue::Word(v) => format!("0x{:04X}  ({})", v, v),
        VarValue::Int(v) => v.to_string(),
        VarValue::DWord(v) => format!("0x{:08X}  ({})", v, v),
        VarValue::DInt(v) => v.to_string(),
        VarValue::Real(v) => format!("{:.3}", v),
        VarValue::StringVal(s) => format!("\"{}\"", s),
        VarValue::Unknown => "--".to_string(),
    }
}

const ALL_TYPE_KINDS: &[&str] = &[
    "Bool", "Byte", "Word", "Int", "DWord", "DInt", "Real", "String",
];

fn type_kind_label(vt: &VarType) -> &'static str {
    match vt {
        VarType::Bool => "Bool",
        VarType::Byte => "Byte",
        VarType::Word => "Word",
        VarType::Int => "Int",
        VarType::DWord => "DWord",
        VarType::DInt => "DInt",
        VarType::Real => "Real",
        VarType::String { .. } => "String",
    }
}

/// Convert a kind label string to a `VarType`, preserving String length when possible.
fn kind_to_var_type(kind: &str, current: &VarType) -> VarType {
    match kind {
        "Bool" => VarType::Bool,
        "Byte" => VarType::Byte,
        "Word" => VarType::Word,
        "Int" => VarType::Int,
        "DWord" => VarType::DWord,
        "DInt" => VarType::DInt,
        "Real" => VarType::Real,
        "String" => {
            if let VarType::String { length } = current {
                VarType::String { length: *length }
            } else {
                VarType::String { length: 32 }
            }
        }
        _ => VarType::Bool,
    }
}

/// Return the display string for a `ConnectionStatus`.
pub fn status_display(status: &ConnectionStatus) -> String {
    match status {
        ConnectionStatus::Connected => "● Connected".to_string(),
        ConnectionStatus::Connecting => "⟳ Connecting...".to_string(),
        ConnectionStatus::Error(msg) => format!("✗ {}", msg),
        ConnectionStatus::Disconnected => "○ Disconnected".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::session::ConnectionConfig;
    use crate::model::variable::VarDef;
    use crate::state::{ConnectionStatus, SharedState};
    use std::sync::{Arc, Mutex};

    fn make_app() -> PlcMonitorApp {
        let state = Arc::new(Mutex::new(SharedState::new()));
        let poll_ms = state.lock().map(|s| s.poll_interval_ms).unwrap_or(100);
        PlcMonitorApp {
            state,
            draft: ConnectionDraft::from_defaults(),
            config_path: "config.json".to_string(),
            config_status: String::new(),
            poll_ms_draft: poll_ms.to_string(),
        }
    }

    #[test]
    fn test_app_default_inputs() {
        let app = make_app();
        let cfg = ConnectionConfig::default();
        assert_eq!(app.draft.ip, cfg.ip);
        assert_eq!(app.draft.rack, cfg.rack.to_string());
        assert_eq!(app.draft.slot, cfg.slot.to_string());
        assert_eq!(app.draft.db_number, cfg.db_number.to_string());
    }

    #[test]
    fn test_add_remove_var_def() {
        let app = make_app();
        {
            let mut s = app.state.lock().expect("lock poisoned");
            s.var_defs.push(VarDef::default());
        }
        assert_eq!(app.state.lock().expect("lock").var_defs.len(), 1);
        {
            let mut s = app.state.lock().expect("lock poisoned");
            s.var_defs.remove(0);
        }
        assert_eq!(app.state.lock().expect("lock").var_defs.len(), 0);
    }

    #[test]
    fn test_connection_status_display() {
        assert_eq!(status_display(&ConnectionStatus::Connected), "● Connected");
        assert_eq!(status_display(&ConnectionStatus::Connecting), "⟳ Connecting...");
        assert_eq!(status_display(&ConnectionStatus::Error("timeout".into())), "✗ timeout");
        assert_eq!(status_display(&ConnectionStatus::Disconnected), "○ Disconnected");
    }

    // --- format_var_value tests ---

    #[test]
    fn format_bool_blink_on_true() {
        assert_eq!(format_var_value(&VarValue::Bool { value: true, blink_on: true }), "●TRUE");
    }

    #[test]
    fn format_bool_blink_on_false() {
        assert_eq!(format_var_value(&VarValue::Bool { value: false, blink_on: false }), "○FALSE");
    }

    #[test]
    fn format_byte() {
        assert_eq!(format_var_value(&VarValue::Byte(255)), "255");
        assert_eq!(format_var_value(&VarValue::Byte(0)), "0");
    }

    #[test]
    fn format_word() {
        assert_eq!(format_var_value(&VarValue::Word(0x1234)), "0x1234  (4660)");
    }

    #[test]
    fn format_int_signed() {
        assert_eq!(format_var_value(&VarValue::Int(-5)), "-5");
        assert_eq!(format_var_value(&VarValue::Int(1000)), "1000");
    }

    #[test]
    fn format_dword() {
        assert_eq!(
            format_var_value(&VarValue::DWord(0xDEAD_BEEF)),
            "0xDEADBEEF  (3735928559)"
        );
    }

    #[test]
    fn format_dint_signed() {
        assert_eq!(format_var_value(&VarValue::DInt(-100_000)), "-100000");
    }

    #[test]
    fn format_real_three_decimals() {
        assert_eq!(format_var_value(&VarValue::Real(3.14159)), "3.142");
        assert_eq!(format_var_value(&VarValue::Real(0.0)), "0.000");
    }

    #[test]
    fn format_string_val() {
        assert_eq!(format_var_value(&VarValue::StringVal("hello".into())), "\"hello\"");
        assert_eq!(format_var_value(&VarValue::StringVal(String::new())), "\"\"");
    }

    #[test]
    fn format_unknown() {
        assert_eq!(format_var_value(&VarValue::Unknown), "--");
    }
}
