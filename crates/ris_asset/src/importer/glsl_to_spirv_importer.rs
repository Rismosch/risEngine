use std::io::Read;
use std::io::Seek;
use std::io::Write;

use ris_error::RisResult;

pub const IN_EXT: &str = "glsl";
pub const OUT_EXT: &str = "spirv";

pub fn import(
    filename: &str,
    input: &mut (impl Read + Seek),
    output: &mut (impl Write + Seek),
) -> RisResult<()> {
    let file_size = ris_file::seek!(input, SeekFrom::End(0))?;
    ris_file::seek!(input, SeekFrom::Start(0))?;
    let mut file_content = vec![0u8; file_size as usize];
    ris_file::read!(input, file_content)?;
    let source_text = ris_error::unroll!(
        String::from_utf8(file_content),
        "failed to convert source to string",
    )?;

    let compiler = ris_error::unroll_option!(
        shaderc::Compiler::new(),
        "failed to initialize shaderc compiler"
    )?;
    let mut options = ris_error::unroll_option!(
        shaderc::CompileOptions::new(),
        "failed to initialize shaderc options"
    )?;
    options.set_warnings_as_errors();
    options.set_optimization_level(shaderc::OptimizationLevel::Performance);

    let artifact = ris_error::unroll!(
        compiler.compile_into_spirv(
            &source_text,
            shaderc::ShaderKind::InferFromSource,
            filename,
            "main",
            Some(&options),
        ),
        "failed to compile shader {}",
        filename
    )?;
    let bytes = artifact.as_binary_u8();
    ris_file::write!(output, bytes)?;

    Ok(())
}
