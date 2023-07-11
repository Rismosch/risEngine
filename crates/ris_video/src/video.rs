use std::sync::Arc;
use sdl2::{video::Window, Sdl};
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::{
    allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo},
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassContents,
};
use vulkano::device::{
    physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo,
    Queue,
    QueueFlags,
};
use vulkano::image::{view::ImageView, ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo, InstanceExtensions};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator};
use vulkano::pipeline::{
    graphics::{
        input_assembly::InputAssemblyState,
        vertex_input::Vertex,
        viewport::{Viewport, ViewportState},
    },
    GraphicsPipeline,
};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::swapchain::{self, AcquireError, SwapchainPresentInfo, Surface, SurfaceApi, Swapchain, SwapchainCreateInfo, SwapchainCreationError};
use vulkano::{Handle, VulkanLibrary, VulkanObject};
use vulkano::shader::ShaderModule;
use vulkano::sync::{self, FlushError, GpuFuture, future::FenceSignalFuture};

#[derive(BufferContents, Vertex)]
#[repr(C)]
struct MyVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
}

pub enum DrawState {
    Ok,
    RecreateSwapchain,
    Err(String),
}

pub struct Video {
    window: Window,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    command_buffer_allocator: StandardCommandBufferAllocator,
    render_pass: Arc<RenderPass>,
    vertex_buffer: Subbuffer<[MyVertex]>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    viewport: Viewport,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
}

