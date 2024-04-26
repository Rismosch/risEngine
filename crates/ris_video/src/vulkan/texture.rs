use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::c_void;
use std::ptr;

use ash::vk;

use ris_asset::AssetId;
use ris_asset::codecs::qoi;
use ris_error::Extensions;
use ris_error::RisResult;

use super::buffer::Buffer;
use super::image::Image;

pub struct Texture {
    pub image: Image,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
}

impl Texture {
    pub fn alloc(
        device: &ash::Device,
        queue: &vk::Queue,
        transient_command_pool: &vk::CommandPool,
        physical_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
        physical_device_properties: vk::PhysicalDeviceProperties,
        asset_id: AssetId,
    ) -> RisResult<Self> {

        // load asset
        let content = ris_asset::load_async(asset_id.clone()).wait(None)??;
        let (pixels, desc) = qoi::decode(&content, None)?;

        let pixels_rgba = match desc.channels {
            qoi::Channels::RGB => {
                ris_log::trace!("adding alpha channel to texture asset... {:?}", asset_id);

                ris_error::assert!(pixels.len() % 3 == 0)?;
                let pixels_rgba_len = (pixels.len() * 4) / 3;
                let mut pixels_rgba = Vec::with_capacity(pixels_rgba_len);

                for chunk in pixels.chunks_exact(3) {
                    let r = chunk[0];
                    let g = chunk[1];
                    let b = chunk[2];
                    let a = u8::MAX;

                    pixels_rgba.push(r);
                    pixels_rgba.push(g);
                    pixels_rgba.push(b);
                    pixels_rgba.push(a);
                }

                ris_log::trace!("added alpha channel to texture asset! {:?}", asset_id);

                pixels_rgba
            },
            qoi::Channels::RGBA => pixels,
        };

        // create image and copy asset to it
        let staging_buffer = Buffer::alloc(
            &device,
            pixels_rgba.len() as vk::DeviceSize,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            physical_device_memory_properties,
        )?;

        staging_buffer.write(&device, &pixels_rgba)?;

        let image = Image::alloc(
            &device,
            desc.width,
            desc.height,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &physical_device_memory_properties,
        )?;

        image.transition_layout(
            &device,
            &queue,
            &transient_command_pool,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )?;

        staging_buffer.copy_to_image(
            &device,
            &queue,
            &transient_command_pool,
            image.image,
            desc.width,
            desc.height,
        )?;

        image.transition_layout(
            &device,
            &queue,
            &transient_command_pool,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )?;

        staging_buffer.free(&device);

        // create image view
        let image_view_create_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::ImageViewCreateFlags::empty(),
            image: image.image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: vk::Format::R8G8B8A8_SRGB,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            },
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };

        let view = unsafe{
            device.create_image_view(&image_view_create_info, None)
        }?;

        // create sampler
        let sampler_create_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::SamplerCreateFlags::empty(),
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            mip_lod_bias: 0.0,
            anisotropy_enable: vk::TRUE,
            max_anisotropy: physical_device_properties.limits.max_sampler_anisotropy,
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::ALWAYS,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
        };

        let sampler = unsafe{device.create_sampler(&sampler_create_info, None)}?;

        Ok(Self{
            image,
            view,
            sampler,
        })
    }

    pub fn free(&self, device: &ash::Device) {
        unsafe{
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.view, None);
        }

        self.image.free(device);
    }
}