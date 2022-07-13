use crate::resources::XrContext;

/// End the current frame
/// Make sure to ONLY call this AFTER `begin_frame` and DO NOT issue any further rendering commands this frame
pub fn end_frame(xr_context: &mut XrContext) {
    xr_context.end_frame().unwrap();
}
