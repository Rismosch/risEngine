use std::sync::Arc;

use vulkano::device::Device;
use vulkano::pipeline::graphics::color_blend::AttachmentBlend;
use vulkano::pipeline::graphics::color_blend::ColorBlendAttachmentState;
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::color_blend::ColorComponents;
use vulkano::pipeline::graphics::depth_stencil::CompareOp;
use vulkano::pipeline::graphics::depth_stencil::DepthState;
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::rasterization::CullMode;
use vulkano::pipeline::graphics::rasterization::FrontFace;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::StateMode;
use vulkano::render_pass::RenderPass;
use vulkano::render_pass::Subpass;
use vulkano::shader::ShaderModule;

use ris_error::RisResult;

use crate::imgui::gpu_objects::ImguiVertex;
use crate::vulkan::gpu_objects::Vertex3d;

pub fn create_pipeline(
    device: Arc<Device>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    render_pass: Arc<RenderPass>,
    viewport: &Viewport,
) -> RisResult<Arc<GraphicsPipeline>> {
    ris_error::unroll!(
        GraphicsPipeline::start()
            .vertex_input_state(ImguiVertex::input_state())
            .vertex_shader(
                ris_error::unroll_option!(
                    vs.clone().entry_point("main"),
                    "failed to locate vertex entry point",
                )?,
                (),
            )
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([
                viewport.clone()
            ]))
            .fragment_shader(
                ris_error::unroll_option!(
                    fs.clone().entry_point("main"),
                    "failed to locate fragment entry point",
                )?,
                (),
            )
            .color_blend_state(ColorBlendState {
                attachments: vec![ColorBlendAttachmentState {
                    blend: Some(AttachmentBlend::alpha()),
                    color_write_mask: ColorComponents::all(),
                    color_write_enable: StateMode::Fixed(true),
                }],
                ..Default::default()
            })
            .render_pass(ris_error::unroll_option!(
                    Subpass::from(render_pass.clone(), 0),
                    "failed to create render subpass",
            )?)
            .build(device.clone()),
        "failed to build graphics pipeline",
    )
}
