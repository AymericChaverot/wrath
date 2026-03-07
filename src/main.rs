mod cli;
mod error;
mod port;
mod process;
mod update;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Target};

fn main() -> ExitCode {
    let args = Cli::parse();
    let target = cli::parse_target(&args.target);
    let system = process::create_system();

    let result = match target {
        Target::Port(port) => match port::resolve_port_to_pid(port) {
            Ok(pid) => process::kill_by_pid(&system, pid, port),
            Err(e) => Err(e),
        },
        Target::Name(name) => process::kill_by_name(&system, &name),
    };

    let exit_code = match result {
        Ok(kill_result) => {
            println!("{}", kill_result.message());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[!] {e}");
            ExitCode::FAILURE
        }
    };

    // Check for updates (non-blocking, silently ignores failures)
    let current_version = env!("CARGO_PKG_VERSION");
    if let Some(latest) = update::check_for_update(current_version) {
        eprintln!("{}", update::update_message(current_version, &latest));
    }

    exit_code
}
