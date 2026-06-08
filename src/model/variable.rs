use serde::{Deserialize, Serialize};
use std::fmt;

/// PLC variable data type, including String with a fixed max length.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "length")]
pub enum VarType {
    Bool,
    Byte,
    Word,
    Int,
    DWord,
    DInt,
    Real,
    String { length: u32 },
}

impl fmt::Display for VarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VarType::Bool => write!(f, "Bool"),
            VarType::Byte => write!(f, "Byte"),
            VarType::Word => write!(f, "Word"),
            VarType::Int => write!(f, "Int"),
            VarType::DWord => write!(f, "DWord"),
            VarType::DInt => write!(f, "DInt"),
            VarType::Real => write!(f, "Real"),
            VarType::String { length } => write!(f, "String[{}]", length),
        }
    }
}

/// A single variable definition within a DB block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VarDef {
    pub name: String,
    pub var_type: VarType,
    pub byte_offset: u32,
    pub bit_offset: u8,
}

impl Default for VarDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            var_type: VarType::Bool,
            byte_offset: 0,
            bit_offset: 0,
        }
    }
}

/// A live-read value for a variable, including blink state for Bool alarms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VarValue {
    Bool { value: bool, blink_on: bool },
    Byte(u8),
    Word(u16),
    Int(i16),
    DWord(u32),
    DInt(i32),
    Real(f32),
    StringVal(String),
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(var_def: &VarDef) {
        let json = serde_json::to_string(var_def).unwrap();
        let restored: VarDef = serde_json::from_str(&json).unwrap();
        assert_eq!(var_def, &restored);
    }

    #[test]
    fn serde_bool() {
        round_trip(&VarDef { name: "flag".into(), var_type: VarType::Bool, byte_offset: 0, bit_offset: 3 });
    }

    #[test]
    fn serde_byte() {
        round_trip(&VarDef { name: "raw".into(), var_type: VarType::Byte, byte_offset: 1, bit_offset: 0 });
    }

    #[test]
    fn serde_word() {
        round_trip(&VarDef { name: "status".into(), var_type: VarType::Word, byte_offset: 2, bit_offset: 0 });
    }

    #[test]
    fn serde_int() {
        round_trip(&VarDef { name: "temp".into(), var_type: VarType::Int, byte_offset: 4, bit_offset: 0 });
    }

    #[test]
    fn serde_dword() {
        round_trip(&VarDef { name: "counter".into(), var_type: VarType::DWord, byte_offset: 6, bit_offset: 0 });
    }

    #[test]
    fn serde_dint() {
        round_trip(&VarDef { name: "position".into(), var_type: VarType::DInt, byte_offset: 10, bit_offset: 0 });
    }

    #[test]
    fn serde_real() {
        round_trip(&VarDef { name: "pressure".into(), var_type: VarType::Real, byte_offset: 14, bit_offset: 0 });
    }

    #[test]
    fn serde_string() {
        round_trip(&VarDef {
            name: "tag".into(),
            var_type: VarType::String { length: 32 },
            byte_offset: 18,
            bit_offset: 0,
        });
    }

    #[test]
    fn display_string_type() {
        let t = VarType::String { length: 64 };
        assert_eq!(t.to_string(), "String[64]");
    }

    #[test]
    fn default_vardef() {
        let d = VarDef::default();
        assert_eq!(d.name, "");
        assert_eq!(d.var_type, VarType::Bool);
        assert_eq!(d.byte_offset, 0);
        assert_eq!(d.bit_offset, 0);
    }
}
