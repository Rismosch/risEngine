use std::ffi::CString;
use std::ptr;

use ash::vk;

use imgui::Context;
use imgui::DrawCmd;
use imgui::DrawCmdParams;
use imgui::DrawData;
use imgui::DrawVert;
use imgui::TextureId;
use imgui::Textures;

use ris_asset::RisGodAsset;
use ris_error::Extensions;
use ris_error::RisResult;
use ris_math::matrix::Mat4;

use crate::vulkan::buffer::Buffer;
use crate::vulkan::renderer::Renderer;
use crate::vulkan::swapchain::BaseSwapchain;
use crate::vulkan::swapchain::Swapchain;
use crate::vulkan::texture::Texture;
use crate::vulkan::transient_command::TransientCommand;

pub struct ImguiRenderer {
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    font_texture: Texture,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
    textures: Textures<vk::DescriptorSet>,
    frames: Option<Frames>,
}

impl ImguiRenderer {
    pub fn free(&mut self, device: &ash::Device) {
        unsafe {
            if let Some(frames) = self.frames.take() {
                frames.free(device);
            }

            self.font_texture.free(device);

            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            device.destroy_render_pass(self.render_pass, None);
        }
    }

    pub fn init(renderer: &Renderer, god_asset: &RisGodAsset, context: &mut Context) -> RisResult<Self> {
        let Renderer {
            instance,
            suitable_device,
            device,
            graphics_queue,
            transient_command_pool,
            swapchain : Swapchain {
                base: BaseSwapchain {
                    format: swapchain_format,
                    ..
                },
                ..
            },
            ..
        } = renderer;

        // shaders
        let vs_asset_future = ris_asset::load_async(god_asset.imgui_vs.clone());
        let fs_asset_future = ris_asset::load_async(god_asset.imgui_fs.clone());

        let vs_bytes = vs_asset_future.wait(None)??;
        let fs_bytes = fs_asset_future.wait(None)??;

        // asset data is read in u8, but vulkan expects it to be in u32.
        // assert that the data is properly aligned
        ris_error::assert!(vs_bytes.len() % 4 == 0)?;
        ris_error::assert!(fs_bytes.len() % 4 == 0)?;

        let vs_shader_module_create_info = vk::ShaderModuleCreateInfo {
            s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::ShaderModuleCreateFlags::empty(),
            code_size: vs_bytes.len(),
            p_code: vs_bytes.as_ptr() as *const u32,
        };
        let fs_shader_module_create_info = vk::ShaderModuleCreateInfo {
            s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::ShaderModuleCreateFlags::empty(),
            code_size: fs_bytes.len(),
            p_code: fs_bytes.as_ptr() as *const u32,
        };

        let vs_shader_module =
            unsafe { device.create_shader_module(&vs_shader_module_create_info, None) }?;
        let fs_shader_module =
            unsafe { device.create_shader_module(&fs_shader_module_create_info, None) }?;

        let main_function_name = CString::new("main").unwrap();

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::PipelineShaderStageCreateFlags::empty(),
                module: vs_shader_module,
                p_name: main_function_name.as_ptr(),
                p_specialization_info: ptr::null(),
                stage: vk::ShaderStageFlags::VERTEX,
            },
            vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::PipelineShaderStageCreateFlags::empty(),
                module: fs_shader_module,
                p_name: main_function_name.as_ptr(),
                p_specialization_info: ptr::null(),
                stage: vk::ShaderStageFlags::FRAGMENT,
            },
        ];

        // pipeline
        let vertex_binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: 20,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let vertex_attribute_descriptions = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 8 as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R8G8B8A8_UNORM,
                offset: 16 as u32,
            },
        ];

        let vertex_input_state = [vk::PipelineVertexInputStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineVertexInputStateCreateFlags::empty(),
            vertex_binding_description_count: vertex_binding_descriptions.len() as u32,
            p_vertex_binding_descriptions: vertex_binding_descriptions.as_ptr(),
            vertex_attribute_description_count: vertex_attribute_descriptions.len() as u32,
            p_vertex_attribute_descriptions: vertex_attribute_descriptions.as_ptr(),
        }];

        let input_assembly_state = [vk::PipelineInputAssemblyStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
        }];

        let viewports = [Default::default()];
        let scissors = [Default::default()];

        let viewport_state = [vk::PipelineViewportStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineViewportStateCreateFlags::empty(),
            viewport_count: 1,
            p_viewports: viewports.as_ptr(),
            scissor_count: 1,
            p_scissors: scissors.as_ptr(),
        }];

        let rasterization_state = [vk::PipelineRasterizationStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineRasterizationStateCreateFlags::empty(),
            depth_clamp_enable: vk::FALSE,
            rasterizer_discard_enable: vk::FALSE,
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::CLOCKWISE,
            depth_bias_enable: vk::FALSE,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        }];

        let multisample_state = [vk::PipelineMultisampleStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineMultisampleStateCreateFlags::empty(),
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            sample_shading_enable: vk::FALSE,
            min_sample_shading: 1.,
            p_sample_mask: ptr::null(),
            alpha_to_coverage_enable: vk::FALSE,
            alpha_to_one_enable: vk::FALSE,
        }];

        let depth_stencil_state = [vk::PipelineDepthStencilStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineDepthStencilStateCreateFlags::empty(),
            depth_test_enable: vk::FALSE,
            depth_write_enable: vk::FALSE,
            depth_compare_op: vk::CompareOp::ALWAYS,
            depth_bounds_test_enable: vk::FALSE,
            stencil_test_enable: vk::FALSE,
            front: Default::default(),
            back: Default::default(),
            min_depth_bounds: 0.0,
            max_depth_bounds: 0.0,
        }];

        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::TRUE,
            src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        }];

        let color_blend_state = [vk::PipelineColorBlendStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineColorBlendStateCreateFlags::empty(),
            logic_op_enable: vk::FALSE,
            logic_op: vk::LogicOp::COPY,
            attachment_count: color_blend_attachment_states.len() as u32,
            p_attachments: color_blend_attachment_states.as_ptr(),
            blend_constants: [0., 0., 0., 0.],
        }];

        let dynamic_states = [vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT];
        let dynamic_state = [vk::PipelineDynamicStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineDynamicStateCreateFlags::empty(),
            dynamic_state_count: dynamic_states.len() as u32,
            p_dynamic_states: dynamic_states.as_ptr(),
        }];

        // pipeline layout
        let descriptor_set_layout_bindings = [
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: ptr::null(),
            },
        ];

        let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::DescriptorSetLayoutCreateFlags::empty(),
            binding_count: descriptor_set_layout_bindings.len() as u32,
            p_bindings: descriptor_set_layout_bindings.as_ptr(),
        };

        let descriptor_set_layout = unsafe {
            device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
        }?;

        let descriptor_set_layouts = [descriptor_set_layout];

        let push_constant_ranges = [vk::PushConstantRange{
            stage_flags: vk::ShaderStageFlags::VERTEX,
            offset: 0,
            size: std::mem::size_of::<Mat4>() as u32,
        }];

        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo {
            s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineLayoutCreateFlags::empty(),
            set_layout_count: descriptor_set_layouts.len() as u32,
            p_set_layouts: descriptor_set_layouts.as_ptr(),
            push_constant_range_count: push_constant_ranges.len() as u32,
            p_push_constant_ranges: push_constant_ranges.as_ptr(),
        };

        let pipeline_layout = unsafe { device.create_pipeline_layout(&pipeline_layout_create_info, None) }?;

        // render pass
        let color_attachment = vk::AttachmentDescription {
            flags: vk::AttachmentDescriptionFlags::empty(),
            format: swapchain_format.format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::LOAD,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
        };

        let attachments = [color_attachment];

        let color_attachment_references = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];

        let subpass_descriptions = [vk::SubpassDescription {
            flags: vk::SubpassDescriptionFlags::empty(),
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: ptr::null(),
            color_attachment_count: color_attachment_references.len() as u32,
            p_color_attachments: color_attachment_references.as_ptr(),
            p_resolve_attachments: ptr::null(),
            p_depth_stencil_attachment: ptr::null(),
            preserve_attachment_count: 0,
            p_preserve_attachments: ptr::null(),
        }];

        let supbass_dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dependency_flags: vk::DependencyFlags::empty(),
        }];

        let render_pass_create_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::RenderPassCreateFlags::empty(),
            attachment_count: attachments.len() as u32,
            p_attachments: attachments.as_ptr(),
            subpass_count: subpass_descriptions.len() as u32,
            p_subpasses: subpass_descriptions.as_ptr(),
            dependency_count: supbass_dependencies.len() as u32,
            p_dependencies: supbass_dependencies.as_ptr(),
        };

        let render_pass = unsafe { device.create_render_pass(&render_pass_create_info, None) }?;

        // pipeline creation
        let graphics_pipeline_create_info = [vk::GraphicsPipelineCreateInfo {
            s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineCreateFlags::empty(),
            stage_count: shader_stages.len() as u32,
            p_stages: shader_stages.as_ptr(),
            p_vertex_input_state: vertex_input_state.as_ptr(),
            p_input_assembly_state: input_assembly_state.as_ptr(),
            p_tessellation_state: ptr::null(),
            p_viewport_state: viewport_state.as_ptr(),
            p_rasterization_state: rasterization_state.as_ptr(),
            p_multisample_state: multisample_state.as_ptr(),
            p_depth_stencil_state: depth_stencil_state.as_ptr(),
            p_color_blend_state: color_blend_state.as_ptr(),
            p_dynamic_state: dynamic_state.as_ptr(),
            layout: pipeline_layout,
            render_pass,
            subpass: 0,
            base_pipeline_handle: vk::Pipeline::null(),
            base_pipeline_index: -1,
        }];

        let graphics_pipelines = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &graphics_pipeline_create_info,
                None,
            )
        }
        .map_err(|e| e.1)?;
        let pipeline = graphics_pipelines.into_iter().next().unroll()?;

        unsafe { device.destroy_shader_module(vs_shader_module, None) };
        unsafe { device.destroy_shader_module(fs_shader_module, None) };

        // textures
        let font_atlas_texture = context.fonts().build_rgba32_texture();

        let physical_device_memory_properties = unsafe {
            instance.get_physical_device_memory_properties(suitable_device.physical_device)
        };
        let physical_device_properties =
            unsafe { instance.get_physical_device_properties(suitable_device.physical_device) };

        let font_texture = Texture::alloc(
            device,
            *graphics_queue,
            *transient_command_pool,
            physical_device_memory_properties,
            physical_device_properties,
            font_atlas_texture.width,
            font_atlas_texture.height,
            font_atlas_texture.data,
        )?;

        let fonts = context.fonts();
        fonts.tex_id = TextureId::from(usize::MAX);

        // descriptor pool
        let descriptor_pool_sizes = [vk::DescriptorPoolSize{
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
        }];

        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::DescriptorPoolCreateFlags::empty(),
            max_sets: 1,
            pool_size_count: descriptor_pool_sizes.len() as u32,
            p_pool_sizes: descriptor_pool_sizes.as_ptr(),
        };
        let descriptor_pool = unsafe{device.create_descriptor_pool(&descriptor_pool_create_info, None)}?;

        // descriptor set
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next: ptr::null(),
            descriptor_pool,
            descriptor_set_count: descriptor_set_layouts.len() as u32,
            p_set_layouts: descriptor_set_layouts.as_ptr(),
        };

        let descriptor_sets = unsafe{device.allocate_descriptor_sets(&descriptor_set_allocate_info)}?;
        let descriptor_set = descriptor_sets.into_iter().next().unroll()?;

        let image_infos = [vk::DescriptorImageInfo{
            sampler: font_texture.sampler,
            image_view: font_texture.view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        }];

        let write_descriptor_sets = [vk::WriteDescriptorSet{
            s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
            p_next: ptr::null(),
            dst_set: descriptor_set,
            dst_binding: 0,
            dst_array_element: 0,
            descriptor_count: image_infos.len() as u32,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            p_image_info: image_infos.as_ptr(),
            p_buffer_info: ptr::null(),
            p_texel_buffer_view: ptr::null(),
        }];
        
        unsafe{device.update_descriptor_sets(&write_descriptor_sets, &[])};

        // init
        context.set_renderer_name(Some(String::from("ris_engine vulkan renderer")));

        Ok(Self {
            descriptor_set_layout,
            pipeline_layout,
            render_pass,
            pipeline,
            font_texture,
            descriptor_pool,
            descriptor_set,
            textures: Textures::new(),
            frames: None,
        })
    }

    pub fn draw(
        &mut self,
        renderer: &Renderer,
        target: vk::ImageView,
        draw_data: &DrawData,
    ) -> RisResult<()>
    {
        if draw_data.total_vtx_count == 0 {
            return Ok(());
        }

        let Renderer {
            instance,
            suitable_device,
            device,
            graphics_queue,
            transient_command_pool,
            swapchain: Swapchain {
                base: BaseSwapchain {
                    extent: swapchain_extent,
                    ..
                },
                entries,
                ..
            },
            ..
        } = renderer;

        let physical_device_memory_properties = unsafe {
            instance.get_physical_device_memory_properties(suitable_device.physical_device)
        };
        let physical_device_properties =
            unsafe { instance.get_physical_device_properties(suitable_device.physical_device)
        };

        if self.frames.is_none() {
            self.frames.replace(Frames::alloc(
                device,
                physical_device_memory_properties,
                draw_data,
                entries.len(),
            )?);
        }

        let mesh = self.frames.as_mut().unroll()?.next();
        mesh.update(
            device,
            physical_device_memory_properties,
            draw_data,
        )?;

        let transient_command = TransientCommand::begin(device, *graphics_queue, *transient_command_pool)?;

        let attachments = [target];

        let framebuffer_create_info = vk::FramebufferCreateInfo {
            s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::FramebufferCreateFlags::empty(),
            render_pass: self.render_pass,
            attachment_count: attachments.len() as u32,
            p_attachments: attachments.as_ptr(),
            width: swapchain_extent.width,
            height: swapchain_extent.height,
            layers: 1,
        };

        let framebuffer = unsafe { device.create_framebuffer(&framebuffer_create_info, None) }?;

        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
        ];


        let render_pass_begin_info = vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
            p_next: ptr::null(),
            render_pass: self.render_pass,
            framebuffer,
            render_area: vk::Rect2D {
                offset: vk::Offset2D {x: 0, y: 0},
                extent: *swapchain_extent,
            },
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
        };

        unsafe {device.cmd_begin_render_pass(
            transient_command.buffer(),
            &render_pass_begin_info,
            vk::SubpassContents::INLINE,
        )};

        unsafe {device.cmd_bind_pipeline(
            transient_command.buffer(),
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline,
        )};

        let framebuffer_width = draw_data.framebuffer_scale[0] * draw_data.display_size[0];
        let framebuffer_height = draw_data.framebuffer_scale[1] * draw_data.display_size[1];
        let viewports = [vk::Viewport{
            width: framebuffer_width,
            height: framebuffer_height,
            max_depth: 1.0,
            ..Default::default()
        }];

        unsafe {device.cmd_set_viewport(transient_command.buffer(), 0, &viewports)};

        let mut projection = Mat4::init(1.0);
        let rml = draw_data.display_size[0];
        let rpl = draw_data.display_size[0];
        let tmb = -draw_data.display_size[1];
        let tpb = -draw_data.display_size[1];
        let fmn = 2.0;
        projection.0.0 = 2.0 / rml;
        projection.1.1 = -2.0 / tmb;
        projection.2.2 = -1.0 / fmn;
        projection.3.0 = -(rpl / rml);
        projection.3.1 = -(tpb / tmb);
        projection.3.2 = 1.0 / fmn;
        projection.3.3 = 1.0;

        unsafe {
            let push_ptr = (&projection) as *const Mat4 as *const u8;
            let push = std::slice::from_raw_parts(push_ptr, std::mem::size_of::<Mat4>());

            device.cmd_push_constants(
                transient_command.buffer(),
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                push,
            );
        }

        unsafe {device.cmd_bind_index_buffer(
            transient_command.buffer(),
            mesh.indices.buffer,
            0,
            vk::IndexType::UINT16,
        )};

        unsafe {device.cmd_bind_vertex_buffers(
            transient_command.buffer(),
            0,
            &[mesh.vertices.buffer],
            &[0],
        )};

        let mut index_offset = 0;
        let mut vertex_offset = 0;
        let mut current_texture_id: Option<TextureId> = None;
        let clip_offset = draw_data.display_pos;
        let clip_scale = draw_data.framebuffer_scale;
        for draw_list in draw_data.draw_lists() {
            for command in draw_list.commands() {
                match command {
                    DrawCmd::Elements {
                        count,
                        cmd_params: DrawCmdParams {
                            clip_rect,
                            texture_id,
                            vtx_offset,
                            idx_offset,
                        },
                    } => {
                        let clip_x = (clip_rect[0] - clip_offset[0]) * clip_scale[0];
                        let clip_y = (clip_rect[1] - clip_offset[1]) * clip_scale[1];
                        let clip_w = (clip_rect[2] - clip_offset[0]) * clip_scale[0] - clip_x;
                        let clip_h = (clip_rect[3] - clip_offset[1]) * clip_scale[1] - clip_y;

                        let scissors = [vk::Rect2D {
                            offset: vk::Offset2D {
                                x: (clip_x as i32).max(0),
                                y: (clip_y as i32).max(0),
                            },
                            extent: vk::Extent2D {
                                width: clip_w as u32,
                                height: clip_h as u32,
                            },
                        }];

                        unsafe {device.cmd_set_scissor(transient_command.buffer(), 0, &scissors)};

                        if Some(texture_id) != current_texture_id {
                            let descriptor_set = self.lookup_descriptor_set(texture_id)?;
                            unsafe {
                                device.cmd_bind_descriptor_sets(
                                    transient_command.buffer(),
                                    vk::PipelineBindPoint::GRAPHICS,
                                    self.pipeline_layout,
                                    0,
                                    &[descriptor_set],
                                    &[],
                                )
                            };
                        }

                        unsafe {
                            device.cmd_draw_indexed(
                                transient_command.buffer(),
                                count as u32,
                                1,
                                index_offset + idx_offset as u32,
                                vertex_offset + vtx_offset as i32,
                                0,
                            )
                        }
                    },
                    DrawCmd::ResetRenderState => {
                        ris_log::warning!("reset render state not supported");
                    },
                    DrawCmd::RawCallback { .. } => {
                        ris_log::warning!("raw callback not supported");
                    }
                }
            }

            index_offset += draw_list.idx_buffer().len() as u32;
            vertex_offset += draw_list.vtx_buffer().len() as i32;
        }

        unsafe{device.cmd_end_render_pass(transient_command.buffer())};
        transient_command.end_and_submit(&[], &[], vk::Fence::null())?;
        unsafe{device.destroy_framebuffer(framebuffer, None)};

        Ok(())
    }

    fn lookup_descriptor_set(&self, texture_id: TextureId) -> RisResult<vk::DescriptorSet> {
        if texture_id.id() == usize::MAX {
            Ok(self.descriptor_set)
        } else if let Some(descriptor_set) = self.textures.get(texture_id) {
            Ok(*descriptor_set)
        } else {
            ris_error::new_result!("bad texture: {:?}", texture_id)
        }
    }
}

