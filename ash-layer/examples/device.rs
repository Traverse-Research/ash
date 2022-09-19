//! Demonstrates how to wrap device functions
//!
//! ```sh
//! cargo b -p ash-layer --example ash-device-example
//! VK_LAYER_PATH=/usr/share/vulkan/explicit_layer.d/:$(realpath ../ash/ash-layer/examples/) VK_INSTANCE_LAYERS=VK_LAYER_ASH_device_example your-application
//! ```
#![allow(non_camel_case_types, non_snake_case)]

use std::{collections::HashMap, ffi::CStr, os::raw::c_char, sync::Mutex};

use ash::vk;
use ash_layer::*;
use once_cell::sync::Lazy;

struct InstanceDispatch {
    get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
}

// TODO: RWLock for concurrent reading
/// Maps Vulkan dispatch-table pointers (first value in a handle) to stored instance table.
/// See `Object Wrapping` under <https://renderdoc.org/vulkan-layer-guide.html>.
static INSTANCE_DISPATCH: Lazy<Mutex<HashMap<usize, InstanceDispatch>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

struct DeviceDispatch {
    get_device_proc_addr: vk::PFN_vkGetDeviceProcAddr,
    allocate_memory: vk::PFN_vkAllocateMemory,
    free_memory: vk::PFN_vkFreeMemory,
}

static DEVICE_DISPATCH: Lazy<Mutex<HashMap<usize, DeviceDispatch>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Entrypoint for the ICD
#[no_mangle]
unsafe extern "system" fn vkGetInstanceProcAddr(
    instance: vk::Instance,
    p_name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    let name = CStr::from_ptr(p_name);
    let f = match name.to_str().unwrap() {
        "vkCreateInstance" => std::mem::transmute(vkCreateInstance as vk::PFN_vkCreateInstance),
        "vkGetDeviceProcAddr" => {
            std::mem::transmute(vkGetDeviceProcAddr as vk::PFN_vkGetDeviceProcAddr)
        }
        "vkCreateDevice" => std::mem::transmute(vkCreateDevice as vk::PFN_vkCreateDevice),
        "vkAllocateMemory" | "vkFreeMemory" => panic!("Use device-optimized loader for this"),
        _ => {
            let instance_dispatch = INSTANCE_DISPATCH.lock().unwrap();
            let loader_dispatch_table = *std::mem::transmute::<_, *const usize>(instance);
            let instance_dispatch = instance_dispatch
                .get(&loader_dispatch_table)
                .expect("vkCreateInstance was not yet called");
            return (instance_dispatch.get_instance_proc_addr)(instance, p_name);
        }
    };
    eprintln!("Returning custom function for {:?}", name);
    Some(f)
}

unsafe extern "system" fn vkCreateInstance(
    p_create_info: *const vk::InstanceCreateInfo,
    p_allocator: *const vk::AllocationCallbacks,
    p_instance: *mut vk::Instance,
) -> vk::Result {
    let create_info = &mut *(p_create_info as *mut vk::InstanceCreateInfo);
    let la = &*create_info.p_application_info;
    dbg!(&*create_info.p_application_info);
    if !la.p_application_name.is_null() {
        dbg!(CStr::from_ptr(la.p_application_name));
    }
    if !la.p_engine_name.is_null() {
        dbg!(CStr::from_ptr(la.p_engine_name));
    }
    let layer_create_info = vk::ptr_chain_iter(create_info)
        .map(|s| s.cast::<LayerInstanceCreateInfo>().as_mut().unwrap())
        .filter(|s| s.s_type == vk::StructureType::LOADER_INSTANCE_CREATE_INFO)
        .find(|s| s.function == LayerFunction::LayerLinkInfo)
        .unwrap();

    dbg!(&layer_create_info);

    let layer_info = &mut *layer_create_info.u.layer_info;

    // Advance the pointer
    layer_create_info.u.layer_info = layer_info.p_next;

    unsafe fn get_function(
        layer_info: &LayerInstanceLink,
        instance: vk::Instance,
        name: &[u8],
    ) -> unsafe extern "system" fn() {
        let cname = ::std::ffi::CStr::from_bytes_with_nul_unchecked(name);
        (layer_info.pfnNextGetInstanceProcAddr)(instance, cname.as_ptr())
            .unwrap_or_else(|| panic!("Failed to load {:?}", cname))
    }

    // Retrieve the actual vkCreateInstance from the next GetInstanceProcAddr in the chain
    let create_instance: vk::PFN_vkCreateInstance = std::mem::transmute(get_function(
        layer_info,
        vk::Instance::null(),
        b"vkCreateInstance\0",
    ));

    let ret = (create_instance)(p_create_info, p_allocator, p_instance);

    let loader_dispatch_table = *std::mem::transmute::<_, *const usize>(*p_instance);

    let mut instance_dispatch = INSTANCE_DISPATCH.lock().unwrap();
    instance_dispatch.insert(
        loader_dispatch_table,
        InstanceDispatch {
            get_instance_proc_addr: layer_info.pfnNextGetInstanceProcAddr,
        },
    );

    ret
}

