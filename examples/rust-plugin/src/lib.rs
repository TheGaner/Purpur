// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{ffi::CStr,os::raw::c_char};
#[no_mangle]pub extern "C" fn horizon_plugin_abi_version()->u32{1}
#[no_mangle]pub unsafe extern "C" fn horizon_plugin_event(kind:u32,tick:u64,text:*const c_char)->i32{
 match kind{
  1=>println!("[hello-horizon] enabled"),
  2 if tick%200==0=>println!("[hello-horizon] tick {tick}"),
  3 if !text.is_null()=>println!("[hello-horizon] command: {}",CStr::from_ptr(text).to_string_lossy()),
  4=>println!("[hello-horizon] disabled"),
  _=>()
 }0
}

/// Optional host-owned-buffer command manifest ABI v2. The event ABI stays v1.
/// Return the byte count, never a pointer; do not write beyond capacity.
#[no_mangle]
pub unsafe extern "C" fn horizon_plugin_commands_v2(out:*mut u8,capacity:usize)->usize{
    const DATA:&[u8]=b"hello|example.hello|hi";
    if out.is_null() || capacity < DATA.len(){return DATA.len();}
    std::ptr::copy_nonoverlapping(DATA.as_ptr(),out,DATA.len());
    DATA.len()
}