struct Frames {
    index: usize,
    count: usize,
    meshes: Vec<Mesh>,
}

impl Frames {
    fn alloc(
        device: &ash::Device,
        physical_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
        draw_data: &DrawData,
        count: usize,
    ) -> RisResult<Self> {
        let meshes = (0..count)
            .map(|_| Mesh::alloc(
                device,
                physical_device_memory_properties,
                draw_data,
            ))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            index: 0,
            count,
            meshes,
        })
    }

    fn free(&self, device: &ash::Device) {
        for mesh in self.meshes.iter() {
            mesh.free(device);
        }
    }

    fn next(&mut self) -> &mut Mesh {
        let result = &mut self.meshes[self.index];
        self.index = (self.index + 1) % self.count;
        result
    }
}

struct Mesh {
    vertices: Buffer,
    vertex_count: usize,
    indices: Buffer,
    index_count: usize,
}

impl Mesh {
    fn alloc(
        device: &ash::Device,
        physical_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
        draw_data: &DrawData,
    ) -> RisResult<Self> {
        let vertices = Self::create_vertices(draw_data);
        let vertex_count = vertices.len();
        let indices = Self::create_indices(draw_data);
        let index_count = vertices.len();

        let vertex_buffer_size = std::mem::size_of_val(vertices.as_slice()) as vk::DeviceSize;
        let vertex_buffer = Buffer::alloc(
            device,
            vertex_buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::DEVICE_LOCAL,
            physical_device_memory_properties,
        )?;

        vertex_buffer.write(device, &vertices)?;

        let index_buffer_size = std::mem::size_of_val(indices.as_slice()) as vk::DeviceSize;
        let index_buffer = Buffer::alloc(
            device,
            index_buffer_size,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::DEVICE_LOCAL,
            physical_device_memory_properties,
        )?;

        index_buffer.write(device, &indices)?;

        Ok(Self{
            vertices: vertex_buffer,
            vertex_count,
            indices: index_buffer,
            index_count,
        })
    }

