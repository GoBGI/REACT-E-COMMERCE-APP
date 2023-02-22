
use std::os::raw::{c_char, c_int, c_void};

#[repr(C)]
pub struct MediaInfo {
    pub tracks: *const TrackInfo,
    pub images: *const ImageInfo,
}