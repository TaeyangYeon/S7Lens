use serde::{Deserialize, Serialize};

/// Connection parameters for a Siemens S7 PLC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub ip: String,
    pub rack: u16,
    pub slot: u16,
    pub db_number: u32,
    pub poll_interval_ms: u64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            ip: "192.168.0.1".to_string(),
            rack: 0,
            slot: 1,
            db_number: 100,
            poll_interval_ms: 100,
        }
    }
}
