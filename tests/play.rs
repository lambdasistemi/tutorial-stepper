//! `tutorial-stepper play --auto` on a one-window bash tutorial.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread;
use std::time::Duration;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("the tutorial-stepper output is utf-8")
}

fn play(dir: &Path, transcript: &Path) -> (Output, u32) {
    play_with(dir, transcript, &[])
}

fn play_with(dir: &Path, transcript: &Path, args: &[String]) -> (Output, u32) {
    play_prefixed(dir, transcript, args, None)
}

fn play_prefixed(
    dir: &Path,
    transcript: &Path,
    args: &[String],
    path_prefix: Option<&Path>,
) -> (Output, u32) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tutorial-stepper"));
    command.arg("play").arg("--auto").arg("--transcript").arg(transcript);
    command.args(args);
    command.arg(dir);
    if let Some(prefix) = path_prefix {
        let inherited = std::env::var_os("PATH").expect("PATH is set");
        let mut entries = vec![prefix.to_path_buf()];
        entries.extend(std::env::split_paths(&inherited));
        command.env("PATH", std::env::join_paths(entries).expect("the prefixed PATH joins"));
    }
    let child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the tutorial-stepper binary runs");
    let pid = child.id();
    let output = child.wait_with_output().expect("the play command finishes");
    (output, pid)
}

