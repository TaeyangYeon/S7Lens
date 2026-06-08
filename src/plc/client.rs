use std::ffi::c_void;
use std::os::raw::c_int;

use crate::model::session::ConnectionConfig;

pub const S7_AREA_DB: i32 = 0x84;
pub const S7_WL_BYTE: i32 = 0x02;

// FFI declarations are excluded from test builds; all test paths go through mock_mode.
#[cfg(not(test))]
extern "C" {
    fn Cli_Create() -> *mut c_void;
    fn Cli_Destroy(client: *mut *mut c_void) -> c_int;
    fn Cli_ConnectTo(
        client: *mut c_void,
        address: *const i8,
        rack: c_int,
        slot: c_int,
    ) -> c_int;
    fn Cli_Disconnect(client: *mut c_void) -> c_int;
    fn Cli_ReadArea(
        client: *mut c_void,
        area: c_int,
        db_number: c_int,
        start: c_int,
        amount: c_int,
        word_len: c_int,
        p_data: *mut c_void,
    ) -> c_int;
    fn Cli_GetConnected(client: *mut c_void, connected: *mut c_int) -> c_int;
}

/// Safe wrapper around a snap7 client handle.
///
/// All public methods are safe; `unsafe` is isolated to the FFI call sites inside
/// private helpers.
pub struct PlcClient {
    handle: *mut c_void,
    mock_mode: bool,
}

// SAFETY: PlcClient owns its handle exclusively. Callers sharing across threads
// must wrap in Arc<Mutex<PlcClient>>.
unsafe impl Send for PlcClient {}

impl PlcClient {
    /// Create a real snap7 client backed by the native library.
    pub fn new() -> Self {
        #[cfg(not(test))]
        let handle = unsafe { Cli_Create() };
        // In test builds the extern block is excluded; produce a null handle so
        // the struct is valid — tests must use new_mock() anyway.
        #[cfg(test)]
        let handle = std::ptr::null_mut();

        Self { handle, mock_mode: false }
    }

    /// Create a mock client that never calls into the native library.
    pub fn new_mock() -> Self {
        Self { handle: std::ptr::null_mut(), mock_mode: true }
    }

    /// Connect to the PLC described by `config`.
    pub fn connect(&mut self, config: &ConnectionConfig) -> Result<(), String> {
        if self.mock_mode {
            return Ok(());
        }
        self.connect_native(config)
    }

    /// Disconnect from the PLC (best-effort; errors are ignored).
    pub fn disconnect(&mut self) {
        if self.mock_mode || self.handle.is_null() {
            return;
        }
        self.disconnect_native();
    }

    /// Read `size` raw bytes from `db_number` starting at byte 0.
    ///
    /// Returns big-endian bytes exactly as stored in the PLC. In mock mode returns
    /// `size` zero bytes.
    pub fn read_db(&self, db_number: i32, size: usize) -> Result<Vec<u8>, String> {
        if self.mock_mode {
            return Ok(vec![0u8; size]);
        }
        self.read_db_native(db_number, size)
    }

    /// Returns true when snap7 reports an active connection.
    pub fn is_connected(&self) -> bool {
        if self.mock_mode || self.handle.is_null() {
            return false;
        }
        self.is_connected_native()
    }

    // ── native helpers (excluded from test builds) ──────────────────────────

    #[cfg(not(test))]
    fn connect_native(&mut self, config: &ConnectionConfig) -> Result<(), String> {
        let ip = std::ffi::CString::new(config.ip.as_str())
            .map_err(|e| format!("invalid IP address string: {}", e))?;

        // SAFETY: handle is non-null (created by Cli_Create); ip lives for the call.
        let rc = unsafe {
            Cli_ConnectTo(
                self.handle,
                ip.as_ptr() as *const i8,
                config.rack as c_int,
                config.slot as c_int,
            )
        };

        if rc == 0 {
            Ok(())
        } else {
            Err(format!("Cli_ConnectTo returned {:#010x}", rc))
        }
    }

    #[cfg(test)]
    fn connect_native(&mut self, _config: &ConnectionConfig) -> Result<(), String> {
        // Unreachable in tests because connect() short-circuits on mock_mode.
        Err("connect_native called in test build without mock_mode".into())
    }

    #[cfg(not(test))]
    fn disconnect_native(&mut self) {
        // SAFETY: handle is non-null and owned by self.
        unsafe { Cli_Disconnect(self.handle) };
    }

    #[cfg(test)]
    fn disconnect_native(&mut self) {}

    #[cfg(not(test))]
    fn read_db_native(&self, db_number: i32, size: usize) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; size];
        // SAFETY: handle is non-null; buf is exclusively owned for the duration.
        let rc = unsafe {
            Cli_ReadArea(
                self.handle,
                S7_AREA_DB,
                db_number,
                0,
                size as c_int,
                S7_WL_BYTE,
                buf.as_mut_ptr() as *mut c_void,
            )
        };
        if rc == 0 {
            Ok(buf)
        } else {
            Err(format!("Cli_ReadArea returned {:#010x}", rc))
        }
    }

    #[cfg(test)]
    fn read_db_native(&self, _db_number: i32, _size: usize) -> Result<Vec<u8>, String> {
        Err("read_db_native called in test build without mock_mode".into())
    }

    #[cfg(not(test))]
    fn is_connected_native(&self) -> bool {
        let mut connected: c_int = 0;
        // SAFETY: handle is non-null; connected is a stack variable.
        let rc = unsafe { Cli_GetConnected(self.handle, &mut connected) };
        rc == 0 && connected != 0
    }

    #[cfg(test)]
    fn is_connected_native(&self) -> bool {
        false
    }
}

impl Drop for PlcClient {
    fn drop(&mut self) {
        if self.mock_mode || self.handle.is_null() {
            return;
        }
        #[cfg(not(test))]
        // SAFETY: handle was created by Cli_Create; passing &mut handle lets snap7 zero it.
        unsafe { Cli_Destroy(&mut self.handle) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_read_db_returns_zeroed_bytes() {
        let client = PlcClient::new_mock();
        let data = client.read_db(100, 32).unwrap();
        assert_eq!(data.len(), 32);
        assert!(data.iter().all(|&b| b == 0));
    }

    #[test]
    fn mock_read_db_size_zero() {
        let client = PlcClient::new_mock();
        let data = client.read_db(1, 0).unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn mock_is_not_connected() {
        let client = PlcClient::new_mock();
        assert!(!client.is_connected());
    }

    #[test]
    fn mock_connect_succeeds() {
        let mut client = PlcClient::new_mock();
        let config = ConnectionConfig::default();
        assert!(client.connect(&config).is_ok());
    }

    #[test]
    fn mock_disconnect_does_not_panic() {
        let mut client = PlcClient::new_mock();
        client.disconnect(); // must not panic
    }
}
