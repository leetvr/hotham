use std::{ffi::CStr, os::raw::c_char, str::Utf8Error};

#[allow(dead_code)]
pub(crate) unsafe fn get_raw_strings(strings: Vec<&str>) -> Vec<*const c_char> {
    strings
        .iter()
        .map(|s| CStr::from_bytes_with_nul_unchecked(s.as_bytes()).as_ptr())
        .collect::<Vec<_>>()
}

#[allow(dead_code)]
pub(crate) unsafe fn parse_raw_strings(raw_strings: &[*const c_char]) -> Vec<&str> {
    raw_strings
        .iter()
        .filter_map(|s| parse_raw_string(*s).ok())
        .collect::<Vec<_>>()
}

#[allow(dead_code)]
pub(crate) unsafe fn parse_raw_string(
    raw_string: *const c_char,
) -> Result<&'static str, Utf8Error> {
    let cstr = CStr::from_ptr(raw_string);
    return cstr.to_str();
}