fn server_is_gone(pid: u32) {
    let needle = format!("tutorial-stepper-{pid}-");
    for _ in 0..40 {
        let ps = Command::new("ps").args(["-eo", "args"]).output().expect("ps runs");
        if !text(&ps.stdout).contains(&needle) {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let ps = Command::new("ps").args(["-eo", "args"]).output().expect("ps runs");
    panic!("a tmux server named {needle} is still running:\n{}", text(&ps.stdout));
}

fn transcript_commands(path: &Path) -> toml_edit::ArrayOfTables {
    let body = fs::read_to_string(path).expect("the transcript can be read");
    if body.trim().is_empty() {
        return toml_edit::ArrayOfTables::new();
    }
    let document: toml_edit::DocumentMut = body.parse().expect("the transcript is TOML");
    document
        .get("command")
        .and_then(toml_edit::Item::as_array_of_tables)
        .expect("the transcript is a list of commands")
        .clone()
}

fn string_field(table: &toml_edit::Table, key: &str) -> String {
    table
        .get(key)
        .and_then(toml_edit::Item::as_str)
        .unwrap_or_else(|| panic!("{key} is a string, table was {table:?}"))
        .to_string()
}

fn status_field(table: &toml_edit::Table) -> i64 {
    table
        .get("status")
        .and_then(toml_edit::Item::as_integer)
        .unwrap_or_else(|| panic!("status is an integer, table was {table:?}"))
}

fn block_field(table: &toml_edit::Table) -> i64 {
    table
        .get("block")
        .and_then(toml_edit::Item::as_integer)
        .unwrap_or_else(|| panic!("block is an integer, table was {table:?}"))
}

fn epoch_seconds(stamp: &str) -> i64 {
    let output = Command::new("date")
        .args(["-u", "-d", stamp, "+%s"])
        .output()
        .expect("date reads a timestamp");
    assert!(output.status.success(), "date rejected {stamp}: {}", text(&output.stderr));
    text(&output.stdout).trim().parse().expect("date prints seconds")
}

fn write_tutorial(root: &Path, index: &str, page: &str) {
    fs::create_dir_all(root).expect("the temporary tutorial directory");
    fs::write(root.join("tutorial.toml"), index).expect("the index is written");
    fs::write(root.join("welcome.md"), page).expect("the page is written");
}

fn temp_tutorial(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    root
}

#[test]
fn one_window_records_the_command_its_output_and_its_status() {
    let dir = manifest_dir().join("examples/one-window");
    let transcript = temp_tutorial("one-window-transcript").join("transcript.toml");
    fs::create_dir_all(transcript.parent().expect("a parent")).expect("the transcript directory");
    let (output, pid) = play(&dir, &transcript);
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        text(&output.stdout),
        text(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "stderr:\n{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "1 command played\n");
    let commands = transcript_commands(&transcript);
    assert_eq!(commands.len(), 1, "{commands:?}");
    let command = commands.get(0).expect("the one command");
    assert_eq!(string_field(command, "page"), "welcome.md");
    assert_eq!(block_field(command), 1);
    assert_eq!(string_field(command, "command"), "echo hi\n");
    assert_eq!(status_field(command), 0);
    assert_eq!(string_field(command, "output"), "hi\n");
    assert_eq!(string_field(command, "window"), "bob");
    let start = string_field(command, "start");
    let end = string_field(command, "end");
    assert!(start <= end, "start {start} is after end {end}");
    assert_eq!(start.len(), 30, "{start}");
    assert!(start.ends_with('Z') && end.ends_with('Z'));
    server_is_gone(pid);
}

#[test]
fn two_windows_run_in_their_own_folders() {
    let dir = manifest_dir().join("examples/two-windows");
    let transcript = temp_tutorial("two-windows-transcript").join("transcript.toml");
    fs::create_dir_all(transcript.parent().expect("a parent")).expect("the transcript directory");
    let (output, pid) = play(&dir, &transcript);
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        text(&output.stdout),
        text(&output.stderr)
    );
    assert_eq!(text(&output.stdout), "3 commands played\n");
    let commands = transcript_commands(&transcript);
    assert_eq!(commands.len(), 3, "{commands:?}");
    assert_eq!(string_field(commands.get(0).expect("alice"), "window"), "alice");
    assert!(
        string_field(commands.get(0).expect("alice"), "output").trim_end().ends_with("/alice"),
        "alice ran in {}",
        string_field(commands.get(0).expect("alice"), "output")
    );
    assert_eq!(string_field(commands.get(1).expect("bob"), "window"), "bob");
    assert!(
        string_field(commands.get(1).expect("bob"), "output").trim_end().ends_with("/bob"),
        "bob ran in {}",
        string_field(commands.get(1).expect("bob"), "output")
    );
    assert_eq!(string_field(commands.get(2).expect("continued"), "window"), "bob");
    assert!(
        string_field(commands.get(2).expect("continued"), "output").trim_end().ends_with("/bob"),
        "the unnamed command ran in {}",
        string_field(commands.get(2).expect("continued"), "output")
    );
    server_is_gone(pid);
}

#[test]
fn a_directory_change_stays_in_that_window() {
    let root = temp_tutorial("window-cd");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"alice\"
folder = \"alice\"

[[window]]
name = \"bob\"
folder = \"bob\"
",
        "\
```bash { window=alice }
cd /tmp
```

```bash { window=bob }
pwd
```

```bash { window=alice }
pwd
```

```bash { }
echo stayed
```
",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play(&root, &transcript);
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    let commands = transcript_commands(&transcript);
    assert_eq!(commands.len(), 4, "{commands:?}");
    assert!(
        string_field(commands.get(1).expect("bob"), "output").trim_end().ends_with("/bob"),
        "bob ran in {}",
        string_field(commands.get(1).expect("bob"), "output")
    );
    assert_eq!(string_field(commands.get(2).expect("alice again"), "output").trim(), "/tmp");
    assert_eq!(string_field(commands.get(3).expect("unnamed"), "window"), "alice");
    assert_eq!(string_field(commands.get(3).expect("unnamed"), "output"), "stayed\n");
    server_is_gone(pid);
}

#[test]
fn a_shown_block_does_not_change_the_window() {
    let root = temp_tutorial("window-show");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"alice\"
folder = \"alice\"

[[window]]
name = \"bob\"
folder = \"bob\"
",
        "\
```bash { window=alice }
echo from-alice
```

```bash { window=bob mode=show }
echo shown
```

```bash { }
echo still-alice
```
",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play(&root, &transcript);
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    let commands = transcript_commands(&transcript);
    assert_eq!(commands.len(), 2, "{commands:?}");
    assert_eq!(string_field(commands.get(0).expect("alice"), "window"), "alice");
    assert_eq!(string_field(commands.get(1).expect("still"), "window"), "alice");
    assert_eq!(string_field(commands.get(1).expect("still"), "output"), "still-alice\n");
    let body = fs::read_to_string(&transcript).expect("the transcript");
    assert!(!body.contains("shown"), "{body}");
    server_is_gone(pid);
}

#[test]
fn a_declared_status_that_does_not_match_names_the_command() {
    let dir = manifest_dir().join("examples/status");
    let transcript = temp_tutorial("status-transcript").join("transcript.toml");
    fs::create_dir_all(transcript.parent().expect("a parent")).expect("the transcript directory");
    let (output, pid) = play(&dir, &transcript);
    assert_eq!(output.status.code(), Some(1), "stdout:\n{}", text(&output.stdout));
    assert!(output.stdout.is_empty(), "stdout:\n{}", text(&output.stdout));
    let err = text(&output.stderr);
    let page = dir.join("build.md");
    assert!(err.starts_with(&format!("{}:1:", page.display())), "{err}");
    assert!(err.contains("echo not yet"), "{err}");
    assert!(err.contains("exited 0"), "{err}");
    assert!(err.contains("expected 19"), "{err}");
    let commands = transcript_commands(&transcript);
    assert_eq!(commands.len(), 1, "{commands:?}");
    let command = commands.get(0).expect("the failing command");
    assert_eq!(string_field(command, "command"), "echo not yet\n");
    assert_eq!(status_field(command), 0);
    assert_eq!(string_field(command, "output"), "not yet\n");
    server_is_gone(pid);
}

#[test]
fn a_declared_status_of_19_passes_when_the_command_exits_19() {
    let dir = manifest_dir().join("examples/declared-status");
    let transcript = temp_tutorial("declared-status-transcript").join("transcript.toml");
    fs::create_dir_all(transcript.parent().expect("a parent")).expect("the transcript directory");
    let (output, pid) = play(&dir, &transcript);
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "1 command played\n");
    let commands = transcript_commands(&transcript);
    assert_eq!(commands.len(), 1);
    let command = commands.get(0).expect("the command");
    assert_eq!(string_field(command, "command"), "(exit 19)\n");
    assert_eq!(status_field(command), 19);
    assert_eq!(string_field(command, "output"), "");
    server_is_gone(pid);
}

#[test]
fn a_continued_command_is_one_entry() {
    let dir = manifest_dir().join("examples/continued");
    let transcript = temp_tutorial("continued-transcript").join("transcript.toml");
    fs::create_dir_all(transcript.parent().expect("a parent")).expect("the transcript directory");
    let (output, pid) = play(&dir, &transcript);
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    let commands = transcript_commands(&transcript);
    assert_eq!(commands.len(), 1, "{commands:?}");
    let command = commands.get(0).expect("the continued command");
    assert_eq!(string_field(command, "command"), "echo hello \\\nworld\n");
    assert_eq!(status_field(command), 0);
    assert_eq!(string_field(command, "output"), "hello world\n");
    server_is_gone(pid);
}

#[test]
fn a_shown_block_and_an_observation_are_not_played() {
    let dir = manifest_dir().join("examples/mode");
    let transcript = temp_tutorial("mode-transcript").join("transcript.toml");
    fs::create_dir_all(transcript.parent().expect("a parent")).expect("the transcript directory");
    let (output, pid) = play(&dir, &transcript);
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "1 command played\n");
    let commands = transcript_commands(&transcript);
    assert_eq!(commands.len(), 1, "{commands:?}");
    let command = commands.get(0).expect("the command");
    assert_eq!(string_field(command, "command"), "echo run\n");
    assert_eq!(string_field(command, "output"), "run\n");
    let body = fs::read_to_string(&transcript).expect("the transcript");
    assert!(!body.contains("observed"), "{body}");
    assert!(!body.contains("shown"), "{body}");
    server_is_gone(pid);
}

#[test]
fn a_command_that_sleeps_120_seconds_passes() {
    let dir = manifest_dir().join("examples/slow");
    let transcript = temp_tutorial("slow-transcript").join("transcript.toml");
    fs::create_dir_all(transcript.parent().expect("a parent")).expect("the transcript directory");
    let (output, pid) = play(&dir, &transcript);
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    let commands = transcript_commands(&transcript);
    assert_eq!(commands.len(), 1, "{commands:?}");
    let command = commands.get(0).expect("the sleep");
    assert_eq!(status_field(command), 0);
    let elapsed = epoch_seconds(&string_field(command, "end"))
        - epoch_seconds(&string_field(command, "start"));
    assert!(elapsed >= 120, "the run ended after {elapsed} seconds");
    server_is_gone(pid);
}

#[test]
fn a_broken_tutorial_prints_the_same_problems_and_writes_no_transcript() {
    let dir = manifest_dir().join("examples/invalid/missing-index");
    let transcript = temp_tutorial("missing-index-transcript").join("transcript.toml");
    let checked = Command::new(env!("CARGO_BIN_EXE_tutorial-stepper"))
        .arg("check")
        .arg(&dir)
        .output()
        .expect("check runs");
    let (output, pid) = play(&dir, &transcript);
    assert_eq!(output.status.code(), checked.status.code());
    assert!(output.stdout.is_empty(), "{}", text(&output.stdout));
    assert_eq!(text(&output.stderr), text(&checked.stderr));
    assert!(!transcript.exists(), "a refused tutorial wrote a transcript");
    server_is_gone(pid);
}

#[test]
fn zsh_a_setup_script_and_a_setting_are_refused() {
    let zsh = manifest_dir().join("examples/if");
    let zsh_transcript = temp_tutorial("zsh-transcript").join("transcript.toml");
    let (output, pid) = play(&zsh, &zsh_transcript);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("bash") && err.contains("zsh"), "{err}");
    assert!(!zsh_transcript.exists());
    server_is_gone(pid);

    let setup = temp_tutorial("setup-script");
    write_tutorial(
        &setup,
        "\
setup = \"setup.sh\"
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    let setup_transcript = setup.join("transcript.toml");
    let (output, pid) = play(&setup, &setup_transcript);
    assert_eq!(output.status.code(), Some(1));
    let err = text(&output.stderr);
    assert!(err.contains("setup.sh"), "{err}");
    assert!(!setup_transcript.exists());
    server_is_gone(pid);

    let setting = temp_tutorial("setting-branch");
    write_tutorial(
        &setting,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"

[[setting]]
name = \"SIGNING\"
default = \"external\"
values = [\"external\", \"internal\"]
",
        "```bash { window=bob if=SIGNING:external }\necho external\n```\n",
    );
    let setting_transcript = setting.join("transcript.toml");
    let (output, pid) = play(&setting, &setting_transcript);
    assert_eq!(output.status.code(), Some(1));
    let err = text(&output.stderr);
    assert!(err.contains("setting"), "{err}");
    assert!(!setting_transcript.exists());
    server_is_gone(pid);
}

#[test]
fn run_keeps_the_window_folder_and_a_second_play_is_refused() {
    let root = temp_tutorial("kept-window");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
",
        "```bash { window=bob }\npwd\n```\n",
    );
    let run = temp_tutorial("kept-run");
    fs::create_dir_all(&run).expect("an empty run directory");
    let transcript = root.join("transcript.toml");
    let (output, pid) =
        play_with(&root, &transcript, &["--run".to_string(), run.display().to_string()]);
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    let commands = transcript_commands(&transcript);
    let got = string_field(commands.get(0).expect("pwd"), "output");
    let folder = run.join("bob");
    assert_eq!(got.trim(), folder.to_string_lossy());
    assert!(folder.is_dir(), "the window folder was removed");
    let mut names = fs::read_dir(&run)
        .expect("the run directory")
        .map(|entry| entry.expect("a run entry").file_name())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(names, vec![std::ffi::OsString::from("bob")]);
    server_is_gone(pid);

    let again = root.join("again.toml");
    let marker = fs::read_dir(&run).expect("the run directory").count();
    let (output, pid) = play_with(&root, &again, &["--run".to_string(), run.display().to_string()]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("not empty"), "{err}");
    assert!(err.contains(&run.display().to_string()), "{err}");
    assert!(!again.exists(), "a refused run wrote a transcript");
    assert_eq!(fs::read_dir(&run).expect("the run directory").count(), marker);
    server_is_gone(pid);
}

#[test]
fn a_run_without_run_removes_the_window_folder() {
    let root = temp_tutorial("removed-window");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
",
        "```bash { window=bob }\npwd\n```\n",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play(&root, &transcript);
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    let commands = transcript_commands(&transcript);
    let got = string_field(commands.get(0).expect("pwd"), "output");
    let folder = PathBuf::from(got.trim());
    assert!(folder.ends_with("bob"), "{got}");
    assert!(!folder.exists(), "the temporary window folder was kept at {got}");
    server_is_gone(pid);
}

#[test]
fn a_run_path_that_is_a_file_is_refused() {
    let root = temp_tutorial("run-file-tutorial");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    let run = temp_tutorial("run-file");
    fs::write(&run, "keep").expect("a file");
    let transcript = root.join("transcript.toml");
    let (output, pid) =
        play_with(&root, &transcript, &["--run".to_string(), run.display().to_string()]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("not a directory"), "{err}");
    assert_eq!(fs::read_to_string(&run).expect("the file"), "keep");
    assert!(!transcript.exists());
    server_is_gone(pid);
}

#[test]
fn a_setup_script_without_run_setup_does_not_run() {
    let root = temp_tutorial("setup-not-run");
    write_tutorial(
        &root,
        "\
setup = \"setup.sh\"
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    fs::write(root.join("setup.sh"), "printf ran > marker\n").expect("the set-up script");
    let transcript = root.join("transcript.toml");
    let (output, pid) = play(&root, &transcript);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("setup.sh"), "{err}");
    assert!(!transcript.exists(), "a refused run wrote a transcript");
    assert!(!root.join("marker").exists(), "the set-up script ran");
    server_is_gone(pid);
}

#[test]
fn run_setup_runs_the_script_before_any_command() {
    let root = temp_tutorial("setup-runs");
    write_tutorial(
        &root,
        "\
setup = \"setup.sh\"
pages = [\"welcome.md\"]

[[window]]
name = \"alice\"
folder = \"alice\"

[[window]]
name = \"bob\"
folder = \"bob\"
",
        "\
```bash { window=alice }
cat who
cat where
```

```bash { window=bob }
cat marker
```
",
    );
    fs::write(
        root.join("setup.sh"),
        "\
id -u > alice/who
pwd > alice/where
printf 'from-setup\\n' > bob/marker
echo noise
",
    )
    .expect("the set-up script");
    let run = temp_tutorial("setup-runs-dir");
    fs::create_dir_all(&run).expect("an empty run directory");
    let transcript = root.join("transcript.toml");
    let (output, pid) = play_with(
        &root,
        &transcript,
        &["--run".to_string(), run.display().to_string(), "--run-setup".to_string()],
    );
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        text(&output.stdout),
        text(&output.stderr)
    );
    assert_eq!(text(&output.stdout), "2 commands played\n");
    assert!(output.stderr.is_empty(), "stderr:\n{}", text(&output.stderr));
    let commands = transcript_commands(&transcript);
    let alice = string_field(commands.get(0).expect("alice"), "output");
    let uid = Command::new("id").arg("-u").output().expect("id runs");
    assert!(uid.status.success(), "{}", text(&uid.stderr));
    let uid = text(&uid.stdout);
    assert!(alice.contains(uid.trim()), "alice output {alice:?} does not name user {uid:?}");
    assert!(
        alice.contains(&run.display().to_string()),
        "the script's working directory was not the run folder: {alice:?}"
    );
    assert_eq!(string_field(commands.get(0).expect("alice"), "window"), "alice");
    let bob = string_field(commands.get(1).expect("bob"), "output");
    assert_eq!(bob, "from-setup\n");
    assert_eq!(string_field(commands.get(1).expect("bob"), "window"), "bob");
    server_is_gone(pid);
}

#[test]
fn a_setup_script_that_fails_stops_before_any_command() {
    let root = temp_tutorial("setup-fails");
    write_tutorial(
        &root,
        "\
setup = \"setup.sh\"
pages = [\"welcome.md\"]

[[window]]
name = \"alice\"
folder = \"alice\"
",
        "```bash { window=alice }\nprintf played > played\n```\n",
    );
    fs::write(root.join("setup.sh"), "exit 7\n").expect("the set-up script");
    let run = temp_tutorial("setup-fails-dir");
    fs::create_dir_all(&run).expect("an empty run directory");
    let transcript = root.join("transcript.toml");
    let (output, pid) = play_with(
        &root,
        &transcript,
        &["--run".to_string(), run.display().to_string(), "--run-setup".to_string()],
    );
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("setup.sh"), "{err}");
    assert!(err.contains("exited 7"), "{err}");
    assert!(!transcript.exists(), "a failed set-up script wrote a transcript");
    assert!(!run.join("alice").join("played").exists(), "a page command ran");
    server_is_gone(pid);
}

#[test]
fn a_missing_setup_script_is_named() {
    let root = temp_tutorial("setup-missing");
    write_tutorial(
        &root,
        "\
setup = \"setup.sh\"
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play_with(&root, &transcript, &["--run-setup".to_string()]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("setup.sh"), "{err}");
    assert!(err.contains("not a file"), "{err}");
    assert!(!transcript.exists());
    server_is_gone(pid);
}

#[test]
fn examples_setup_prepares_both_windows() {
    let root = manifest_dir().join("examples/setup");
    let run = temp_tutorial("examples-setup-run");
    fs::create_dir_all(&run).expect("an empty run directory");
    let transcript = temp_tutorial("examples-setup-transcript").join("transcript.toml");
    fs::create_dir_all(transcript.parent().expect("a parent")).expect("the transcript directory");
    let (output, pid) = play_with(
        &root,
        &transcript,
        &["--run".to_string(), run.display().to_string(), "--run-setup".to_string()],
    );
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        text(&output.stdout),
        text(&output.stderr)
    );
    assert_eq!(text(&output.stdout), "2 commands played\n");
    let commands = transcript_commands(&transcript);
    let alice = string_field(commands.get(0).expect("alice"), "output");
    let bob = string_field(commands.get(1).expect("bob"), "output");
    assert!(alice.contains(&run.join("alice").display().to_string()), "{alice}");
    assert!(alice.contains("prepared\n"), "{alice}");
    assert_eq!(string_field(commands.get(0).expect("alice"), "window"), "alice");
    assert!(bob.contains(&run.join("bob").display().to_string()), "{bob}");
    assert!(bob.contains("prepared\n"), "{bob}");
    assert_eq!(string_field(commands.get(1).expect("bob"), "window"), "bob");
    assert!(run.join("alice").join("ready").is_file());
    assert!(run.join("bob").join("ready").is_file());
    server_is_gone(pid);
}

#[test]
fn run_setup_without_a_script_changes_nothing() {
    let root = temp_tutorial("setup-absent");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play_with(&root, &transcript, &["--run-setup".to_string()]);
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "1 command played\n");
    assert_eq!(
        string_field(transcript_commands(&transcript).get(0).expect("echo"), "output"),
        "hi\n"
    );
    server_is_gone(pid);
}

#[test]
fn the_shell_starts_in_the_window_folder() {
    let root = temp_tutorial("window-folder");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
",
        "```bash { window=bob }\npwd\n```\n",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play(&root, &transcript);
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    let commands = transcript_commands(&transcript);
    let command = commands.get(0).expect("pwd");
    let got = string_field(command, "output");
    assert!(got.trim_end().ends_with("/bob"), "working directory was {got:?}");
    server_is_gone(pid);
}

#[test]
fn a_missing_tmux_or_bash_is_named() {
    let dir = manifest_dir().join("examples/one-window");
    let bash = program_on_path("bash");
    let tmux = program_on_path("tmux");
    let only_bash = temp_tutorial("path-bash");
    fs::create_dir_all(&only_bash).expect("a directory");
    std::os::unix::fs::symlink(&bash, only_bash.join("bash")).expect("a bash link");
    let transcript = only_bash.join("transcript.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_tutorial-stepper"))
        .args(["play", "--auto", "--transcript"])
        .arg(&transcript)
        .arg(&dir)
        .env("PATH", &only_bash)
        .output()
        .expect("play runs");
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stderr));
    assert!(text(&output.stderr).contains("tmux"), "{}", text(&output.stderr));
    assert!(!transcript.exists());

    let only_tmux = temp_tutorial("path-tmux");
    fs::create_dir_all(&only_tmux).expect("a directory");
    std::os::unix::fs::symlink(&tmux, only_tmux.join("tmux")).expect("a tmux link");
    let transcript = only_tmux.join("transcript.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_tutorial-stepper"))
        .args(["play", "--auto", "--transcript"])
        .arg(&transcript)
        .arg(&dir)
        .env("PATH", &only_tmux)
        .output()
        .expect("play runs");
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stderr));
    assert!(text(&output.stderr).contains("bash"), "{}", text(&output.stderr));
    assert!(!transcript.exists());
}

fn executable(dir: &Path, name: &str, body: &str) {
    fs::create_dir_all(dir).expect("the program directory");
    let path = dir.join(name);
    fs::write(&path, body).expect("the program");
    let mut permissions = fs::metadata(&path).expect("the program metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("the program is executable");
}

fn first_dotted(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let mut end = index;
        let mut groups = 0;
        while end < bytes.len() && bytes[end] == b'.' {
            let mut cursor = end + 1;
            if cursor >= bytes.len() || !bytes[cursor].is_ascii_digit() {
                break;
            }
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            end = cursor;
            groups += 1;
        }
        if groups >= 1 {
            return Some(&text[start..end]);
        }
    }
    None
}

fn dotted_version(program: &str) -> String {
    let output = Command::new(program).arg("--version").output().expect("the program runs");
    let mut combined = text(&output.stdout);
    combined.push_str(&text(&output.stderr));
    first_dotted(&combined).expect("a dotted version").to_string()
}

#[test]
fn a_missing_program_is_named_before_any_command() {
    let root = manifest_dir().join("examples/needs-missing");
    let run = temp_tutorial("needs-missing-run");
    let transcript = temp_tutorial("needs-missing-transcript").join("transcript.toml");
    fs::create_dir_all(transcript.parent().expect("a parent")).expect("the transcript directory");
    let (output, pid) =
        play_with(&root, &transcript, &["--run".to_string(), run.display().to_string()]);
    assert_eq!(output.status.code(), Some(1), "stdout:\n{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("tutorial-stepper-missing-program"), "{err}");
    assert!(err.contains("was not found"), "{err}");
    assert!(!transcript.exists(), "a refused needs check wrote a transcript");
    assert!(!run.exists(), "a refused needs check created the run directory");
    server_is_gone(pid);
}

#[test]
fn an_older_program_is_named_with_the_version_found() {
    let root = temp_tutorial("needs-older");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
needs = [\"bash >= 99.0.0\"]
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    let run = temp_tutorial("needs-older-run");
    let transcript = root.join("transcript.toml");
    let (output, pid) =
        play_with(&root, &transcript, &["--run".to_string(), run.display().to_string()]);
    assert_eq!(output.status.code(), Some(1), "stdout:\n{}", text(&output.stdout));
    let found = dotted_version("bash");
    let err = text(&output.stderr);
    assert!(err.contains(&format!("bash {found} is older than 99.0.0")), "{err}");
    assert!(!transcript.exists());
    assert!(!run.exists(), "a refused needs check created the run directory");
    server_is_gone(pid);
}

#[test]
fn a_program_that_prints_no_version_is_named() {
    let root = temp_tutorial("needs-no-version");
    executable(
        &root.join("bin"),
        "tutorial-stepper-no-version",
        "#!/bin/sh\nprintf 'release\\n'\n",
    );
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
needs = [\"tutorial-stepper-no-version >= 1.0\"]
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play_prefixed(&root, &transcript, &[], Some(&root.join("bin")));
    assert_eq!(output.status.code(), Some(1), "stdout:\n{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("tutorial-stepper-no-version"), "{err}");
    assert!(err.contains("printed no version"), "{err}");
    assert!(!transcript.exists());
    server_is_gone(pid);
}

#[test]
fn a_missing_program_stops_before_the_setup_script() {
    let root = temp_tutorial("needs-before-setup");
    write_tutorial(
        &root,
        "\
setup = \"setup.sh\"
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
needs = [\"tutorial-stepper-missing-program\"]
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    let marker = root.join("marker");
    fs::write(root.join("setup.sh"), format!("printf ran > {}\n", marker.display()))
        .expect("the set-up script");
    let run = temp_tutorial("needs-before-setup-run");
    let transcript = root.join("transcript.toml");
    let (output, pid) = play_with(
        &root,
        &transcript,
        &["--run".to_string(), run.display().to_string(), "--run-setup".to_string()],
    );
    assert_eq!(output.status.code(), Some(1), "stdout:\n{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("tutorial-stepper-missing-program was not found"), "{err}");
    assert!(!marker.exists(), "the set-up script ran");
    assert!(!transcript.exists());
    assert!(!run.exists(), "a refused needs check created the run directory");
    server_is_gone(pid);
}

#[test]
fn the_first_failing_need_is_the_one_named() {
    let root = temp_tutorial("needs-order");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"alice\"
folder = \"alice\"
needs = [\"tutorial-stepper-missing-program\"]

[[window]]
name = \"bob\"
folder = \"bob\"
needs = [\"bash >= 99.0.0\"]
",
        "```bash { window=alice }\necho hi\n```\n",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play(&root, &transcript);
    assert_eq!(output.status.code(), Some(1), "stdout:\n{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("tutorial-stepper-missing-program was not found"), "{err}");
    assert!(!err.contains("older"), "{err}");
    assert!(!transcript.exists());
    server_is_gone(pid);
}

#[test]
fn a_version_is_compared_by_numeric_components() {
    let root = temp_tutorial("needs-numeric");
    executable(&root.join("bin"), "number-tool", "#!/bin/sh\nprintf '1.9\\n'\n");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
needs = [\"number-tool >= 1.10\"]
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play_prefixed(&root, &transcript, &[], Some(&root.join("bin")));
    assert_eq!(output.status.code(), Some(1), "stdout:\n{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("number-tool 1.9 is older than 1.10"), "{err}");
    assert!(!transcript.exists());
    server_is_gone(pid);
}

#[test]
fn a_shorter_version_is_padded_with_zeros() {
    let root = temp_tutorial("needs-padded");
    executable(&root.join("bin"), "number-tool", "#!/bin/sh\nprintf '1.2\\n'\n");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
needs = [\"number-tool >= 1.2.1\"]
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play_prefixed(&root, &transcript, &[], Some(&root.join("bin")));
    assert_eq!(output.status.code(), Some(1), "stdout:\n{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("number-tool 1.2 is older than 1.2.1"), "{err}");
    assert!(!transcript.exists());
    server_is_gone(pid);
}

#[test]
fn the_version_on_standard_error_counts() {
    let root = temp_tutorial("needs-stderr");
    executable(&root.join("bin"), "number-tool", "#!/bin/sh\nprintf 'tool 4.5.6\\n' >&2\n");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
needs = [\"number-tool >= 9.0\"]
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play_prefixed(&root, &transcript, &[], Some(&root.join("bin")));
    assert_eq!(output.status.code(), Some(1), "stdout:\n{}", text(&output.stdout));
    let err = text(&output.stderr);
    assert!(err.contains("number-tool 4.5.6 is older than 9.0"), "{err}");
    assert!(!transcript.exists());
    server_is_gone(pid);
}

#[test]
fn a_program_with_no_minimum_is_not_asked_for_its_version() {
    let root = temp_tutorial("needs-present");
    let record = root.join("record");
    executable(
        &root.join("bin"),
        "tutorial-stepper-present",
        &format!("#!/bin/sh\nprintf '%s\\n' \"$*\" > {}\n", record.display()),
    );
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
needs = [\"tutorial-stepper-present\"]
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play_prefixed(&root, &transcript, &[], Some(&root.join("bin")));
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "1 command played\n");
    assert!(!record.exists(), "a need with no minimum ran the program");
    server_is_gone(pid);
}

#[test]
fn an_equal_version_is_new_enough() {
    let root = temp_tutorial("needs-equal");
    executable(&root.join("bin"), "number-tool", "#!/bin/sh\nprintf '1.2.0\\n'\n");
    write_tutorial(
        &root,
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
needs = [\"number-tool >= 1.2\"]
",
        "```bash { window=bob }\necho hi\n```\n",
    );
    let transcript = root.join("transcript.toml");
    let (output, pid) = play_prefixed(&root, &transcript, &[], Some(&root.join("bin")));
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "1 command played\n");
    server_is_gone(pid);
}

fn program_on_path(name: &str) -> PathBuf {
    let paths = std::env::var_os("PATH").expect("PATH");
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("{name} is not on PATH");
}
