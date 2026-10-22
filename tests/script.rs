//! `tutorial-stepper script` prints runnable commands as one bash script.

use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_tutorial-stepper"))
        .args(args)
        .output()
        .expect("the tutorial-stepper binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("the tutorial-stepper output is utf-8")
}

fn script(relative: &str) -> std::process::Output {
    let dir = manifest_dir().join(relative);
    run([OsStr::new("script"), dir.as_os_str()])
}

fn sound_examples() -> Vec<String> {
    let mut names = Vec::new();
    for entry in
        fs::read_dir(manifest_dir().join("examples")).expect("the example directory exists")
    {
        let entry = entry.expect("an example directory entry");
        if entry.file_type().expect("a file type").is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name != "invalid" {
                names.push(name);
            }
        }
    }
    names.sort();
    names
}

#[test]
fn one_window_names_the_page_and_the_window() {
    let output = script("examples/one-window");
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    assert!(output.stderr.is_empty(), "stderr:\n{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        "\
#!/usr/bin/env bash

# welcome.md, window bob
echo hi
"
    );
}

#[test]
fn show_is_left_out_and_the_observation_appears_where_it_stands() {
    let output = script("examples/mode");
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    let got = text(&output.stdout);
    assert!(!got.contains("shown"), "a block marked mode=show was printed:\n{got}");
    assert_eq!(
        got,
        "\
#!/usr/bin/env bash

# look.md, window bob
echo observed

# look.md, window bob
echo run
"
    );
}

#[test]
fn a_declared_status_does_not_remove_the_command() {
    let output = script("examples/status");
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        "\
#!/usr/bin/env bash

# build.md, window bob
echo not yet
"
    );
}

#[test]
fn one_tested_path_prints_only_its_branch() {
    let external = run([
        OsStr::new("script"),
        OsStr::new("--path"),
        OsStr::new("external"),
        manifest_dir().join("examples/if").as_os_str(),
    ]);
    assert!(external.status.success(), "stderr:\n{}", text(&external.stderr));
    assert!(external.stderr.is_empty(), "stderr:\n{}", text(&external.stderr));
    assert_eq!(
        text(&external.stdout),
        "\
#!/usr/bin/env bash

# sign.md, window alice
echo external
"
    );

    let internal = run([
        OsStr::new("script"),
        OsStr::new("--path"),
        OsStr::new("internal"),
        manifest_dir().join("examples/if").as_os_str(),
    ]);
    assert!(internal.status.success(), "stderr:\n{}", text(&internal.stderr));
    assert_eq!(
        text(&internal.stdout),
        "\
#!/usr/bin/env bash

# sign.md, window alice
echo internal
"
    );
}

