use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::session::ConnectionConfig;
use crate::model::variable::VarDef;

/// Serializable snapshot of the full application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub connection: ConnectionConfig,
    pub vars: Vec<VarDef>,
}

/// Serialize `cfg` to pretty-printed JSON and write it to `path`.
pub fn save_config(path: &Path, cfg: &ConfigFile) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(cfg)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Read JSON from `path` and deserialize into a `ConfigFile`.
pub fn load_config(path: &Path) -> Result<ConfigFile, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let cfg: ConfigFile = serde_json::from_str(&content)?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::variable::{VarDef, VarType};
    use std::env;

    #[test]
    fn config_round_trip() {
        let cfg = ConfigFile {
            connection: ConnectionConfig::default(),
            vars: vec![
                VarDef { name: "flag".into(), var_type: VarType::Bool, byte_offset: 0, bit_offset: 3 },
                VarDef { name: "counter".into(), var_type: VarType::Word, byte_offset: 2, bit_offset: 0 },
                VarDef { name: "temp".into(), var_type: VarType::Real, byte_offset: 4, bit_offset: 0 },
            ],
        };
        let path = env::temp_dir().join("plc_monitor_test_config_round_trip.json");
        save_config(&path, &cfg).expect("save failed");
        let loaded = load_config(&path).expect("load failed");
        assert_eq!(cfg.connection, loaded.connection);
        assert_eq!(cfg.vars, loaded.vars);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_config_missing_file_returns_error() {
        let path = Path::new("/tmp/nonexistent_plc_config_xyz987.json");
        assert!(load_config(path).is_err());
    }

    #[test]
    fn save_config_creates_file() {
        let cfg = ConfigFile { connection: ConnectionConfig::default(), vars: Vec::new() };
        let path = env::temp_dir().join("plc_monitor_test_save_creates.json");
        save_config(&path, &cfg).expect("save failed");
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }
}
