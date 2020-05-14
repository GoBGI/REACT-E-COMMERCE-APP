use std::error::Error as StdError;
use std::ffi::CString;
use std::os::raw::{c_int, c_void};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use bytes::{BytesMut, buf