use std::ffi::CStr;

pub(crate) unsafe fn _get_raw_strings(strings: Vec<&str>) -> Vec<*const i8> {
    strings
        .iter()
        .map(|s| CStr::from_bytes_with_nul_unchecked(s.as_bytes()).as_ptr())
        .collect::<Vec<_>>()
}

pub(crate) unsafe fn _parse_raw_strings(raw_strings: &[*const i8]) -> Vec<&str> {
    raw_strings
        .iter()
        .map(|s| _parse_raw_string(*s))
        .collect::<Vec<_>>()
}

pub(crate) unsafe fn _parse_raw_string(raw_string: *const i8) -> &'static str {
    CStr::from_ptr(raw_string).to_str().unwrap()
}
