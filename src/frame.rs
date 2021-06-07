use ash::vk;

#[derive(Debug, Clone)]
pub(crate) struct Frame {
    pub fence: vk::Fence,
}