#[test]
fn a_shared_block_stays_and_the_other_branch_is_left_out() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("script-one-path");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("the temporary tutorial directory");
    fs::write(
        root.join("tutorial.toml"),
        "\
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"

[[setting]]
name = \"SIGNING\"
default = \"external\"
values = [\"external\", \"internal\"]

[[path]]
name = \"external\"
values = { SIGNING = \"external\" }

[[path]]
name = \"internal\"
values = { SIGNING = \"internal\" }
",
    )
    .expect("the index is written");
    fs::write(
        root.join("welcome.md"),
        "\
```bash { window=bob mode=show if=SIGNING:external }
echo shown
```

```bash { window=bob mode=observe if=SIGNING:external }
echo look external
```

```bash { window=bob mode=observe if=SIGNING:internal }
echo look internal
```

```bash { window=bob }
echo shared
```

```bash { window=bob if=SIGNING:external }
echo external
```

```bash { window=bob if=SIGNING:internal }
echo internal
```
",
    )
    .expect("the page is written");

    let output =
        run([OsStr::new("script"), OsStr::new("--path"), OsStr::new("external"), root.as_os_str()]);
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        "\
#!/usr/bin/env bash

# welcome.md, window bob
echo look external

# welcome.md, window bob
echo shared

# welcome.md, window bob
echo external
"
    );
}

#[test]
fn an_unknown_path_names_itself_and_prints_nothing() {
    let output = run([
        OsStr::new("script"),
        OsStr::new("--path"),
        OsStr::new("nope"),
        manifest_dir().join("examples/if").as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(1), "stdout:\n{}", text(&output.stdout));
    assert!(output.stdout.is_empty(), "{}", text(&output.stdout));
    let index = manifest_dir().join("examples/if/tutorial.toml");
    assert_eq!(
        text(&output.stderr),
        format!("{}:1: path \"nope\" is not a tested path\n", index.display())
    );
}

#[test]
fn a_broken_tutorial_ignores_the_path_and_prints_the_check_problems() {
    let dir = manifest_dir().join("examples/invalid/undeclared-setting");
    let checked = run([OsStr::new("check"), dir.as_os_str()]);
    let printed =
        run([OsStr::new("script"), OsStr::new("--path"), OsStr::new("external"), dir.as_os_str()]);
    assert!(!checked.status.success());
    assert!(printed.stdout.is_empty(), "{}", text(&printed.stdout));
    assert_eq!(printed.status.code(), checked.status.code());
    assert_eq!(text(&printed.stderr), text(&checked.stderr));
}

#[test]
fn both_setting_branches_are_printed_in_order() {
    let output = script("examples/if");
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        "\
#!/usr/bin/env bash

# sign.md, window alice
echo external

# sign.md, window alice
echo internal
"
    );
}

#[test]
fn pages_follow_the_index_and_ordinary_blocks_are_left_out() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("pages-follow-the-index");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("the temporary tutorial directory");
    fs::write(
        root.join("tutorial.toml"),
        "\
pages = [\"second.md\", \"first.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
",
    )
    .expect("the index is written");
    fs::write(
        root.join("second.md"),
        "\
```bash
echo plain
```

```bash { window=bob mode=show }
echo shown
```

```bash { }
echo bare
```

```bash { window=bob mode=observe }
echo look
```

```bash { window=bob }
echo one
echo two
```
",
    )
    .expect("the first page in the index is written");
    fs::write(
        root.join("first.md"),
        "\
```bash { window=bob }
echo first
```
",
    )
    .expect("the second page in the index is written");

    let output = run([OsStr::new("script"), root.as_os_str()]);
    assert!(output.status.success(), "stderr:\n{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        "\
#!/usr/bin/env bash

# second.md, window not named
echo bare

# second.md, window bob
echo look

# second.md, window bob
echo one
echo two

# first.md, window bob
echo first
"
    );
    assert!(!text(&output.stdout).contains("echo plain"));
    assert!(!text(&output.stdout).contains("echo shown"));
}

#[test]
fn a_broken_tutorial_prints_the_same_problems_as_check() {
    let mut invalid = Vec::new();
    let root = manifest_dir().join("examples/invalid");
    for entry in fs::read_dir(&root).expect("the invalid examples exist") {
        let entry = entry.expect("an invalid example");
        if entry.file_type().expect("a file type").is_dir() {
            invalid.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    invalid.sort();
    assert!(!invalid.is_empty(), "examples/invalid has tutorials");

    for name in invalid {
        let dir = root.join(&name);
        let checked = run([OsStr::new("check"), dir.as_os_str()]);
        let printed = run([OsStr::new("script"), dir.as_os_str()]);
        assert!(
            !checked.status.success(),
            "examples/invalid/{name} passed check, so it does not exercise a refusal"
        );
        assert!(
            !printed.status.success(),
            "examples/invalid/{name} printed a script:\n{}",
            text(&printed.stdout)
        );
        assert!(
            printed.stdout.is_empty(),
            "examples/invalid/{name} wrote a script:\n{}",
            text(&printed.stdout)
        );
        assert_eq!(printed.status.code(), checked.status.code(), "examples/invalid/{name}");
        assert_eq!(
            text(&printed.stderr),
            text(&checked.stderr),
            "examples/invalid/{name} problems differ from check"
        );
    }
}

#[test]
fn every_sound_example_is_a_bash_script() {
    let names = sound_examples();
    assert_eq!(
        names.join(" "),
        "continued declared-status if mode needs-missing one-window setup slow status two-windows",
        "every sound example is passed to bash -n"
    );
    for name in names {
        let output = script(&format!("examples/{name}"));
        let got = text(&output.stdout);
        assert!(
            output.status.success(),
            "examples/{name} exited {:?}\nstdout:\n{got}\nstderr:\n{}",
            output.status.code(),
            text(&output.stderr)
        );
        let mut child = Command::new("bash")
            .arg("-n")
            .stdin(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("bash runs");
        child
            .stdin
            .take()
            .expect("a stdin pipe")
            .write_all(got.as_bytes())
            .expect("the script is written");
        let finished = child.wait_with_output().expect("bash finishes");
        assert!(
            finished.status.success(),
            "bash -n rejected examples/{name}:\n{}\n--- script ---\n{got}",
            text(&finished.stderr)
        );
    }
}
