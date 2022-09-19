//! Demonstrates how to wrap instance functions
//!
//! ```sh
//! cargo b -p ash-layer --example ash-instance-example
//! VK_LAYER_PATH=/usr/share/vulkan/explicit_layer.d/:$(realpath ../ash/ash-layer/examples/) VK_INSTANCE_LAYERS=VK_LAYER_ASH_instance_example your-application
//! ```
#![allow(non_camel_case_types, non_snake_case)]

use std::{collections::HashMap, ffi::CStr, os::raw::c_char, sync::Mutex};

use ash::vk;
use ash_layer::*;
use once_cell::sync::Lazy;

struct InstanceDispatch {
    get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
    get_physical_device_properties: vk::PFN_vkGetPhysicalDeviceProperties,
    get_physical_device_properties2: vk::PFN_vkGetPhysicalDeviceProperties2,
}

// TODO: RWLock for concurrent reading
/// Maps Vulkan dispatch-table pointers (first value in a handle) to stored instance table.
/// See `Object Wrapping` under <https://renderdoc.org/vulkan-layer-guide.html>.
static INSTANCE_DISPATCH: Lazy<Mutex<HashMap<usize, InstanceDispatch>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Entrypoint for the ICD
#[no_mangle]
unsafe extern "system" fn vkGetInstanceProcAddr(
    instance: vk::Instance,
    p_name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    let name = CStr::from_ptr(p_name);
    // Example replacement functions
    let f = match name.to_str().unwrap() {
        // Using `as` casts to aid "type safety" for function signatures
        // TODO: Do this in a macro or something when providing better wrapper-helpers
        "vkCreateInstance" => std::mem::transmute(vkCreateInstance as vk::PFN_vkCreateInstance),
        "vkGetPhysicalDeviceProperties" => std::mem::transmute(
            vkGetPhysicalDeviceProperties as vk::PFN_vkGetPhysicalDeviceProperties,
        ),
        "vkGetPhysicalDeviceProperties2" | "vkGetPhysicalDeviceProperties2KHR" => {
            std::mem::transmute(
                vkGetPhysicalDeviceProperties2 as vk::PFN_vkGetPhysicalDeviceProperties2,
            )
        }
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
            // get_instance_proc_addr: std::mem::transmute(get_function(
            //     layer_info,
            //     *p_instance,
            //     b"vkGetInstanceProcAddr\0",
            // )),
            get_instance_proc_addr: layer_info.pfnNextGetInstanceProcAddr,
            get_physical_device_properties: std::mem::transmute(get_function(
                layer_info,
                *p_instance,
                b"vkGetPhysicalDeviceProperties\0",
            )),
            get_physical_device_properties2: std::mem::transmute(get_function(
                layer_info,
                *p_instance,
                b"vkGetPhysicalDeviceProperties2\0",
            )),
        },
    );

    ret
}

unsafe extern "system" fn vkGetPhysicalDeviceProperties(
    physical_device: vk::PhysicalDevice,
    p_properties: *mut vk::PhysicalDeviceProperties,
) {
    let instance_dispatch = INSTANCE_DISPATCH.lock().unwrap();
    let loader_dispatch_table = *std::mem::transmute::<_, *const usize>(physical_device);
    let instance_dispatch = instance_dispatch.get(&loader_dispatch_table).unwrap();

    (instance_dispatch.get_physical_device_properties)(physical_device, p_properties);

    let properties = p_properties.as_mut().unwrap();
    dbg!(properties);
}

unsafe extern "system" fn vkGetPhysicalDeviceProperties2(
    physical_device: vk::PhysicalDevice,
    p_properties2: *mut vk::PhysicalDeviceProperties2,
) {
    let instance_dispatch = INSTANCE_DISPATCH.lock().unwrap();
    let loader_dispatch_table = *std::mem::transmute::<_, *const usize>(physical_device);
    let instance_dispatch = instance_dispatch.get(&loader_dispatch_table).unwrap();

    (instance_dispatch.get_physical_device_properties2)(physical_device, p_properties2);

    let properties2 = p_properties2.as_mut().unwrap();

    let driver_id = vk::ptr_chain_iter(properties2).find_map(|p| {
        ash::match_out_struct!(match p {
            physical_device_driver_properties @ vk::PhysicalDeviceDriverProperties => {
                Some(physical_device_driver_properties.driver_id)
            }
            physical_device_vulkan_1_2_properties @ vk::PhysicalDeviceDriverProperties => {
                Some(physical_device_vulkan_1_2_properties.driver_id)
            }
            _ => {
                None
            }
        })
    });

    let properties = &mut properties2.properties;

    println!(
        "Found driver {:?} {:?} ({})",
        driver_id,
        version_from_vulkan(properties.driver_version),
        properties.driver_version
    );

    for p in vk::ptr_chain_iter(properties2) {
        // eprintln!("Properties2 next {:p}: {:?}", p, (*p).s_type);

        ash::match_out_struct!(match p {
            physical_device_driver_properties @ vk::PhysicalDeviceDriverProperties => {
                dbg!(&physical_device_driver_properties);
            }
            physical_device_vulkan_1_2_properties @ vk::PhysicalDeviceVulkan12Properties => {
                dbg!(&physical_device_vulkan_1_2_properties);
            }
            physical_device_id_properties @ vk::PhysicalDeviceIDProperties => {
                dbg!(&physical_device_id_properties);
            }
            physical_device_vulkan_1_1_properties @ vk::PhysicalDeviceVulkan11Properties => {
                dbg!(&physical_device_vulkan_1_1_properties);
            }
        })
    }
}
