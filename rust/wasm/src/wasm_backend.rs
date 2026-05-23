use crate::host;

/// WASM ストレージバックエンド
/// ホスト関数経由で I/O を実行
pub struct WasmStorageBackend;

impl WasmStorageBackend {
    pub fn new() -> Self {
        Self
    }

    pub fn put(&self, key: &str, data: &str) -> Result<(), String> {
        let ret = unsafe {
            host::storage_put(
                key.as_ptr(),
                key.len(),
                data.as_ptr(),
                data.len(),
            )
        };
        if ret == 0 {
            Ok(())
        } else {
            Err("storage_put failed".into())
        }
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        let mut ptr: *const u8 = std::ptr::null();
        let mut len: usize = 0;
        let ret = unsafe {
            host::storage_get(key.as_ptr(), key.len(), &mut ptr, &mut len)
        };
        if ret == -1 {
            return Ok(None); // not found
        }
        if ret != 0 {
            return Err("storage_get failed".into());
        }
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        Ok(Some(String::from_utf8_lossy(bytes).to_string()))
    }

    pub fn list(&self, prefix: &str) -> Result<Vec<String>, String> {
        let mut ptr: *const u8 = std::ptr::null();
        let mut len: usize = 0;
        let ret = unsafe {
            host::storage_list(prefix.as_ptr(), prefix.len(), &mut ptr, &mut len)
        };
        if ret != 0 {
            return Err("storage_list failed".into());
        }
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        let json_str = String::from_utf8_lossy(bytes);
        serde_json::from_str(&json_str).map_err(|e| format!("JSON parse error: {}", e))
    }

    pub fn delete(&self, key: &str) -> Result<(), String> {
        let ret = unsafe { host::storage_delete(key.as_ptr(), key.len()) };
        if ret == 0 {
            Ok(())
        } else {
            Err("storage_delete failed".into())
        }
    }
}
