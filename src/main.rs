mod cli;
mod error;
mod inspect;
mod port;
mod process;
mod ui;
mod update;

use std::io::{self, IsTerminal};
use std::process::ExitCode;
use std::time::Duration;

use anstream::{eprintln, println};
use clap::Parser;

use cli::{Cli, Target};
use error::WrathError;
use process::KillResult;

/// How long after startup wrath accepts to wait for the update check.
const UPDATE_CHECK_BUDGET: Duration = Duration::from_millis(1500);

fn main() -> ExitCode {
    let current_version = env!("CARGO_PKG_VERSION");
    // Runs in parallel while wrath does its job; the result is shown at the end.
    let update_check = update::BackgroundCheck::spawn(current_version);

    let args = Cli::parse();
    let exit_code = match run(&args) {
        Ok(result) => {
            println!("{}", ui::render_result(&result));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", ui::render_error(&e));
            ExitCode::FAILURE
        }
    };

    if let Some(latest) = update_check.result(UPDATE_CHECK_BUDGET) {
        eprintln!("{}", update::update_message(current_version, &latest));
    }

    exit_code
}

/// Find the target, show what it is, confirm, then kill it.
fn run(args: &Cli) -> Result<KillResult, WrathError> {
    let mut system = process::create_system();

    let plan = match cli::parse_target(&args.target) {
        Target::Port(port) => {
            let pid = port::resolve_port_to_pid(port)?;
            process::plan_by_pid(&system, pid, port)?
        }
        Target::Name(name) => process::plan_by_name(&system, &name)?,
    };

    let infos = process::describe(&system, &plan);
    println!("{}", ui::render_plan(&plan, &infos));

    let total = plan.pids.len();
    if args.yes {
        println!("{}", ui::render_skipped(total));
    } else {
        let stdin = io::stdin();
        if !stdin.is_terminal() {
            return Err(WrathError::ConfirmationRequired);
        }
        let confirmed = ui::confirm(&mut stdin.lock(), &mut anstream::stdout(), total)
            .map_err(|_| WrathError::Aborted)?;
        if !confirmed {
            return Err(WrathError::Aborted);
        }
    }

    process::execute(&mut system, &plan)
}
