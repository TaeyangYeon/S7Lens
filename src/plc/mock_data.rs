/// Generate a deterministic mock DB byte slice of the given `size`.
///
/// Each byte at index `i` equals `i mod 256`, giving predictable, non-zero
/// test data that exercises all variable parser paths without a real PLC.
pub fn make_mock_db(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i & 0xFF) as u8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_mock_db_length() {
        assert_eq!(make_mock_db(64).len(), 64);
    }

    #[test]
    fn make_mock_db_deterministic() {
        let a = make_mock_db(8);
        let b = make_mock_db(8);
        assert_eq!(a, b);
    }

    #[test]
    fn make_mock_db_values() {
        let db = make_mock_db(4);
        assert_eq!(db, vec![0x00, 0x01, 0x02, 0x03]);
    }

    #[test]
    fn make_mock_db_wraps_at_256() {
        let db = make_mock_db(257);
        assert_eq!(db[0], 0x00);
        assert_eq!(db[255], 0xFF);
        assert_eq!(db[256], 0x00);
    }

    #[test]
    fn make_mock_db_empty() {
        assert!(make_mock_db(0).is_empty());
    }
}
