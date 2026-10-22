//! Check a tutorial's format, and step its commands into real shells.

pub mod fence;

use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};

/// Version and commit, without the program name. Clap prints the name itself.
const VERSION_SUFFIX: &str = concat!(
    env!("TUTORIAL_STEPPER_VERSION_RESOLVED"),
    " (commit ",
    env!("TUTORIAL_STEPPER_COMMIT_RESOLVED"),
    ")"
);

/// `tutorial-stepper`, its version, and the commit it was built from.
const IDENTITY: &str = concat!(
    "tutorial-stepper ",
    env!("TUTORIAL_STEPPER_VERSION_RESOLVED"),
    " (commit ",
    env!("TUTORIAL_STEPPER_COMMIT_RESOLVED"),
    ")"
);

/// Run the binary. The value is the process exit code.
pub fn run() -> u8 {
    let argv: Vec<_> = std::env::args_os().collect();
    if argv.len() == 1 {
        print_overview();
        return 0;
    }
    match Cli::try_parse_from(argv) {
        Ok(cli) => match cli.command {
            Command::Check { dir } => check(&dir),
        },
        Err(err) => {
            let code = err.exit_code();
            err.print().ok();
            u8::try_from(code).unwrap_or(1)
        }
    }
}

fn print_overview() {
    Cli::command().print_help().ok();
    println!();
}

#[derive(Parser)]
#[command(
    name = "tutorial-stepper",
    version = VERSION_SUFFIX,
    long_version = VERSION_SUFFIX,
    before_help = IDENTITY,
    about = "Check a tutorial's format, and step its commands into real shells.",
    disable_help_subcommand = true,
    after_help = "Run `tutorial-stepper check --help` for help on that command."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Report format problems in a tutorial directory.
    #[command(
        before_help = IDENTITY,
        after_help = "A sound tutorial prints one confirmation, with the number of pages and the number of commands, and exits 0. Each problem is printed as FILE:LINE: message and the command exits 1. A mistake in the command line exits 2.\n\nExample:\n  tutorial-stepper check examples/one-window"
    )]
    Check {
        /// Tutorial directory: an index file and one Markdown page per step.
        dir: PathBuf,
    },
}

fn check(dir: &std::path::Path) -> u8 {
    eprintln!("tutorial-stepper: cannot check {} yet", dir.display());
    1
}
