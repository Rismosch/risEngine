pub mod ci_error;
pub mod cmd_stream;
pub mod commands;
pub mod util;

use std::path::Path;
use std::path::PathBuf;

pub use ci_error::CiResult;
pub use ci_error::CiResultExtensions;
pub use cmd_stream::CmdStream;
pub use cmd_stream::EmptyWrite;
pub use commands::*;

struct Command {
    name: String,
    run: Box<dyn Fn(Vec<String>, PathBuf, PathBuf) -> CiResult<()>>,
    usage: Box<dyn Fn() -> String>,
}

macro_rules! command {
    ($cmd:ident) => {{
        Command {
            name: stringify!($cmd).to_string(),
            run: Box::new($cmd::run),
            usage: Box::new($cmd::usage),
        }
    }};
}

macro_rules! command_vec {
    ($($cmd:ident),+ $(,)*) => {{
        vec![$(command!($cmd)),+]
    }};
}

fn main() {
    let commands = command_vec!(
        archive,
        build,
        clean,
        doc,
        pipeline,
    );

    let raw_args = std::env::args().collect::<Vec<_>>();

    if raw_args.len() < 2 {
        print_help(None);
        return;
    }

    let arg1 = &raw_args[1];
    if is_help_arg(arg1) {
        print_help(Some(commands));
        return;
    }

    let trimmed_arg = arg1.trim().to_lowercase();
    let command = commands.into_iter().find(|x| x.name == trimmed_arg);

    match command {
        Some(Command { name, run, usage }) => {
            if raw_args.len() > 2 {
                let arg2 = &raw_args[2];
                if is_help_arg(arg2) {
                    eprintln!("usage: {}", usage());
                    return;
                }
            }

            let start = std::time::SystemTime::now();

            let result = match get_target_dir(&raw_args[0], &name) {
                Ok((target_dir, log_dir)) => run(raw_args, target_dir, log_dir),
                Err(e) => Err(e),
            };

            match result {
                Ok(()) => {
                    let end = std::time::SystemTime::now();
                    if let Ok(duration) = end.duration_since(start) {
                        eprintln!("finished in {:?}", duration);
                    }
                }
                Err(error) => eprintln!("ERROR: {}", error),
            }
        }
        None => {
            eprintln!("unkown command: {}", arg1);
        }
    }
}

fn print_help(to_print: Option<Vec<Command>>) {
    let name = env!("CARGO_PKG_NAME");
    eprintln!("usage: {} <command> [help]", name);

    if let Some(commands) = to_print {
        eprintln!("commands:");
        for command in commands {
            eprintln!("    {}", (command.usage)());
        }
    } else {
        eprintln!("use `{} help` to list all available commands", name);
    }
}

fn is_help_arg(arg: &str) -> bool {
    let arg = arg.trim().to_lowercase();

    arg == "h"
        || arg == "-h"
        || arg == "--h"
        || arg == "help"
        || arg == "-help"
        || arg == "--help"
        || arg == "man"
        || arg == "-man"
        || arg == "--man"
        || arg == "manual"
        || arg == "-manual"
        || arg == "--manual"
}

fn get_target_dir(program: &str, command: &str) -> CiResult<(PathBuf, PathBuf)> {
    let parent = match get_root_dir() {
        Ok(root_dir) => root_dir,
        Err(_) => PathBuf::from(program)
            .parent()
            .to_ci_result()?
            .to_path_buf(),
    };

    let target_dir = parent.join("ci_out").join(command);
    let log_dir = parent.join("ci_out").join("log").join(command);

    Ok((target_dir, log_dir))
}

fn get_root_dir() -> CiResult<PathBuf> {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()?
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output)?.trim());

    let root_dir = cargo_path.parent().to_ci_result()?.to_path_buf();

    Ok(root_dir)
}