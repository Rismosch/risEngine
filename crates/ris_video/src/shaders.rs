use std::sync::Arc;

use vulkano::device::Device;
use vulkano::shader::ShaderModule;

use ris_asset::asset_loader::AssetId;
use ris_util::ris_error::RisError;

pub type Shaders = (Arc<ShaderModule>, Arc<ShaderModule>);

pub fn load_shaders(device: &Arc<Device>) -> Result<Shaders, RisError> {
    let vertex_id = AssetId::Directory(String::from("__imported_raw_assets/default.vert.spirv"));
    let fragmend_id = AssetId::Directory(String::from("__imported_raw_assets/default.frag.spirv"));

    let vertex_future = ris_asset::asset_loader::load(vertex_id);
    let fragment_future = ris_asset::asset_loader::load(fragmend_id);

    let vertex_bytes = ris_util::unroll!(vertex_future.wait(), "failed to load vertex asset")?;
    let fragment_bytes =
        ris_util::unroll!(fragment_future.wait(), "failed to load fragment asset")?;

    let vertex_shader = ris_util::unroll!(
        unsafe { ShaderModule::from_bytes(device.clone(), &vertex_bytes) },
        "failed to load vertex shader module"
    )?;

    let fragment_shader = ris_util::unroll!(
        unsafe { ShaderModule::from_bytes(device.clone(), &fragment_bytes) },
        "failed to lad fragment shader module"
    )?;

    Ok((vertex_shader, fragment_shader))
}
