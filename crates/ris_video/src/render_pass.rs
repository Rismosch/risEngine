use std::sync::Arc;

use vulkano::device::Device;
use vulkano::render_pass::RenderPass;
use vulkano::swapchain::Swapchain;

use ris_util::ris_error::RisError;

pub fn create_render_pass(
    device: &Arc<Device>,
    swapchain: &Arc<Swapchain>,
) -> Result<Arc<RenderPass>, RisError> {
    ris_util::unroll!(
        vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: super::DEPTH_FORMAT,
                    samples: 1,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {depth},
            },
        ),
        "failed to create render pass"
    )
}