unsafe extern "system" fn vkCreateDevice(
    physical_device: vk::PhysicalDevice,
    p_create_info: *const vk::DeviceCreateInfo,
    p_allocator: *const vk::AllocationCallbacks,
    p_device: *mut vk::Device,
) -> vk::Result {
    let create_info = &mut *(p_create_info as *mut vk::DeviceCreateInfo);

    let layer_create_info = vk::ptr_chain_iter(create_info)
        .map(|s| s.cast::<LayerDeviceCreateInfo>().as_mut().unwrap())
        .filter(|s| s.s_type == vk::StructureType::LOADER_DEVICE_CREATE_INFO)
        .find(|s| s.function == LayerFunction::LayerLinkInfo)
        .unwrap();

    dbg!(&layer_create_info);

    let layer_info = &mut *layer_create_info.u.layer_info;

    // Advance the pointer
    layer_create_info.u.layer_info = layer_info.p_next;

    unsafe fn get_instance_function(
        layer_info: &LayerDeviceLink,
        instance: vk::Instance,
        name: &[u8],
    ) -> unsafe extern "system" fn() {
        let cname = ::std::ffi::CStr::from_bytes_with_nul_unchecked(name);
        (layer_info.pfnNextGetInstanceProcAddr)(instance, cname.as_ptr())
            .unwrap_or_else(|| panic!("Failed to load {:?}", cname))
    }

    unsafe fn get_device_function(
        layer_info: &LayerDeviceLink,
        device: vk::Device,
        name: &[u8],
    ) -> unsafe extern "system" fn() {
        let cname = ::std::ffi::CStr::from_bytes_with_nul_unchecked(name);
        (layer_info.pfnNextGetDeviceProcAddr)(device, cname.as_ptr())
            .unwrap_or_else(|| panic!("Failed to load {:?}", cname))
    }

    // Retrieve the actual vkCreateDevice from the next GetInstanceProcAddr in the chain
    let create_device: vk::PFN_vkCreateDevice = std::mem::transmute(get_instance_function(
        layer_info,
        vk::Instance::null(),
        b"vkCreateDevice\0",
    ));

    let ret = (create_device)(physical_device, p_create_info, p_allocator, p_device);

    println!("Created device {:?}", *p_device);

    let loader_dispatch_table = *std::mem::transmute::<_, *const usize>(*p_device);

    let mut device_dispatch = DEVICE_DISPATCH.lock().unwrap();
    device_dispatch.insert(
        loader_dispatch_table,
        DeviceDispatch {
            get_device_proc_addr: layer_info.pfnNextGetDeviceProcAddr,
            allocate_memory: std::mem::transmute(get_device_function(
                layer_info,
                *p_device,
                b"vkAllocateMemory\0",
            )),
            free_memory: std::mem::transmute(get_device_function(
                layer_info,
                *p_device,
                b"vkFreeMemory\0",
            )),
        },
    );

    ret
}

/// Entrypoint for the ICD
#[no_mangle]
unsafe extern "system" fn vkGetDeviceProcAddr(
    device: vk::Device,
    p_name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    let name = CStr::from_ptr(p_name);
    let f = match name.to_str().unwrap() {
        "vkAllocateMemory" => std::mem::transmute(vkAllocateMemory as vk::PFN_vkAllocateMemory),
        "vkFreeMemory" => std::mem::transmute(vkFreeMemory as vk::PFN_vkFreeMemory),
        _ => {
            let device_dispatch = DEVICE_DISPATCH.lock().unwrap();
            let loader_dispatch_table = *std::mem::transmute::<_, *const usize>(device);
            let device_dispatch = device_dispatch.get(&loader_dispatch_table).unwrap();
            return (device_dispatch.get_device_proc_addr)(device, p_name);
        }
    };
    eprintln!("Returning custom DEVICE function for {:?}", name);
    Some(f)
}

unsafe extern "system" fn vkAllocateMemory(
    device: vk::Device,
    p_allocate_info: *const vk::MemoryAllocateInfo,
    p_allocator: *const vk::AllocationCallbacks,
    p_memory: *mut vk::DeviceMemory,
) -> vk::Result {
    let allocate_info = &*p_allocate_info;

    let device_dispatch = DEVICE_DISPATCH.lock().unwrap();
    let loader_dispatch_table = *std::mem::transmute::<_, *const usize>(device);
    let device_dispatch = device_dispatch.get(&loader_dispatch_table).unwrap();

    let ret = (device_dispatch.allocate_memory)(device, p_allocate_info, p_allocator, p_memory);

    println!(
        "Allocating {} bytes on memory type #{} = {:?}",
        allocate_info.allocation_size, allocate_info.memory_type_index, *p_memory
    );

    ret
}

unsafe extern "system" fn vkFreeMemory(
    device: vk::Device,
    memory: vk::DeviceMemory,
    p_allocator: *const vk::AllocationCallbacks,
) {
    let device_dispatch = DEVICE_DISPATCH.lock().unwrap();
    let loader_dispatch_table = *std::mem::transmute::<_, *const usize>(device);
    let device_dispatch = device_dispatch.get(&loader_dispatch_table).unwrap();

    println!("Freeing {:?}", memory);

    (device_dispatch.free_memory)(device, memory, p_allocator)
}
