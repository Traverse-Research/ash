#![allow(non_camel_case_types, non_snake_case)]

use std::{ffi::c_void, os::raw::c_char};

/// Reexport a matching version of [`ash`]
pub use ash;
use ash::vk;

// TODO: Move to ash (vk) prelude - or use as replacement type?
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn from_vulkan(driver_version: u32) -> Self {
        Self {
            major: vk::api_version_major(driver_version),
            minor: vk::api_version_minor(driver_version),
            patch: vk::api_version_patch(driver_version),
        }
    }

    pub fn to_vulkan(self) -> u32 {
        vk::make_api_version(0, self.major, self.minor, self.patch)
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
    // TODO: Wrap in safe enum?
    pub function: LayerFunction,
    pub u: LayerInstanceCreateInfo_u,
}

unsafe impl vk::TaggedStructure for LayerInstanceCreateInfo {
    const STRUCTURE_TYPE: vk::StructureType = vk::StructureType::LOADER_INSTANCE_CREATE_INFO;
}

#[repr(C)]
// #[derive(Debug)]
pub struct LayerDeviceLink {
    pub p_next: *mut LayerDeviceLink,
    pub pfnNextGetInstanceProcAddr: vk::PFN_vkGetInstanceProcAddr,
    pub pfnNextGetDeviceProcAddr: vk::PFN_vkGetDeviceProcAddr,
}

#[repr(C)]
pub union LayerDeviceCreateInfo_u {
    pub layer_info: *mut LayerDeviceLink,
    pub pfnSetDeviceLoaderData: PFN_vkSetDeviceLoaderData,
}

impl std::fmt::Debug for LayerDeviceCreateInfo_u {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayerDeviceCreateInfo_u")
            // .field("layer_info", &*self.layer_info)
            .finish()
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct LayerDeviceCreateInfo {
    pub s_type: vk::StructureType,
    pub p_next: *const c_void,
    // TODO: Wrap in safe enum?
    pub function: LayerFunction,
    pub u: LayerDeviceCreateInfo_u,
}

unsafe impl vk::TaggedStructure for LayerDeviceCreateInfo {
    const STRUCTURE_TYPE: vk::StructureType = vk::StructureType::LOADER_DEVICE_CREATE_INFO;
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum NegotiateLayerStructType {
    Uninitialized = 0,
    InterfaceStruct = 1,
}

pub struct NegotiateLayerInterface {
    pub type_: NegotiateLayerStructType,
    pub p_next: *const c_void,
    pub loader_layer_interface_version: u32,
    pub pfnGetInstanceProcAddr: vk::PFN_vkGetInstanceProcAddr,
    pub pfnGetDeviceProcAddr: vk::PFN_vkGetDeviceProcAddr,
    pub pfnGetPhysicalDeviceProcAddr: PFN_GetPhysicalDeviceProcAddr,
}
