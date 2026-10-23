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

#[no_mangle]
pub extern "C" fn horizon_plugin_commands() -> *const c_char {
    c"hello|example.hello|hi".as_ptr()
}
