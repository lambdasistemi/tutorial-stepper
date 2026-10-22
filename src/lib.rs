//! Check a tutorial's format, print its commands as one script, and step them into real shells.

pub mod fence;

mod index;
mod play;
mod tutorial;

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
            Command::Script { dir } => script(&dir),
            Command::Play { auto: _, dir, transcript, run, run_setup } => {
                play(&dir, &transcript, run.as_deref(), run_setup)
            }
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
    about = "Check a tutorial's format, print its commands as one script, and step them into real shells.",
    disable_help_subcommand = true,
    after_help = "Run `tutorial-stepper check --help` for help on that command.\nRun `tutorial-stepper script --help` for help on that command.\nRun `tutorial-stepper play --help` for help on that command."
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
    /// Print the runnable commands as one bash script.
    #[command(
        before_help = IDENTITY,
        after_help = "Each runnable command is preceded by a comment naming its page and its window. A block marked mode=show is left out. An observation is included where it appears. A sound tutorial prints the script and exits 0. Each problem is printed as FILE:LINE: message and the command exits 1, with nothing on standard output. A mistake in the command line exits 2.\n\nExample:\n  tutorial-stepper script examples/one-window"
    )]
    Script {
        /// Tutorial directory: an index file and one Markdown page per step.
        dir: PathBuf,
    },
    /// Play a tutorial in the bash shells its windows declare.
    #[command(
        before_help = IDENTITY,
        after_help = "Each command runs in the bash shell of its window. The transcript records the window, the command, its output, its status, and its start and end time. A sound tutorial prints how many commands played and exits 0. A command that exits with a different status is printed as FILE:LINE: message and the command exits 1. A mistake in the command line exits 2.\n\nExample:\n  tutorial-stepper play --auto examples/one-window --transcript transcript.toml"
    )]
    Play {
        /// Play the tutorial without stopping for a person.
        #[arg(long, required = true)]
        auto: bool,
        /// File that receives one entry per command that ran.
        #[arg(long, value_name = "FILE", required = true)]
        transcript: PathBuf,
        /// Directory that keeps each window's folder. Removed at the end when absent.
        #[arg(long, value_name = "DIR")]
        run: Option<PathBuf>,
        /// Run the declared set-up script before the first page.
        #[arg(long)]
        run_setup: bool,
        /// Tutorial directory: an index file and one Markdown page per step.
        dir: PathBuf,
    },
}

fn check(dir: &std::path::Path) -> u8 {
    tutorial::check(dir)
}

fn script(dir: &std::path::Path) -> u8 {
    tutorial::script(dir)
}

fn play(
    dir: &std::path::Path,
    transcript: &std::path::Path,
    run: Option<&std::path::Path>,
    run_setup: bool,
) -> u8 {
    tutorial::play(dir, transcript, run, run_setup)
}
