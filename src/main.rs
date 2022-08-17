use ris_core::engine::Engine;
use ris_data::info::package_info::PackageInfo;
use ris_data::package_info;
use ris_log::{
    appenders::{console_appender::ConsoleAppender, i_appender::IAppender},
    log,
    log_level::LogLevel,
};

fn main() -> Result<(), String> {
    let appenders: Vec<Box<(dyn IAppender + 'static)>> = vec![ConsoleAppender::new()];
    log::init(LogLevel::Trace, appenders);

    let package_info = package_info!();
    let result = run_engine(package_info);

    log::drop();

    result
}

fn run_engine(package_info: PackageInfo) -> Result<(), String> {
    Engine::new(package_info)?.run()?;

    Ok(())
}