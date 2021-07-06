#[cfg(target_os = "windows")]
pub mod openxr_loader;
#[cfg(target_os = "windows")]
pub mod simulator;
#[cfg(target_os = "windows")]
pub mod space_state;
#[cfg(target_os = "windows")]
pub mod state;

use crate::openxr_loader::{
    PFN_xrEnumerateInstanceExtensionProperties, PFN_xrGetInstanceProcAddr, PFN_xrVoidFunction,
    XrInstance_T, XrResult,
};
#[cfg(target_os = "windows")]
use crate::simulator::*;

#[cfg(target_os = "windows")]
use openxr_sys::{pfn, Result};

type DummyFn = unsafe extern "system" fn() -> Result;

#[no_mangle]
#[cfg(target_os = "windows")]
pub unsafe extern "C" fn get_instance_proc_addr(
    _instance: *mut XrInstance_T,
    name: *const i8,
    function: *mut PFN_xrVoidFunction,
) -> XrResult {
    use std::{ffi::CStr, intrinsics::transmute};

    let name = CStr::from_ptr(name);
    let name = name.to_bytes();
    if name == b"xrGetInstanceProcAddr" {
        *function = transmute::<PFN_xrGetInstanceProcAddr, _>(Some(get_instance_proc_addr));
    } else if name == b"xrEnumerateInstanceExtensionProperties" {
        *function = transmute::<PFN_xrEnumerateInstanceExtensionProperties, _>(Some(
            enumerate_instance_extension_properties,
        ));
    } else if name == b"xrCreateInstance" {
        *function = transmute::<pfn::CreateInstance, _>(create_instance);
    } else if name == b"xrCreateVulkanInstanceKHR" {
        *function = transmute::<pfn::CreateVulkanInstanceKHR, _>(create_vulkan_instance);
    } else if name == b"xrCreateVulkanDeviceKHR" {
        *function = transmute::<pfn::CreateVulkanDeviceKHR, _>(create_vulkan_device);
    } else if name == b"xrGetVulkanGraphicsDevice2KHR" {
        *function = transmute::<pfn::GetVulkanGraphicsDevice2KHR, _>(create_vulkan_physical_device);
    } else if name == b"xrGetInstanceProperties" {
        *function = transmute::<pfn::GetInstanceProperties, _>(get_instance_properties);
    } else if name == b"xrGetVulkanGraphicsRequirements2KHR" {
        *function = transmute::<pfn::GetVulkanGraphicsRequirements2KHR, _>(
            get_vulkan_graphics_requirements,
        );
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
    } else if name == b"xrEndSession" {
        *function = transmute::<pfn::EndSession, _>(end_session);
    } else {
        let _name = String::from_utf8_unchecked(name.to_vec());
        // eprintln!("[HOTHAM_SIMULATOR] {} is unimplemented", name);
        unsafe extern "system" fn bang() -> Result {
            panic!("BAD BADB ADB ADB BAD");
        }
        *function = transmute::<DummyFn, _>(bang);
        // return Result::ERROR_HANDLE_INVALID.into_raw();
    }
    Result::SUCCESS.into_raw()
}

static GET_INSTANCE_PROC_ADDR: PFN_xrGetInstanceProcAddr = Some(get_instance_proc_addr);

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "system" fn xrNegotiateLoaderRuntimeInterface(
    _loader_info: *const openxr_loader::XrNegotiateLoaderInfo,
    runtime_request: *mut openxr_loader::XrNegotiateRuntimeRequest,
) -> i32 {
    let runtime_request = &mut *runtime_request;
    runtime_request.runtimeInterfaceVersion = 1;
    runtime_request.runtimeApiVersion = openxr_sys::CURRENT_API_VERSION.into_raw();
    runtime_request.getInstanceProcAddr = GET_INSTANCE_PROC_ADDR;

    Result::SUCCESS.into_raw()
}
