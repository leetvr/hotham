use super::XrContext;
use crate::HothamResult;

#[cfg(target_os = "windows")]
impl XrContext {
    pub fn now(self: &Self) -> HothamResult<openxr::Time> {
        if let Some(ext) = &self
            .instance
            .exts()
            .khr_win32_convert_performance_counter_time
        {
            let mut xr_time = openxr::Time::from_nanos(0);
            let performance_counter = get_performance_counter().unwrap();
            match unsafe {
                (ext.convert_win32_performance_counter_to_time)(
                    self.instance.as_raw(),
                    &performance_counter,
                    &mut xr_time,
                )
            } {
                openxr::sys::Result::SUCCESS => Ok(xr_time),
                _ => Err(anyhow::anyhow!(
                    "OpenXR convert_win32_performance_counter_to_time failed."
                )
                .into()),
            }
        } else {
            Err(anyhow::anyhow!(
                "OpenXR extension khr_win32_convert_performance_counter_time needs to be enabled. \
                Enable it via XrContextBuilder::required_extensions()."
            )
            .into())
        }
    }
}

#[cfg(target_os = "windows")]
fn get_performance_counter() -> Result<i64, windows::core::Error> {
    unsafe {
        let mut time = 0;
        windows::Win32::System::Performance::QueryPerformanceCounter(&mut time).ok()?;
        Ok(time)
    }
}

#[cfg(not(target_os = "windows"))]
impl XrContext {
    pub fn now(self: &Self) -> HothamResult<openxr::Time> {
        todo!("XrContext::now() is not yet implemented for non-windows platforms.")
    }
}
