use std::{ffi::CStr, os::raw::c_char};

pub(crate) unsafe fn _get_raw_strings(strings: Vec<&str>) -> Vec<*const c_char> {
    strings
        .iter()
        .map(|s| CStr::from_bytes_with_nul_unchecked(s.as_bytes()).as_ptr())
        .collect::<Vec<_>>()
}

pub(crate) unsafe fn _parse_raw_strings(raw_strings: &[*const c_char]) -> Vec<&str> {
    raw_strings
        .iter()
        .map(|s| _parse_raw_string(*s))
        .collect::<Vec<_>>()
}

pub(crate) unsafe fn _parse_raw_string(raw_string: *const c_char) -> &'static str {
    CStr::from_ptr(raw_string).to_str().unwrap()
}