    fn free(&self, device: &ash::Device) {
        self.vertices.free(device);
        self.indices.free(device);
    }

    fn create_vertices(draw_data: &DrawData) -> Vec<DrawVert> {
        let vertex_count = draw_data.total_vtx_count as usize;
        let mut vertices = Vec::with_capacity(vertex_count);
        for draw_list in draw_data.draw_lists() {
            vertices.extend_from_slice(draw_list.vtx_buffer());
        }
        vertices
    }

    fn create_indices(draw_data: &DrawData) -> Vec<u16> {
        let index_count = draw_data.total_idx_count as usize;
        let mut indices = Vec::with_capacity(index_count);
        for draw_list in draw_data.draw_lists() {
            indices.extend_from_slice(draw_list.idx_buffer());
        }
        indices
    }

    fn update(
        &mut self,
        device: &ash::Device,
        physical_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
        draw_data: &DrawData,
    ) -> RisResult<()> {
        let vertices = Self::create_vertices(draw_data);
        let old_vertex_count = self.vertex_count;
        let new_vertex_count = draw_data.total_vtx_count as usize;

        if old_vertex_count < new_vertex_count {
            //ris_log::trace!("resizing vertex buffer from {} to {}...", old_vertex_count, new_vertex_count);

            let vertex_buffer_size = std::mem::size_of_val(vertices.as_slice()) as vk::DeviceSize;
            let new_vertex_buffer = Buffer::alloc(
                device,
                vertex_buffer_size,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::DEVICE_LOCAL,
                physical_device_memory_properties,
            )?;

            self.vertex_count = vertices.len();

            let old_buffer = self.vertices;
            self.vertices = new_vertex_buffer;

            old_buffer.free(device);
        }
        self.vertices.write(device, &vertices)?;

        let indices = Self::create_indices(draw_data);
        let old_index_count = self.index_count;
        let new_index_count = draw_data.total_idx_count as usize;

        if old_index_count < new_index_count {
            //ris_log::trace!("resizing index buffer from {} to {}...", old_index_count, new_index_count);

            let index_buffer_size = std::mem::size_of_val(indices.as_slice()) as vk::DeviceSize;
            let new_index_buffer = Buffer::alloc(
                device,
                index_buffer_size,
                vk::BufferUsageFlags::INDEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::DEVICE_LOCAL,
                physical_device_memory_properties,
            )?;

            self.index_count = indices.len();

            let old_buffer = self.indices;
            self.indices = new_index_buffer;

            old_buffer.free(device);
        }
        self.indices.write(device, &indices)?;

        Ok(())
    }
}
