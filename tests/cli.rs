//! What a person sees when they run the binary.

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_tutorial-stepper"))
        .args(args)
        .output()
        .expect("the tutorial-stepper binary runs")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("help and version are utf-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("errors are utf-8")
}

#[test]
fn version_names_the_program_the_version_and_the_commit() {
    let output = run(&["--version"]);
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let line = stdout(&output).lines().next().unwrap_or("").to_string();
    assert!(
        line.starts_with("tutorial-stepper 0.1.0 (commit ") && line.ends_with(')'),
        "version line was {line:?}"
    );
}

#[test]
fn no_arguments_prints_a_one_screen_overview() {
    let output = run(&[]);
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let text = stdout(&output);
    let first = text.lines().next().unwrap_or("");
    assert!(
        first.starts_with("tutorial-stepper 0.1.0 (commit "),
        "overview did not open with the identity: {first:?}"
    );
    assert!(text.contains("check"), "overview did not name the check command:\n{text}");
    assert!(text.contains("script"), "overview did not name the script command:\n{text}");
    assert!(
        text.contains("tutorial-stepper check --help"),
        "overview did not say how to get help on a command:\n{text}"
    );
    assert!(
        text.contains("tutorial-stepper script --help"),
        "overview did not say how to get help on the script command:\n{text}"
    );
    assert!(text.contains("play"), "overview did not name the play command:\n{text}");
    assert!(
        text.contains("tutorial-stepper play --help"),
        "overview did not say how to get help on the play command:\n{text}"
    );
    let lines = text.lines().count();
    assert!(lines <= 24, "overview is {lines} lines, which is more than one screen:\n{text}");
}

#[test]
fn check_help_describes_that_command_only() {
    let output = run(&["check", "--help"]);
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("<DIR>"), "check help did not name the directory:\n{text}");
    assert!(
        text.contains("FILE:LINE: message"),
        "check help did not say how problems are printed:\n{text}"
    );
    assert!(text.contains("tutorial-stepper check"), "check help had no example:\n{text}");
    assert!(!text.contains("Commands:"), "check help listed other commands:\n{text}");
    let lines = text.lines().count();
    assert!(lines <= 40, "check help is {lines} lines:\n{text}");
}

#[test]
fn a_mistyped_command_is_a_short_error() {
    let output = run(&["chek"]);
    assert_eq!(output.status.code(), Some(2), "stderr:\n{}", stderr(&output));
    let text = stderr(&output);
    assert!(text.contains("chek"), "the error did not repeat the unknown command:\n{text}");
    let lines = text.lines().count();
    assert!(lines <= 8, "error is {lines} lines:\n{text}");
}

#[test]
fn script_help_describes_that_command_only() {
    let output = run(&["script", "--help"]);
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("<DIR>"), "script help did not name the directory:\n{text}");
    assert!(
        text.contains("mode=show"),
        "script help did not say that a shown block is left out:\n{text}"
    );
    assert!(
        text.contains("observation"),
        "script help did not say that an observation is included:\n{text}"
    );
    assert!(
        text.contains("FILE:LINE: message"),
        "script help did not say how problems are printed:\n{text}"
    );
    assert!(text.contains("tutorial-stepper script"), "script help had no example:\n{text}");
    assert!(!text.contains("Commands:"), "script help listed other commands:\n{text}");
    let lines = text.lines().count();
    assert!(lines <= 40, "script help is {lines} lines:\n{text}");
}

#[test]
fn play_help_describes_that_command_only() {
    let output = run(&["play", "--help"]);
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("--auto"), "play help did not name --auto:\n{text}");
    assert!(text.contains("--transcript"), "play help did not name --transcript:\n{text}");
    assert!(text.contains("<DIR>"), "play help did not name the directory:\n{text}");
    assert!(text.contains("<FILE>"), "play help did not name the transcript file:\n{text}");
    assert!(
        text.contains("FILE:LINE: message"),
        "play help did not say how problems are printed:\n{text}"
    );
    assert!(text.contains("tutorial-stepper play --auto"), "play help had no example:\n{text}");
    assert!(!text.contains("Commands:"), "play help listed other commands:\n{text}");
    let lines = text.lines().count();
    assert!(lines <= 40, "play help is {lines} lines:\n{text}");
}

#[test]
fn play_without_auto_is_a_short_error() {
    let output = run(&["play", "examples/one-window"]);
    assert_eq!(output.status.code(), Some(2), "stderr:\n{}", stderr(&output));
    let text = stderr(&output);
    assert!(text.contains("--auto"), "the error did not name --auto:\n{text}");
    let lines = text.lines().count();
    assert!(lines <= 8, "error is {lines} lines:\n{text}");
}

#[test]
fn script_without_a_directory_is_a_short_error() {
    let output = run(&["script"]);
    assert_eq!(output.status.code(), Some(2), "stderr:\n{}", stderr(&output));
    let text = stderr(&output);
    assert!(
        text.contains("DIR") || text.contains("dir"),
        "the error did not name the missing directory:\n{text}"
    );
    let lines = text.lines().count();
    assert!(lines <= 8, "error is {lines} lines:\n{text}");
}

#[test]
fn check_without_a_directory_is_a_short_error() {
    let output = run(&["check"]);
    assert_eq!(output.status.code(), Some(2), "stderr:\n{}", stderr(&output));
    let text = stderr(&output);
    assert!(
        text.contains("DIR") || text.contains("dir"),
        "the error did not name the missing directory:\n{text}"
    );
    let lines = text.lines().count();
    assert!(lines <= 8, "error is {lines} lines:\n{text}");
}