impl Video {
    pub fn new(sdl_context: &Sdl) -> Result<Video, String> {
        // window
        let video_subsystem = sdl_context.video()?;
        let window = video_subsystem
            .window("ris_engine", 640, 480)
            .resizable()
            .position_centered()
            .vulkan()
            .build()
            .map_err(|e| e.to_string())?;

        // instance
        let library = VulkanLibrary::new().map_err(|_| "no local vulkano library/dll")?;
        let instance_extensions = InstanceExtensions::from_iter(
            window
                .vulkan_instance_extensions()
                .map_err(|_| "failed to get vulkan instance extensions")?,
        );
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                enabled_extensions: instance_extensions,
                ..Default::default()
            },
        )
        .map_err(|_| "failed to create instance")?;

        // surface
        let surface_handle = window
            .vulkan_create_surface(instance.handle().as_raw() as _)
            .map_err(|_| "failed to create vulkan surface handle")?;
        let surface = unsafe {
            Surface::from_handle(
                instance.clone(),
                <_ as Handle>::from_raw(surface_handle),
                SurfaceApi::Win32,
                None,
            )
        };
        let surface = Arc::new(surface);

        // physical device
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .map_err(|_| "failed to enumerate devices")?
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.contains(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|q| (p, q as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                _ => 4,
            })
            .ok_or("no devices available")?;

        // device
        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                ..Default::default()
            },
        )
        .map_err(|_| "failed to create device")?;
        let queue = queues.next().ok_or("no queues available")?;

        // swapchain
        let capabilities = physical_device
            .surface_capabilities(&surface, Default::default())
            .map_err(|_| "failed to get surface capabilities")?;
        let dimensions = window.vulkan_drawable_size();
        let composite_alpha = capabilities
            .supported_composite_alpha
            .into_iter()
            .next()
            .ok_or("could not get supported composite alpha")?;
        let image_format = Some(
            physical_device
                .surface_formats(&surface, Default::default())
                .map_err(|_| "failed to get surface formats")?[0]
                .0,
        );
        let (swapchain, images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: capabilities.min_image_count + 1,
                image_format,
                image_extent: [dimensions.0, dimensions.1],
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha,
                ..Default::default()
            },
        )
        .map_err(|_| "failed to create swapchain")?;

        // render pass
        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                },
            },
            pass : {
                color: [color],
                depth_stencil: {},
            },
        )
        .map_err(|_| "failed to create render pass")?;

        // framebuffers
        let framebuffers = get_framebuffers(&images, &render_pass)?;

        // allocators
        let memory_allocator = StandardMemoryAllocator::new_default(device.clone());
        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        );

        // vertex buffer
        let vertex1 = MyVertex {
            position: [0.0, 0.5],
        };
        let vertex2 = MyVertex {
            position: [-0.5, -0.5],
        };
        let vertex3 = MyVertex {
            position: [0.5, -0.5],
        };

        let vertex_buffer = Buffer::from_iter(
            &memory_allocator,
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            vec![vertex1, vertex2, vertex3],
        )
        .map_err(|_| "failed to create vertex buffer")?;

        // shaders
        let vertex_source = "
            #version 460

            layout(location = 0) in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ";

        let fragment_source = "
            #version 460

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        ";

        let compiler = shaderc::Compiler::new().ok_or("failed to initialize shaderc compiler")?;
        let options =
            shaderc::CompileOptions::new().ok_or("could not initialize shaderc options")?;

        let vertex_artifact = compiler
            .compile_into_spirv(
                vertex_source,
                shaderc::ShaderKind::Vertex,
                "vertex.glsl",
                "main",
                Some(&options),
            )
            .map_err(|_| "failed to compile vertex shader")?;
        let vertex_words: &[u32] = vertex_artifact.as_binary();
        let vertex_shader =
            unsafe { vulkano::shader::ShaderModule::from_words(device.clone(), vertex_words) }
                .map_err(|_| "failed to load vertex shader module")?;

        let fragment_artifact = compiler
            .compile_into_spirv(
                fragment_source,
                shaderc::ShaderKind::Fragment,
                "fragment.glsl",
                "main",
                Some(&options),
            )
            .map_err(|_| "failed to compile fragment shader")?;
        let fragment_words: &[u32] = fragment_artifact.as_binary();
        let fragment_shader =
            unsafe { vulkano::shader::ShaderModule::from_words(device.clone(), fragment_words) }
                .map_err(|_| "failed to load fragment shader module")?;

        // viewport
        let (w, h) = window.vulkan_drawable_size();
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [w as f32, h as f32],
            depth_range: 0.0..1.0,
        };

        // pipeline
        let pipeline = get_pipeline(
            device.clone(),
            vertex_shader.clone(),
            fragment_shader.clone(),
            render_pass.clone(),
            viewport.clone(),
        )?;

        // command buffers
        let command_buffers = get_command_buffers(
            &command_buffer_allocator,
            &queue,
            &pipeline,
            &framebuffers,
            &vertex_buffer,
        )?;

        // initialization finished
        Ok(Video {
            window,
            device,
            queue,
            swapchain,
            render_pass,
            command_buffer_allocator,
            vertex_buffer,
            vertex_shader,
            fragment_shader,
            viewport,
            command_buffers,
        })
    }

    pub fn recreate_swapchain(
        &mut self,
        window_size_changed: bool,
        recreate_swapchain: bool
    ) -> Result<(), String> {
        if !window_size_changed && !recreate_swapchain {
            return Ok(());
        }

        ris_log::trace!("recreate swapchain...");

        let new_dimensions = self.window.vulkan_drawable_size();
        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: [new_dimensions.0, new_dimensions.1],
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return Ok(()),
            Err(e) => return Err(format!("failed to recreate swapchain: {}", e)),
        };

        if window_size_changed {
            let new_framebuffers = get_framebuffers(&new_images, &self.render_pass)?;
            self.viewport.dimensions = [new_dimensions.0 as f32, new_dimensions.1 as f32];
            let new_pipeline = get_pipeline(
                self.device.clone(),
                self.vertex_shader.clone(),
                self.fragment_shader.clone(),
                self.render_pass.clone(),
                self.viewport.clone()
            )?;
            let new_command_buffers = get_command_buffers(
                &self.command_buffer_allocator,
                &self.queue,
                &new_pipeline,
                &new_framebuffers,
                &self.vertex_buffer,
            )?;

            self.command_buffers = new_command_buffers;
        }

        self.swapchain = new_swapchain;

        ris_log::debug!("recreated swapchain!");
        Ok(())
    }

    pub fn draw(&mut self) -> DrawState {
        let (image_i, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => return DrawState::RecreateSwapchain,
                Err(e) => return DrawState::Err(format!("failed to acquire next image: {}", e)),
            };

        if suboptimal {
            return DrawState::RecreateSwapchain;
        }

        let execution = match sync::now(self.device.clone())
            .join(acquire_future)
            .then_execute(self.queue.clone(), self.command_buffers[image_i as usize].clone()) {
                Ok(x) => x,
                Err(_) => return DrawState::Err(String::from("failed to execute command buffer")),
            }
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
            )
            .then_signal_fence_and_flush();

        match execution {
            Ok(future) => {
                if let Err(e) = future.wait(None) {
                    DrawState::Err(format!("failed to wait on future: {}", e))
                } else {
                    DrawState::Ok
                }
            },
            Err(FlushError::OutOfDate) => DrawState::RecreateSwapchain,
            Err(e) => DrawState::Err(format!("failed to flush future: {}", e)),
        }
    }
}

