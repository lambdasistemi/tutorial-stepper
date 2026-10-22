//! `tutorial-stepper check` on the example tutorials.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_check(relative: &str) -> std::process::Output {
    let dir = manifest_dir().join(relative);
    Command::new(env!("CARGO_BIN_EXE_tutorial-stepper"))
        .arg("check")
        .arg(&dir)
        .output()
        .expect("the tutorial-stepper binary runs")
}

fn directories(relative: &str) -> Vec<String> {
    let mut names = Vec::new();
    for entry in fs::read_dir(manifest_dir().join(relative)).expect("the example directory exists")
    {
        let entry = entry.expect("an example directory entry");
        if entry.file_type().expect("a file type").is_dir() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    names
}

#[test]
fn valid_examples_pass_with_the_page_and_command_counts() {
    let expected = [
        ("one-window", "1 page, 1 command\n"),
        ("status", "1 page, 1 command\n"),
        ("mode", "1 page, 1 command\n"),
        ("if", "1 page, 2 commands\n"),
    ];
    let mut seen: Vec<&str> = expected.iter().map(|(name, _)| *name).collect();
    seen.sort_unstable();
    let mut found = directories("examples");
    found.retain(|name| name != "invalid");
    assert_eq!(found, seen, "every valid example is asserted");

    for (name, stdout) in expected {
        let output = run_check(&format!("examples/{name}"));
        let got_out = String::from_utf8_lossy(&output.stdout);
        let got_err = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "examples/{name} exited {:?}\nstdout:\n{got_out}\nstderr:\n{got_err}",
            output.status.code()
        );
        assert_eq!(got_out.as_ref(), stdout, "examples/{name}");
        assert!(got_err.is_empty(), "examples/{name} stderr:\n{got_err}");
    }
}

#[test]
fn invalid_examples_name_the_file_and_the_line() {
    let expected: &[(&str, usize, &[&str])] = &[
        ("unknown-attribute", 1, &["unknown attribute", "colour"]),
        ("malformed-attribute", 1, &["JSON"]),
        ("undeclared-window", 1, &["carol", "not declared"]),
        ("unlisted-page", 1, &["notes.md", "not listed"]),
        ("missing-page", 1, &["missing.md", "missing"]),
        ("undeclared-setting", 1, &["THEME", "not declared"]),
        ("unknown-key", 1, &["unknown key", "theme"]),
        ("bad-default", 1, &["nope", "values"]),
        ("path-misses-setting", 1, &["does not set", "SIGNING"]),
        ("path-unknown-setting", 1, &["THEME", "not a setting"]),
        ("path-bad-value", 1, &["nope", "not allowed"]),
        ("duplicate-window", 1, &["bob", "more than once"]),
        ("duplicate-setting", 1, &["SIGNING", "more than once"]),
        ("duplicate-path", 1, &["external", "more than once"]),
        ("bad-needs", 1, &["singular > 1", ">="]),
        ("bad-shell", 1, &["bash or zsh"]),
        ("bad-setup", 1, &["relative path"]),
        ("no-window", 1, &["at least one window"]),
        ("empty-pages", 1, &["at least one page"]),
        ("bad-if-value", 1, &["nope", "not allowed"]),
        ("bad-folder", 1, &["relative path"]),
        ("two-problems", 2, &["colour", "carol"]),
        ("missing-index", 1, &["tutorial.toml is missing"]),
    ];
    let mut seen: Vec<&str> = expected.iter().map(|(name, _, _)| *name).collect();
    seen.sort_unstable();
    assert_eq!(directories("examples/invalid"), seen, "every invalid example is asserted");

    for (name, line_count, needles) in expected {
        let relative = format!("examples/invalid/{name}");
        let output = run_check(&relative);
        let got_out = String::from_utf8_lossy(&output.stdout);
        let got_err = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "{relative} exited 0\nstdout:\n{got_out}\nstderr:\n{got_err}"
        );
        assert!(got_out.is_empty(), "{relative} printed a confirmation:\n{got_out}");
        let lines: Vec<&str> = got_err.lines().collect();
        assert_eq!(lines.len(), *line_count, "{relative} stderr:\n{got_err}");
        let prefix = manifest_dir().join(&relative);
        for line in &lines {
            assert!(
                line.starts_with(&prefix.display().to_string()),
                "{relative} line does not start with the tutorial path: {line}"
            );
            let rest = &line[prefix.display().to_string().len()..];
            let (_, message) = rest.split_once(": ").unwrap_or(("", ""));
            let (location, _) = rest.split_once(": ").unwrap_or(("", ""));
            assert!(
                location.starts_with(':') || location.starts_with('/'),
                "{relative} location: {line}"
            );
            let after_path = location.trim_start_matches('/');
            let line_number = after_path.rsplit_once(':').map(|(_, number)| number).unwrap_or("");
            assert!(
                line_number.chars().all(|c| c.is_ascii_digit()) && !line_number.is_empty(),
                "{line}"
            );
            assert!(!message.is_empty(), "{line}");
        }
        for needle in *needles {
            assert!(got_err.contains(needle), "{relative} stderr missing {needle:?}:\n{got_err}");
        }
        if *name == "missing-index" {
            let index = prefix.join("tutorial.toml");
            assert_eq!(
                got_err.as_ref(),
                format!("{}:1: tutorial.toml is missing\n", index.display()),
                "{relative} names the missing index"
            );
        }
    }
}
