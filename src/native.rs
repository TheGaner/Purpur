// SPDX-License-Identifier: AGPL-3.0-or-later
// Trust boundary: Rust native plugins have complete process privileges.
// Linux proof-of-concept C ABI. Versioning prevents arbitrary entrypoint ABI drift.
#[cfg(unix)]
use std::{
    ffi::{CStr, CString},
    os::{
        raw::{c_char, c_int, c_void},
        unix::ffi::OsStrExt,
    },
    path::Path,
};
#[cfg(unix)]
#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlerror() -> *const c_char;
}
#[cfg(unix)]
type OnEvent = unsafe extern "C" fn(u32, u64, *const c_char) -> i32;
#[cfg(unix)]
const MAX_COMMAND_MANIFEST_BYTES: usize = 8192;

/// Decode a versioned, length-delimited, caller-owned command manifest.
/// A trusted .so can still corrupt memory; no native-library sandbox implied.
#[cfg(unix)]
fn decode_manifest(bytes: &[u8], used: usize) -> Option<String> {
    if used == 0 || used > bytes.len() || bytes[..used].contains(&0) {
        return None;
    }
    std::str::from_utf8(&bytes[..used]).ok().map(str::to_owned)
}

#[cfg(unix)]
pub struct Plugin {
    handle: *mut c_void,
    callback: OnEvent,
    pub name: String,
}
#[cfg(unix)]
impl Plugin {
    pub fn load(path: &Path) -> Result<Self, String> {
        let file = CString::new(path.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
        unsafe {
            let handle = dlopen(file.as_ptr(), 2);
            if handle.is_null() {
                let e = dlerror();
                return Err(if e.is_null() {
                    "dlopen failed".into()
                } else {
                    CStr::from_ptr(e).to_string_lossy().into_owned()
                });
            }
            let load = || -> Result<OnEvent, String> {
                let abi = dlsym(handle, c"horizon_plugin_abi_version".as_ptr());
                let call = dlsym(handle, c"horizon_plugin_event".as_ptr());
                if abi.is_null() || call.is_null() {
                    return Err("plugin must export ABI version and event callback".into());
                }
                let v: unsafe extern "C" fn() -> u32 = std::mem::transmute(abi);
                if v() != 1 {
                    return Err("wrong native plugin ABI version".into());
                }
                Ok(std::mem::transmute::<*mut c_void, OnEvent>(call))
            };
            match load() {
                Ok(callback) => Ok(Self {
                    handle,
                    callback,
                    name: path.file_name().unwrap().to_string_lossy().into_owned(),
                }),
                Err(e) => {
                    dlclose(handle);
                    Err(e)
                }
            }
        }
    }
    pub fn declared_commands(&self) -> Option<String> {
        // Manifest ABI v2: host owns a fixed-sized writable byte buffer.
        // fn horizon_plugin_commands_v2(out: *mut u8, capacity: usize) -> usize
        // v1 pointer-returning export is deliberately unsupported: it could
        // perform an unbounded NUL scan before validating the buffer length.
        // Event/lifecycle ABI v1 remains unchanged.
        unsafe {
            let symbol = dlsym(self.handle, c"horizon_plugin_commands_v2".as_ptr());
            if symbol.is_null() {
                return None;
            }
            let function: unsafe extern "C" fn(*mut u8, usize) -> usize =
                std::mem::transmute(symbol);
            let mut output = [0u8; MAX_COMMAND_MANIFEST_BYTES];
            let used = function(output.as_mut_ptr(), output.len());
            decode_manifest(&output, used)
        }
    }
    pub fn event(&self, code: u32, tick: u64, payload: &str) {
        if let Ok(t) = CString::new(payload) {
            unsafe {
                (self.callback)(code, tick, t.as_ptr());
            }
        }
    }
}
#[cfg(unix)]
impl Drop for Plugin {
    fn drop(&mut self) {
        unsafe {
            dlclose(self.handle);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn manifest_v2_bounds_utf8_and_binary_validation() {
        let data = b"hello|example.hello|hi";
        assert_eq!(
            decode_manifest(data, data.len()).as_deref(),
            Some("hello|example.hello|hi")
        );
        assert_eq!(decode_manifest(data, 0), None);
        assert_eq!(decode_manifest(data, data.len() + 1), None);
        assert_eq!(decode_manifest(b"hello\0invalid", 13), None);
        assert_eq!(decode_manifest(&[255], 1), None);
        assert_eq!(
            decode_manifest(
                &vec![b'x'; MAX_COMMAND_MANIFEST_BYTES],
                MAX_COMMAND_MANIFEST_BYTES
            )
            .unwrap()
            .len(),
            MAX_COMMAND_MANIFEST_BYTES
        );
        assert!(decode_manifest(
            &vec![b'x'; MAX_COMMAND_MANIFEST_BYTES],
            MAX_COMMAND_MANIFEST_BYTES + 1
        )
        .is_none());
    }
}
