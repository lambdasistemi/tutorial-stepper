//! Check a tutorial's format, print its commands as one script, and step them into real shells.

pub mod fence;

mod face;
mod index;
mod play;
mod render;
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
            Command::Script { dir, path } => script(&dir, path.as_deref()),
            Command::Play { auto, page, dir, transcript, run, run_setup, path } => dispatch_play(
                auto,
                page,
                &dir,
                transcript.as_deref(),
                run.as_deref(),
                run_setup,
                path.as_deref(),
            ),
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
        after_help = "Each runnable command is preceded by a comment naming its page and its window. A block marked mode=show is left out. An observation is included where it appears. Without --path, every branch is printed. --path NAME prints the blocks of that tested path and leaves the other branches out. A sound tutorial prints the script and exits 0. Each problem is printed as FILE:LINE: message and the command exits 1, with nothing on standard output. A mistake in the command line exits 2.\n\nExample:\n  tutorial-stepper script examples/one-window\n  tutorial-stepper script examples/if --path external"
    )]
    Script {
        /// Tested path. Omit it to print every branch.
        #[arg(long, value_name = "NAME")]
        path: Option<String>,
        /// Tutorial directory: an index file and one Markdown page per step.
        dir: PathBuf,
    },
    /// Play a tutorial page by page in the terminal.
    #[command(
        before_help = IDENTITY,
        after_help = "The page sits beside the window of the next command. Enter runs it, e places it at the prompt to edit first, s skips it, c sends Ctrl-C, b goes back, n and p turn the page, and q closes the page and keeps the windows. A later play returns to the same place. The folder is $HOME/<name>-run unless --run names another.\n\nWith --auto each command runs in the shell the index declares, bash when the index does not say. An observation runs where it appears and again at the end of the path. --path NAME plays that tested path. Without --path, every tested path plays from its own run folder, and the command exits 1 if any path fails. The values are written to .env in the run folder, and every window loads the file before its first command. A different status is printed as FILE:LINE: message and the command exits 1. A mistake in the command line exits 2. --transcript and --run-setup are used with --auto.\n\nExample:\n  tutorial-stepper play examples/two-windows\n  tutorial-stepper play --auto examples/one-window --transcript transcript.toml"
    )]
    Play {
        /// Play every command without stopping for a person. Needs --transcript.
        #[arg(long)]
        auto: bool,
        /// Draw one page inside the stepper's own tmux server.
        #[arg(long, hide = true)]
        page: bool,
        /// File that receives one entry per command and per observation run.
        #[arg(long, value_name = "FILE")]
        transcript: Option<PathBuf>,
        /// Folder for the windows and the position. Defaults to $HOME/<name>-run.
        #[arg(long, value_name = "DIR")]
        run: Option<PathBuf>,
        /// With --auto, run the declared set-up script before the first command.
        #[arg(long)]
        run_setup: bool,
        /// With --auto, play this tested path. Omit it to play every path.
        #[arg(long, value_name = "NAME")]
        path: Option<String>,
        /// Tutorial directory: an index file and one Markdown page per step.
        dir: PathBuf,
    },
}

fn check(dir: &std::path::Path) -> u8 {
    tutorial::check(dir)
}

fn script(dir: &std::path::Path, path: Option<&str>) -> u8 {
    tutorial::script(dir, path)
}

fn dispatch_play(
    auto: bool,
    page: bool,
    dir: &std::path::Path,
    transcript: Option<&std::path::Path>,
    run: Option<&std::path::Path>,
    run_setup: bool,
    path: Option<&str>,
) -> u8 {
    if page {
        let Some(run) = run else {
            eprintln!("the page needs --run DIR");
            eprintln!("Run `tutorial-stepper play --help` for help.");
            return 2;
        };
        return face::page(dir, run);
    }
    if auto {
        let Some(transcript) = transcript else {
            eprintln!("play --auto needs --transcript");
            eprintln!("Run `tutorial-stepper play --help` for help.");
            return 2;
        };
        return tutorial::play(dir, transcript, run, run_setup, path);
    }
    if transcript.is_some() {
        return used_with_auto("--transcript");
    }
    if path.is_some() {
        return used_with_auto("--path");
    }
    if run_setup {
        return used_with_auto("--run-setup");
    }
    face::play(dir, run)
}

fn used_with_auto(option: &str) -> u8 {
    eprintln!("{option} is used with --auto");
    eprintln!("Run `tutorial-stepper play --help` for help.");
    2
}
