use std::sync::Arc;

use vulkano::device::physical::PhysicalDevice;
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::DeviceExtensions;
use vulkano::device::QueueFlags;
use vulkano::instance::Instance;
use vulkano::swapchain::Surface;

pub fn select_physical_device(
    instance: &Arc<Instance>,
    surface: &Arc<Surface>,
    device_extensions: &DeviceExtensions,
) -> Result<(Arc<PhysicalDevice>, u32), String> {
    Ok(instance
        .enumerate_physical_devices()
        .map_err(|e| format!("failed to enumerate_physical_devices: {}", e))?
        .filter(|p| p.supported_extensions().contains(device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.contains(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, surface).unwrap_or(false)
                })
                .map(|q| (p, q as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
            _ => 5,
        })
        .ok_or("no devices available")?)
}