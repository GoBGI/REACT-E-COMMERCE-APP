use std::error::Error as StdError;
use std::ffi::CString;
use std::os::raw::{c_int, c_void};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use bytes::{BytesMut, buf::ext::BufExt};
use tokio::sync::mpsc::Sender;

use crate::musicd_c;

extern "C" fn stream_c_callback(opaque: *const c_void, data: *const u8, len: c_int) -> c_int {
    let closure: &mut &mut dyn FnMut(&[u8]) -> usize =
        unsafe { &mut *(opaque as *mut &mut dyn for<'r> std::ops::FnMut(&'r [u8]) -> usize) };

    let slice = unsafe { std::slice::from_raw_parts(data, len as usize) };

    closure(slice) as i32
}

pub struct AudioStream {
    stream: *const c_void,
}

unsafe impl Send for AudioStream {}

impl Drop for AudioStream {
    fn drop(&mut self) {
        un