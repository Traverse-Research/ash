#![allow(non_camel_case_types, non_snake_case)]

use std::{ffi::c_void, os::raw::c_char};

/// Reexport a matching version of [`ash`]
pub use ash;
use ash::vk;

pub struct InstanceDispatch {
    pub get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
    pub create_instance: vk::PFN_vkCreateInstance,
    pub get_physical_device_properties: vk::PFN_vkGetPhysicalDeviceProperties,
    pub get_physical_device_properties2: vk::PFN_vkGetPhysicalDeviceProperties2,
}

// TODO: Move to ash (vk) prelude - or use as replacement type?
#[derive(Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

pub fn version_from_vulkan(driver_version: u32) -> Version {
    Version {
        major: vk::api_version_major(driver_version),
        minor: vk::api_version_minor(driver_version),
        patch: vk::api_version_patch(driver_version),
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum LayerFunction {
    LayerLinkInfo = 0,
    LoaderDataCallback = 1,
}

#[repr(C)]
// #[derive(Debug)]
pub struct LayerInstanceLink {
    pub p_next: *mut LayerInstanceLink,
    pub pfnNextGetInstanceProcAddr: vk::PFN_vkGetInstanceProcAddr,
    pub pfnNextGetPhysicalDeviceProcAddr: PFN_GetPhysicalDeviceProcAddr,
}

pub type PFN_GetPhysicalDeviceProcAddr =
    extern "system" fn(instance: vk::Instance, p_name: *const c_char) -> vk::Result;
pub type PFN_vkSetInstanceLoaderData =
    extern "system" fn(instance: vk::Instance, object: *mut c_void) -> vk::Result;
pub type PFN_vkSetDeviceLoaderData =
    extern "system" fn(instance: vk::Instance, object: *mut c_void) -> vk::Result;

#[repr(C)]
pub union LayerInstanceCreateInfo_u {
    pub layer_info: *mut LayerInstanceLink,
    pub pfnSetInstanceLoaderData: PFN_vkSetInstanceLoaderData,
}

impl std::fmt::Debug for LayerInstanceCreateInfo_u {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayerInstanceCreateInfo_u")
            // .field("layer_info", &*self.layer_info)
            .finish()
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct LayerInstanceCreateInfo {
    pub s_type: vk::StructureType,
    pub p_next: *const c_void,
    pub function: LayerFunction,
    pub u: LayerInstanceCreateInfo_u,
}

unsafe impl vk::TaggedStructure for LayerInstanceCreateInfo {
    const STRUCTURE_TYPE: vk::StructureType = vk::StructureType::LOADER_INSTANCE_CREATE_INFO;
}

// /// Entrypoint for the ICD
// #[no_mangle]
// unsafe extern "system" fn vkGetInstanceProcAddr(
//     instance: vk::Instance,
//     p_name: *const c_char,
// ) -> vk::PFN_vkVoidFunction {
//     let name = CStr::from_ptr(p_name);

//     // TODO: Make this a helper wrapper function?
// }
