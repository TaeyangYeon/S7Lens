use crate::model::variable::{VarDef, VarType, VarValue};

/// Parse a single variable from a raw DB byte slice according to its definition.
///
/// Returns `VarValue::Unknown` on any out-of-bounds access — never panics.
/// All multi-byte types are decoded big-endian (Siemens S7 wire format).
pub fn parse_var(bytes: &[u8], def: &VarDef) -> VarValue {
    let o = def.byte_offset as usize;
    match &def.var_type {
        VarType::Bool => {
            if o >= bytes.len() {
                return VarValue::Unknown;
            }
            let value = (bytes[o] >> def.bit_offset) & 1 == 1;
            VarValue::Bool { value, blink_on: false }
        }
        VarType::Byte => {
            if o >= bytes.len() {
                return VarValue::Unknown;
            }
            VarValue::Byte(bytes[o])
        }
        VarType::Word => {
            if o + 2 > bytes.len() {
                return VarValue::Unknown;
            }
            VarValue::Word(u16::from_be_bytes([bytes[o], bytes[o + 1]]))
        }
        VarType::Int => {
            if o + 2 > bytes.len() {
                return VarValue::Unknown;
            }
            VarValue::Int(i16::from_be_bytes([bytes[o], bytes[o + 1]]))
        }
        VarType::DWord => {
            if o + 4 > bytes.len() {
                return VarValue::Unknown;
            }
            VarValue::DWord(u32::from_be_bytes([
                bytes[o],
                bytes[o + 1],
                bytes[o + 2],
                bytes[o + 3],
            ]))
        }
        VarType::DInt => {
            if o + 4 > bytes.len() {
                return VarValue::Unknown;
            }
            VarValue::DInt(i32::from_be_bytes([
                bytes[o],
                bytes[o + 1],
                bytes[o + 2],
                bytes[o + 3],
            ]))
        }
        VarType::Real => {
            if o + 4 > bytes.len() {
                return VarValue::Unknown;
            }
            VarValue::Real(f32::from_be_bytes([
                bytes[o],
                bytes[o + 1],
                bytes[o + 2],
                bytes[o + 3],
            ]))
        }
        VarType::String { length } => {
            let len = *length as usize;
            if o + len > bytes.len() {
                return VarValue::Unknown;
            }
            let s: String = bytes[o..o + len]
                .iter()
                .take_while(|&&b| b != 0)
                .map(|&b| b as char)
                .collect();
            VarValue::StringVal(s)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::variable::{VarDef, VarType, VarValue};

    fn def(var_type: VarType, byte_offset: u32, bit_offset: u8) -> VarDef {
        VarDef { name: "v".into(), var_type, byte_offset, bit_offset }
    }

    #[test]
    fn parse_bool_false() {
        let bytes = vec![0b0000_0000u8];
        let result = parse_var(&bytes, &def(VarType::Bool, 0, 3));
        assert_eq!(result, VarValue::Bool { value: false, blink_on: false });
    }

    #[test]
    fn parse_bool_true() {
        // bit 2 is set in 0b0000_0100
        let bytes = vec![0b0000_0100u8];
        let result = parse_var(&bytes, &def(VarType::Bool, 0, 2));
        assert_eq!(result, VarValue::Bool { value: true, blink_on: false });
    }

    #[test]
    fn parse_byte() {
        let bytes = vec![0x00, 0xAB];
        let result = parse_var(&bytes, &def(VarType::Byte, 1, 0));
        assert_eq!(result, VarValue::Byte(0xAB));
    }

    #[test]
    fn parse_word() {
        let bytes = vec![0x12, 0x34];
        let result = parse_var(&bytes, &def(VarType::Word, 0, 0));
        assert_eq!(result, VarValue::Word(0x1234));
    }

    #[test]
    fn parse_int_negative() {
        // i16 big-endian: 0xFF00 = -256
        let bytes = vec![0xFF, 0x00];
        let result = parse_var(&bytes, &def(VarType::Int, 0, 0));
        assert_eq!(result, VarValue::Int(-256));
    }

    #[test]
    fn parse_dword() {
        let bytes = vec![0x00, 0x01, 0x02, 0x03];
        let result = parse_var(&bytes, &def(VarType::DWord, 0, 0));
        assert_eq!(result, VarValue::DWord(0x0001_0203));
    }

    #[test]
    fn parse_dint_negative() {
        // 0xFFFF_FF00 = -256 as i32
        let bytes = vec![0xFF, 0xFF, 0xFF, 0x00];
        let result = parse_var(&bytes, &def(VarType::DInt, 0, 0));
        assert_eq!(result, VarValue::DInt(-256));
    }

    #[test]
    fn parse_real() {
        // IEEE 754 big-endian for 1.0f32 = 0x3F80_0000
        let bytes = vec![0x3F, 0x80, 0x00, 0x00];
        let result = parse_var(&bytes, &def(VarType::Real, 0, 0));
        assert_eq!(result, VarValue::Real(1.0f32));
    }

    #[test]
    fn parse_string_null_terminated() {
        let mut bytes = b"hello".to_vec();
        bytes.extend_from_slice(&[0x00, 0x00, 0x00]); // padding
        let result = parse_var(&bytes, &def(VarType::String { length: 8 }, 0, 0));
        assert_eq!(result, VarValue::StringVal("hello".into()));
    }

    #[test]
    fn parse_out_of_bounds_returns_unknown() {
        let bytes = vec![0x01]; // only 1 byte
        // Word needs 2 bytes — must return Unknown
        let result = parse_var(&bytes, &def(VarType::Word, 0, 0));
        assert_eq!(result, VarValue::Unknown);
    }

    #[test]
    fn parse_empty_bytes_returns_unknown() {
        let result = parse_var(&[], &def(VarType::Bool, 0, 0));
        assert_eq!(result, VarValue::Unknown);
    }
}