fn get_framebuffers(
    images: &[Arc<SwapchainImage>],
    render_pass: &Arc<RenderPass>,
) -> Result<Vec<Arc<Framebuffer>>, String> {
    let mut framebuffers = Vec::new();
    for image in images {
        let view =
            ImageView::new_default(image.clone()).map_err(|_| "failed to create image view")?;
        let framebuffer = Framebuffer::new(
            render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![view],
                ..Default::default()
            },
        )
        .map_err(|_| "failed to create frame buffer")?;

        framebuffers.push(framebuffer);
    }

    Ok(framebuffers)
}

fn get_pipeline(
    device: Arc<Device>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    render_pass: Arc<RenderPass>,
    viewport: Viewport,
) -> Result<Arc<GraphicsPipeline>,String> {
    let pipeline = GraphicsPipeline::start()
        .vertex_input_state(MyVertex::per_vertex())
        .vertex_shader(
            vertex_shader.entry_point("main").ok_or("failed to locate vertex entry point")?,
            ()
        )
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport.clone()]))
        .fragment_shader(
            fragment_shader.entry_point("main").ok_or("failed to locate fragment entry point")?,
            ()
        )
        .render_pass(Subpass::from(render_pass.clone(), 0).ok_or("failed to create render subpass")?)
        .build(device.clone())
        .map_err(|_| "failed to build graphics pipeline")?;

    Ok(pipeline)
}

fn get_command_buffers(
    command_buffer_allocator: &StandardCommandBufferAllocator,
    queue: &Arc<Queue>,
    pipeline: &Arc<GraphicsPipeline>,
    framebuffers: &Vec<Arc<Framebuffer>>,
    vertex_buffer: &Subbuffer<[MyVertex]>,
) -> Result<Vec<Arc<PrimaryAutoCommandBuffer>>, String> {
    let mut command_buffers = Vec::new();
    for framebuffer in framebuffers {
        let mut builder = AutoCommandBufferBuilder::primary(
            command_buffer_allocator,
            queue.queue_family_index(),
            CommandBufferUsage::MultipleSubmit,
        )
        .map_err(|_| "failed to create auto command buffer builder")?;

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.1, 0.1, 0.1, 0.1].into())],
                    ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                },
                SubpassContents::Inline,
            )
            .map_err(|_| "failed to begin render pass")?
            .bind_pipeline_graphics(pipeline.clone())
            .bind_vertex_buffers(0, vertex_buffer.clone())
            .draw(vertex_buffer.len() as u32, 1, 0, 0)
            .map_err(|_| "failed to draw")?
            .end_render_pass()
            .map_err(|_| "failed to end render pass")?;

        let command_buffer = Arc::new(
            builder
                .build()
                .map_err(|_| "failed to build command buffer")?,
        );

        command_buffers.push(command_buffer);
    }

    Ok(command_buffers)
}
