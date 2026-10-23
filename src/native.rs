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
        // Optional ABI extension: exported static UTF-8 C string, newline-
        // separated name|permission|alias1,alias2 definitions.
        // Native plugins are trusted; they can crash the host regardless.
        unsafe {
            let symbol = dlsym(self.handle, c"horizon_plugin_commands".as_ptr());
            if symbol.is_null() {
                return None;
            }
            let func: unsafe extern "C" fn() -> *const c_char = std::mem::transmute(symbol);
            let ptr = func();
            if ptr.is_null() {
                return None;
            }
            let bytes = CStr::from_ptr(ptr).to_bytes();
            if bytes.len() > 8192 {
                return None;
            }
            String::from_utf8(bytes.to_vec()).ok()
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
