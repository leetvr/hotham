#![allow(clippy::missing_safety_doc)]

mod action_state;
mod client;
mod space_state;

use crate::client::*;
use openxr_sys::{loader, pfn, Instance, Result};
use std::ffi::c_char;

type DummyFn = unsafe extern "system" fn() -> Result;

pub unsafe extern "system" fn get_instance_proc_addr(
    _instance: Instance,
    name: *const c_char,
    function: *mut Option<pfn::VoidFunction>,
) -> Result {
    use std::{ffi::CStr, intrinsics::transmute};

    let name = CStr::from_ptr(name);
    let name = name.to_bytes();
    if name == b"xrGetInstanceProcAddr" {
        *function = transmute::<pfn::GetInstanceProcAddr, _>(get_instance_proc_addr);
    } else if name == b"xrEnumerateInstanceExtensionProperties" {
        *function = transmute::<pfn::EnumerateInstanceExtensionProperties, _>(
            enumerate_instance_extension_properties,
        );
    } else if name == b"xrCreateInstance" {
        *function = transmute::<pfn::CreateInstance, _>(create_instance);
    } else if name == b"xrCreateVulkanInstanceKHR" {
        *function = transmute::<pfn::CreateVulkanInstanceKHR, _>(create_vulkan_instance);
    } else if name == b"xrCreateVulkanDeviceKHR" {
        *function = transmute::<pfn::CreateVulkanDeviceKHR, _>(create_vulkan_device);
    } else if name == b"xrGetVulkanGraphicsDevice2KHR" {
        *function = transmute::<pfn::GetVulkanGraphicsDevice2KHR, _>(get_vulkan_graphics_device_2);
    } else if name == b"xrGetInstanceProperties" {
        *function = transmute::<pfn::GetInstanceProperties, _>(get_instance_properties);
    } else if name == b"xrGetVulkanGraphicsRequirements2KHR" {
        *function = transmute::<pfn::GetVulkanGraphicsRequirements2KHR, _>(
            get_vulkan_graphics_requirements,
        );
    } else if name == b"xrGetVulkanGraphicsDeviceKHR" {
        *function = transmute::<pfn::GetVulkanGraphicsDeviceKHR, _>(get_vulkan_physical_device);
    } else if name == b"xrGetVulkanGraphicsRequirementsKHR" {
        *function =
            transmute::<pfn::GetVulkanGraphicsRequirementsKHR, _>(get_vulkan_graphics_requirements);
    } else if name == b"xrGetVulkanInstanceExtensionsKHR" {
        *function =
            transmute::<pfn::GetVulkanInstanceExtensionsKHR, _>(get_vulkan_instance_extensions);
    } else if name == b"xrGetVulkanDeviceExtensionsKHR" {
        *function = transmute::<pfn::GetVulkanDeviceExtensionsKHR, _>(get_vulkan_device_extensions);
    } else if name == b"xrEnumerateEnvironmentBlendModes" {
        *function =
            transmute::<pfn::EnumerateEnvironmentBlendModes, _>(enumerate_environment_blend_modes);
    } else if name == b"xrGetSystem" {
        *function = transmute::<pfn::GetSystem, _>(get_system);
    } else if name == b"xrCreateSession" {
        *function = transmute::<pfn::CreateSession, _>(create_session);
    } else if name == b"xrCreateActionSet" {
        *function = transmute::<pfn::CreateActionSet, _>(create_action_set);
    } else if name == b"xrCreateAction" {
        *function = transmute::<pfn::CreateAction, _>(create_action);
    } else if name == b"xrSuggestInteractionProfileBindings" {
        *function = transmute::<pfn::SuggestInteractionProfileBindings, _>(
            suggest_interaction_profile_bindings,
        );
    } else if name == b"xrStringToPath" {
        *function = transmute::<pfn::StringToPath, _>(string_to_path);
    } else if name == b"xrAttachSessionActionSets" {
        *function = transmute::<pfn::AttachSessionActionSets, _>(attach_action_sets);
    } else if name == b"xrCreateActionSpace" {
        *function = transmute::<pfn::CreateActionSpace, _>(create_action_space);
    } else if name == b"xrCreateReferenceSpace" {
        *function = transmute::<pfn::CreateReferenceSpace, _>(create_reference_space);
    } else if name == b"xrPollEvent" {
        *function = transmute::<pfn::PollEvent, _>(poll_event);
    } else if name == b"xrBeginSession" {
        *function = transmute::<pfn::BeginSession, _>(begin_session);
    } else if name == b"xrWaitFrame" {
        *function = transmute::<pfn::WaitFrame, _>(wait_frame);
    } else if name == b"xrBeginFrame" {
        *function = transmute::<pfn::BeginFrame, _>(begin_frame);
    } else if name == b"xrEnumerateViewConfigurationViews" {
        *function = transmute::<pfn::EnumerateViewConfigurationViews, _>(
            enumerate_view_configuration_views,
        );
    } else if name == b"xrCreateSwapchain" {
        *function = transmute::<pfn::CreateSwapchain, _>(create_xr_swapchain);
    } else if name == b"xrEnumerateSwapchainImages" {
        *function = transmute::<pfn::EnumerateSwapchainImages, _>(enumerate_swapchain_images);
    } else if name == b"xrAcquireSwapchainImage" {
        *function = transmute::<pfn::AcquireSwapchainImage, _>(acquire_swapchain_image);
    } else if name == b"xrWaitSwapchainImage" {
        *function = transmute::<pfn::WaitSwapchainImage, _>(wait_swapchain_image);
    } else if name == b"xrSyncActions" {
        *function = transmute::<pfn::SyncActions, _>(sync_actions);
    } else if name == b"xrLocateSpace" {
        *function = transmute::<pfn::LocateSpace, _>(locate_space);
    } else if name == b"xrGetActionStatePose" {
        *function = transmute::<pfn::GetActionStatePose, _>(get_action_state_pose);
    } else if name == b"xrLocateViews" {
        *function = transmute::<pfn::LocateViews, _>(locate_views);
    } else if name == b"xrReleaseSwapchainImage" {
        *function = transmute::<pfn::ReleaseSwapchainImage, _>(release_swapchain_image);
    } else if name == b"xrEndFrame" {
        *function = transmute::<pfn::EndFrame, _>(end_frame);
    } else if name == b"xrRequestExitSession" {
        *function = transmute::<pfn::RequestExitSession, _>(request_exit_session);
    } else if name == b"xrDestroySpace" {
        *function = transmute::<pfn::DestroySpace, _>(destroy_space);
    } else if name == b"xrDestroyAction" {
        *function = transmute::<pfn::DestroyAction, _>(destroy_action);
    } else if name == b"xrDestroyActionSet" {
        *function = transmute::<pfn::DestroyActionSet, _>(destroy_action_set);
    } else if name == b"xrDestroySwapchain" {
        *function = transmute::<pfn::DestroySwapchain, _>(destroy_swapchain);
    } else if name == b"xrDestroySession" {
        *function = transmute::<pfn::DestroySession, _>(destroy_session);
    } else if name == b"xrDestroyInstance" {
        *function = transmute::<pfn::DestroyInstance, _>(destroy_instance);
    } else if name == b"xrEnumerateViewConfigurations" {
        *function = transmute::<pfn::EnumerateViewConfigurations, _>(enumerate_view_configurations);
    } else if name == b"xrEnumerateReferenceSpaces" {
        *function = transmute::<pfn::EnumerateReferenceSpaces, _>(enumerate_reference_spaces);
    } else if name == b"xrGetSystemProperties" {
        *function = transmute::<pfn::GetSystemProperties, _>(get_system_properties);
    } else if name == b"xrEnumerateSwapchainFormats" {
        *function = transmute::<pfn::EnumerateSwapchainFormats, _>(enumerate_swapchain_formats);
    } else if name == b"xrGetActionStateFloat" {
        *function = transmute::<pfn::GetActionStateFloat, _>(get_action_state_float);
    } else if name == b"xrGetActionStateBoolean" {
        *function = transmute::<pfn::GetActionStateBoolean, _>(get_action_state_boolean);
    } else if name == b"xrApplyHapticFeedback" {
        *function = transmute::<pfn::ApplyHapticFeedback, _>(apply_haptic_feedback);
    } else if name == b"xrEndSession" {
        *function = transmute::<pfn::EndSession, _>(end_session);
    } else {
        let _name = String::from_utf8_unchecked(name.to_vec());
        unsafe extern "system" fn bang() -> Result {
            panic!("UNIMPLEMENTED FUNCTION!");
        }
        *function = transmute::<DummyFn, _>(bang);
    }
    Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "system" fn xrNegotiateLoaderRuntimeInterface(
    _loader_info: *const loader::XrNegotiateLoaderInfo,
    runtime_request: *mut loader::XrNegotiateRuntimeRequest,
) -> Result {
    let runtime_request = &mut *runtime_request;
    runtime_request.runtime_interface_version = 1;
    runtime_request.runtime_api_version = openxr_sys::CURRENT_API_VERSION;
    runtime_request.get_instance_proc_addr = Some(get_instance_proc_addr);

    Result::SUCCESS
}
