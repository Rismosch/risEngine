pub mod commands;
pub mod ci_error;
pub mod util;

pub use ci_error::CiResult;
pub use commands::*;

struct Command {
    name: String,
    run: Box<dyn Fn(Vec<String>) -> CiResult<()>>,
    usage: Box<dyn Fn() -> &'static str>,
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
    let command = commands
        .into_iter()
        .find(|x| x.name == trimmed_arg);

    match command {
        Some(Command { name: _, run, usage }) => {
            if raw_args.len() > 2 {
                let arg2 = &raw_args[2];
                if is_help_arg(arg2) {
                    eprintln!("usage: {}", usage());
                    return;
                }
            }

            let result = run(raw_args);
            if let Err(error) = result {
                eprintln!("ERROR: {}", error);
                return;
            }
        },
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

    arg == "h" ||
        arg == "-h" ||
        arg == "--h" ||
        arg == "help" ||
        arg == "-help" ||
        arg == "--help" ||
        arg == "man" ||
        arg == "-man" ||
        arg == "--man" ||
        arg == "manual" ||
        arg == "-manual" ||
        arg == "--manual"
}
