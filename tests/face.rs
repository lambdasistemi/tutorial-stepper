//! `tutorial-stepper play DIR` in a detached tmux session, driven by keys.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn unique() -> u64 {
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn tmux_bin() -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH is set");
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join("tmux");
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("tmux is not on PATH");
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

struct Face {
    tmux: PathBuf,
    run: PathBuf,
    home: PathBuf,
    scratch: PathBuf,
    tutorial: PathBuf,
    with_run: bool,
}

impl Face {
    fn launch(tutorial: PathBuf, with_run: bool) -> Self {
        let scratch = std::env::temp_dir().join(format!(
            "tutorial-stepper-face-{}-{}",
            std::process::id(),
            unique()
        ));
        let home = scratch.join("home");
        fs::create_dir_all(&home).expect("the home directory is created");
        let run = scratch.join("run");
        if with_run {
            fs::create_dir_all(&run).expect("the run directory is created");
        }
        let face = Self { tmux: tmux_bin(), run, home, scratch, tutorial, with_run };
        face.attach_again();
        face
    }

    fn attach_again(&self) {
        let bin = env!("CARGO_BIN_EXE_tutorial-stepper");
        let mut command = format!(
            "exec env HOME={} {} play",
            shell_quote(&self.home.to_string_lossy()),
            shell_quote(bin)
        );
        if self.with_run {
            command.push_str(" --run ");
            command.push_str(&shell_quote(&self.run.to_string_lossy()));
        }
        command.push(' ');
        command.push_str(&shell_quote(&self.tutorial.to_string_lossy()));
        let outer = format!("face-outer-{}-{}", std::process::id(), unique());
        let status = Command::new(&self.tmux)
            .args([
                "-f",
                "/dev/null",
                "-L",
                &outer,
                "new-session",
                "-d",
                "-s",
                "drive",
                "-x",
                "160",
                "-y",
                "40",
                &command,
            ])
            .env_remove("TMUX")
            .status()
            .expect("the outer tmux session starts");
        assert!(status.success(), "the outer tmux session did not start");
        // The socket is recorded after the server exists so Drop can kill it.
        // A panic before that still kills this outer session.
        let _ = fs::write(self.scratch.join(format!("outer-{outer}")), &outer);
    }

    fn run_dir(&self) -> PathBuf {
        if self.with_run {
            return self.run.clone();
        }
        let name = self
            .tutorial
            .canonicalize()
            .unwrap_or_else(|_| self.tutorial.clone())
            .file_name()
            .expect("the tutorial directory has a name")
            .to_string_lossy()
            .into_owned();
        self.home.join(format!("{name}-run"))
    }

    fn socket(&self) -> Option<String> {
        let text =
            fs::read_to_string(self.run_dir().join(".tutorial-stepper/session.toml")).ok()?;
        let document: toml_edit::DocumentMut = text.parse().ok()?;
        document.get("socket").and_then(toml_edit::Item::as_str).map(str::to_string)
    }

    fn page_id(&self) -> Option<String> {
        let text = fs::read_to_string(self.run_dir().join(".tutorial-stepper/page-pane")).ok()?;
        let id = text.trim();
        if id.is_empty() {
            None
        } else {
            Some(id.to_string())
        }
    }

    fn tmux(&self, socket: &str, args: &[&str]) -> std::process::Output {
        let mut command = Command::new(&self.tmux);
        command.args(["-f", "/dev/null", "-L", socket]);
        command.args(args);
        command.env_remove("TMUX").output().expect("tmux runs")
    }

    fn capture(&self, socket: &str, target: &str) -> Option<String> {
        let output = self.tmux(socket, &["capture-pane", "-p", "-t", target]);
        output.status.success().then(|| String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn outer_sockets(&self) -> Vec<String> {
        let mut names = Vec::new();
        let entries = fs::read_dir(&self.scratch).ok();
        let Some(entries) = entries else {
            return names;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(socket) = name.strip_prefix("outer-") {
                names.push(socket.to_string());
            }
        }
        names
    }

    fn dump(&self) -> String {
        let mut text = String::new();
        for socket in self.outer_sockets() {
            let command = self.tmux(
                &socket,
                &["display-message", "-p", "-t", "drive", "#{pane_current_command}"],
            );
            let screen = self.capture(&socket, "drive").unwrap_or_default();
            text.push_str(&format!(
                "outer {socket} command {}\n{screen}\n",
                String::from_utf8_lossy(&command.stdout).trim()
            ));
        }
        if let Some(socket) = self.socket() {
            let alive = self.tmux(&socket, &["has-session", "-t", "play"]);
            let panes = self.tmux(
                &socket,
                &[
                    "list-panes",
                    "-a",
                    "-F",
                    "#{pane_id} #{window_name} #{pane_current_command} dead=#{pane_dead}",
                ],
            );
            text.push_str(&format!(
                "inner {socket} has-session {} {}\npanes {}\n{}\n",
                alive.status,
                String::from_utf8_lossy(&alive.stderr).trim(),
                panes.status,
                String::from_utf8_lossy(&panes.stdout)
            ));
            if let Some(page) = self.page_id() {
                let window =
                    self.tmux(&socket, &["display-message", "-p", "-t", &page, "#{window_name}"]);
                text.push_str(&format!(
                    "page id {page} open {} window {}\n",
                    self.pane_open(&page),
                    String::from_utf8_lossy(&window.stdout).trim()
                ));
                text.push_str(&self.capture(&socket, &page).unwrap_or_default());
                text.push_str("\n-- shell --\n");
                let name = String::from_utf8_lossy(&window.stdout).trim().to_string();
                if !name.is_empty() {
                    let target = format!("play:={name}");
                    text.push_str(
                        &self
                            .capture(&socket, &target)
                            .unwrap_or_else(|| "no shell capture".to_string()),
                    );
                }
            }
            text.push_str("\n-- log bob --\n");
            text.push_str(&self.log("bob"));
            text.push_str("\n-- log alice --\n");
            text.push_str(&self.log("alice"));
        }
        text
    }

    fn wait_until<T>(
        &self,
        timeout: Duration,
        what: &str,
        mut probe: impl FnMut(&Self) -> Option<T>,
    ) -> T {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Some(value) = probe(self) {
                return value;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("timed out waiting for {what} after {timeout:?}\n{}", self.dump());
    }

    fn wait_page(&self, needle: &str) -> String {
        self.wait_until(Duration::from_secs(20), needle, |face| {
            let socket = face.socket()?;
            let page = face.page_id()?;
            let text = face.capture(&socket, &page)?;
            text.contains(needle).then_some(text)
        })
    }

    fn wait_attached(&self) {
        self.wait_until(Duration::from_secs(20), "the learner's terminal to attach", |face| {
            face.outer_sockets().into_iter().find_map(|socket| {
                let output = face.tmux(
                    &socket,
                    &["display-message", "-p", "-t", "drive", "#{pane_current_command}"],
                );
                let command = String::from_utf8_lossy(&output.stdout);
                (output.status.success() && command.trim() == "tmux").then_some(())
            })
        });
    }

    fn send(&self, keys: &[&str]) {
        let socket = self.socket().unwrap_or_else(|| panic!("no server yet\n{}", self.dump()));
        let page = self.page_id().unwrap_or_else(|| panic!("no page yet\n{}", self.dump()));
        let mut args = vec!["send-keys", "-t", page.as_str()];
        args.extend(keys);
        let output = self.tmux(&socket, &args);
        assert!(
            output.status.success(),
            "send-keys {keys:?} failed: {}\n{}",
            String::from_utf8_lossy(&output.stderr),
            self.dump()
        );
    }

    fn window_of_page(&self) -> String {
        let socket = self.socket().expect("the server is up");
        let page = self.page_id().expect("the page pane is recorded");
        let output = self.tmux(&socket, &["display-message", "-p", "-t", &page, "#{window_name}"]);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn page_is_left_of_the_shell(&self, window: &str) {
        let socket = self.socket().expect("the server is up");
        let page = self.page_id().expect("the page pane is recorded");
        let target = format!("play:={window}");
        let output =
            self.tmux(&socket, &["list-panes", "-t", &target, "-F", "#{pane_id} #{pane_left}"]);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let text = String::from_utf8_lossy(&output.stdout);
        let mut page_left = None;
        let mut shell_left = None;
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let Some(id) = parts.next() else { continue };
            let Some(left) = parts.next().and_then(|value| value.parse::<i64>().ok()) else {
                continue;
            };
            if id == page {
                page_left = Some(left);
            } else {
                shell_left = Some(left);
            }
        }
        let page_left = page_left
            .unwrap_or_else(|| panic!("the page is not in {window}:\n{text}\n{}", self.dump()));
        let shell_left = shell_left.expect("the window has its shell");
        assert!(page_left < shell_left, "the page is not to the left of the shell:\n{text}");
    }

    fn log(&self, window: &str) -> String {
        fs::read_to_string(
            self.run_dir().join(".tutorial-stepper/control").join(format!("log-{window}")),
        )
        .unwrap_or_default()
    }

    /// The shell pane, not the page that shares its window.
    fn shell_screen(&self, window: &str) -> Option<String> {
        let text =
            fs::read_to_string(self.run_dir().join(".tutorial-stepper/session.toml")).ok()?;
        let document: toml_edit::DocumentMut = text.parse().ok()?;
        let windows = document.get("window")?.as_array_of_tables()?;
        let pane = windows.iter().find_map(|table| {
            let name = table.get("name").and_then(toml_edit::Item::as_str)?;
            (name == window).then(|| table.get("pane").and_then(toml_edit::Item::as_str))?
        })?;
        let socket = self.socket()?;
        self.capture(&socket, pane)
    }

    fn wait_log(&self, window: &str, needle: &str) -> String {
        self.wait_until(Duration::from_secs(15), needle, |face| {
            let text = face.log(window);
            text.contains(needle).then_some(text)
        })
    }

    /// Lines the shell showed. A carriage return redraws the line, so the
    /// visible text is the part after the last one.
    fn shown_lines(&self, window: &str, text: &str) -> usize {
        self.log(window)
            .lines()
            .filter(|line| line.trim_end_matches('\r').rsplit('\r').next() == Some(text))
            .count()
    }

    fn position(&self) -> String {
        fs::read_to_string(self.run_dir().join(".tutorial-stepper/position.toml"))
            .unwrap_or_default()
    }

    fn server_up(&self) -> bool {
        let Some(socket) = self.socket() else {
            return false;
        };
        self.tmux(&socket, &["has-session", "-t", "play"]).status.success()
    }

    fn pane_open(&self, pane: &str) -> bool {
        let Some(socket) = self.socket() else {
            return false;
        };
        // display-message exits 0 for a pane that has already closed.
        self.tmux(&socket, &["list-panes", "-t", pane, "-F", "#{pane_id}"]).status.success()
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        for socket in self.outer_sockets() {
            let _ = Command::new(&self.tmux)
                .args(["-f", "/dev/null", "-L", &socket, "kill-server"])
                .env_remove("TMUX")
                .output();
        }
        if let Some(socket) = self.socket() {
            let _ = Command::new(&self.tmux)
                .args(["-f", "/dev/null", "-L", &socket, "kill-server"])
                .env_remove("TMUX")
                .output();
        }
        let _ = fs::remove_dir_all(&self.scratch);
    }
}

fn write_tutorial(dir: &Path, pages: &[(&str, &str)]) {
    fs::create_dir_all(dir).expect("the tutorial directory is created");
    let listed = pages.iter().map(|(name, _)| format!("\"{name}\"")).collect::<Vec<_>>().join(", ");
    let index = format!("pages = [{listed}]\n\n[[window]]\nname = \"bob\"\nfolder = \"bob\"\n");
    fs::write(dir.join("tutorial.toml"), index).expect("the index is written");
    for (name, body) in pages {
        fs::write(dir.join(name), body).expect("a page is written");
    }
}

fn named_tutorial(name: &str, pages: &[(&str, &str)]) -> (TutorialGuard, PathBuf) {
    let parent = std::env::temp_dir().join(format!(
        "tutorial-stepper-face-src-{}-{}",
        std::process::id(),
        unique()
    ));
    let dir = parent.join(name);
    write_tutorial(&dir, pages);
    (TutorialGuard(parent), dir)
}

struct TutorialGuard(PathBuf);

impl Drop for TutorialGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_page_sits_beside_the_window_and_enter_runs_the_command() {
    let (_keep, tutorial) = named_tutorial(
        "rich",
        &[(
            "welcome.md",
            "\
# Title

A **bold** word, `code`, and a [name](https://example.com).

- alpha

```bash { window=bob }
echo ran-from-the-page
```
",
        )],
    );
    let face = Face::launch(tutorial, true);
    face.wait_attached();
    let page = face.wait_page("Enter runs it");
    assert!(page.contains("Title"), "{page}");
    assert!(page.contains("bold"), "{page}");
    assert!(page.contains("code"), "{page}");
    assert!(page.contains("https://example.com"), "{page}");
    assert!(page.contains("alpha"), "{page}");
    assert!(page.contains("next"), "{page}");
    assert!(page.contains("ran-from-the-page"), "{page}");
    assert_eq!(face.window_of_page(), "bob");
    face.page_is_left_of_the_shell("bob");
    assert!(!face.home.join("rich-run").exists(), "the default folder was used as well as --run");
    face.send(&["Enter"]);
    let log = face.wait_log("bob", "ran-from-the-page");
    assert!(log.contains("ran-from-the-page"), "{log}");
    face.wait_page("That was the tutorial");
}

#[test]
fn the_page_moves_beside_the_next_window() {
    let tutorial = manifest_dir().join("examples/two-windows");
    let face = Face::launch(tutorial, true);
    face.wait_attached();
    face.wait_page("Enter runs it");
    assert_eq!(face.window_of_page(), "alice");
    face.page_is_left_of_the_shell("alice");
    face.send(&["Enter"]);
    face.wait_until(Duration::from_secs(15), "the page to sit beside bob", |face| {
        (face.window_of_page() == "bob").then_some(())
    });
    face.page_is_left_of_the_shell("bob");
    let alice = face.run_dir().join("alice").canonicalize().expect("alice's folder exists");
    let log = face.wait_log("alice", &alice.to_string_lossy());
    assert!(log.contains(alice.to_string_lossy().as_ref()), "{log}");
}

#[test]
fn skip_goes_past_the_command_without_running_it() {
    let (_keep, tutorial) = named_tutorial(
        "skip",
        &[("welcome.md", "```bash { window=bob }\necho skip-marker-not-run\n```\n")],
    );
    let face = Face::launch(tutorial, true);
    face.wait_page("Enter runs it");
    face.send(&["s"]);
    face.wait_page("That was the tutorial");
    assert!(!face.log("bob").contains("skip-marker-not-run"));
}

#[test]
fn c_interrupts_the_running_command() {
    let (_keep, tutorial) = named_tutorial(
        "interrupt",
        &[("welcome.md", "```bash { window=bob status=130 }\nsleep 30\n```\n")],
    );
    let face = Face::launch(tutorial, true);
    face.wait_page("Enter runs it");
    let started = Instant::now();
    face.send(&["Enter"]);
    face.wait_page("running in bob");
    face.send(&["c"]);
    face.wait_page("That was the tutorial");
    assert!(started.elapsed() < Duration::from_secs(15), "sleep 30 was not interrupted");
}

#[test]
fn back_returns_to_the_previous_command_and_leaves_its_file() {
    let (_keep, tutorial) = named_tutorial(
        "back",
        &[(
            "welcome.md",
            "\
```bash { window=bob }
touch kept-by-back.txt
```

```bash { window=bob }
echo second-command
```
",
        )],
    );
    let face = Face::launch(tutorial, true);
    face.wait_page("kept-by-back");
    face.send(&["Enter"]);
    face.wait_until(Duration::from_secs(15), "the second command", |face| {
        face.position().contains("line = 5\n").then_some(())
    });
    let kept = face.run_dir().join("bob/kept-by-back.txt");
    assert!(kept.is_file(), "the first command did not leave its file");
    let after_first = face.position();
    face.send(&["b"]);
    face.wait_until(Duration::from_secs(10), "the position to move back", |face| {
        let text = face.position();
        (text != after_first && text.contains("line = 1\n")).then_some(())
    });
    assert!(kept.is_file(), "back removed the file the command had created");
    let page = face.wait_page("Enter runs it");
    assert!(page.contains("kept-by-back"), "{page}");
}

#[test]
fn n_and_p_turn_the_page_without_moving_the_next_command() {
    let (_keep, tutorial) = named_tutorial(
        "pages",
        &[
            (
                "one.md",
                "\
# First

```bash { window=bob }
echo page-turn-marker
```
",
            ),
            ("two.md", "# Second\n\nThe next page.\n"),
        ],
    );
    let face = Face::launch(tutorial, true);
    face.wait_page("First");
    let position = face.position();
    assert!(position.contains("one.md"), "{position}");
    face.send(&["n"]);
    let turned = face.wait_page("Second");
    assert!(turned.contains("Enter runs the next command"), "{turned}");
    let after = face.position();
    assert!(after.contains("one.md"), "turning the page moved the next command:\n{after}");
    assert!(after.contains("view = \"two.md\""), "{after}");
    face.send(&["p"]);
    let back = face.wait_page("First");
    assert!(back.contains("Enter runs it"), "{back}");
    assert!(!face.log("bob").contains("page-turn-marker"));
}

#[test]
fn an_unexpected_status_can_be_retried_or_skipped() {
    let tutorial = manifest_dir().join("examples/status");
    let face = Face::launch(tutorial, true);
    face.wait_page("Enter runs it");
    face.send(&["Enter"]);
    let choice = face.wait_page("Enter retries");
    assert!(choice.contains("exited 0, expected 19"), "{choice}");
    face.send(&["Enter"]);
    face.wait_until(Duration::from_secs(15), "the command to run again", |face| {
        (face.shown_lines("bob", "not yet") >= 2).then_some(())
    });
    let again = face.wait_page("Enter retries");
    assert!(again.contains("exited 0, expected 19"), "{again}");
    face.send(&["s"]);
    face.wait_page("That was the tutorial");
}

#[test]
fn quit_on_an_unexpected_status_keeps_the_choice_for_the_next_play() {
    let tutorial = manifest_dir().join("examples/status");
    let face = Face::launch(tutorial, true);
    face.wait_page("Enter runs it");
    face.send(&["Enter"]);
    face.wait_page("Enter retries");
    let ran = face.shown_lines("bob", "not yet");
    face.send(&["q"]);
    let page = face.page_id().expect("the page was recorded");
    face.wait_until(Duration::from_secs(10), "the page to close", |face| {
        (!face.pane_open(&page) && face.server_up()).then_some(())
    });
    let saved = face.position();
    assert!(saved.contains("got = 0"), "{saved}");
    assert!(saved.contains("expected = 19"), "{saved}");
    face.attach_again();
    let choice = face.wait_page("Enter retries");
    assert!(choice.contains("exited 0, expected 19"), "{choice}");
    thread::sleep(Duration::from_millis(400));
    assert_eq!(face.shown_lines("bob", "not yet"), ran, "resume ran the command again");
}

#[test]
fn quit_keeps_the_windows_and_a_second_play_resumes() {
    let tutorial = manifest_dir().join("examples/one-window");
    let face = Face::launch(tutorial, true);
    face.wait_attached();
    face.wait_page("echo hi");
    face.send(&["q"]);
    face.wait_until(Duration::from_secs(10), "the server to stay up", |face| {
        face.server_up().then_some(())
    });
    let saved = face.position();
    assert!(saved.contains("welcome.md"), "{saved}");
    assert!(saved.contains("done = false"), "{saved}");
    face.attach_again();
    let page = face.wait_page("Enter runs it");
    assert!(page.contains("echo hi"), "{page}");
    assert!(page.contains("next"), "{page}");
    assert_eq!(face.window_of_page(), "bob");
    assert!(!face.log("bob").contains("hi\n") && !face.log("bob").contains("echo hi"));
}

fn tutorial_with_shell(name: &str, shell: &str, page: &str) -> (TutorialGuard, PathBuf) {
    let parent = std::env::temp_dir().join(format!(
        "tutorial-stepper-face-src-{}-{}",
        std::process::id(),
        unique()
    ));
    let dir = parent.join(name);
    fs::create_dir_all(&dir).expect("the tutorial directory is created");
    let index = format!(
        "shell = \"{shell}\"\npages = [\"welcome.md\"]\n\n[[window]]\nname = \"bob\"\nfolder = \"bob\"\n"
    );
    fs::write(dir.join("tutorial.toml"), index).expect("the index is written");
    fs::write(dir.join("welcome.md"), page).expect("the page is written");
    (TutorialGuard(parent), dir)
}

fn shell_has_drawn(face: &Face) -> Option<String> {
    let screen = face.shell_screen("bob")?;
    screen.lines().any(|line| !line.trim().is_empty()).then_some(screen)
}

fn the_prompt_names_the_window(shell: &str) {
    let page = "```bash { window=bob }\necho hi\n```\n";
    let (_keep, tutorial) = tutorial_with_shell("prompt", shell, page);
    let face = Face::launch(tutorial, true);
    face.wait_page("Enter runs it");
    let screen = face.wait_until(Duration::from_secs(15), "the shell to draw", shell_has_drawn);
    assert!(screen.contains("bob$"), "the {shell} prompt does not name the window:\n{screen}");
    assert!(
        !screen.contains("tutorial-stepper-prompt"),
        "the {shell} window shows the internal marker:\n{screen}"
    );
}

fn the_typed_command_shows_in_the_window(shell: &str) {
    let page = "```bash { window=bob }\nprintf 'from-the-command\\n'\n```\n";
    let (_keep, tutorial) = tutorial_with_shell("typed", shell, page);
    let face = Face::launch(tutorial, true);
    face.wait_page("Enter runs it");
    face.send(&["Enter"]);
    let screen = face.wait_until(Duration::from_secs(15), "the command output", |face| {
        let screen = shell_has_drawn(face)?;
        screen.lines().any(|line| line.trim() == "from-the-command").then_some(screen)
    });
    assert!(
        screen.contains("printf 'from-the-command"),
        "the {shell} window does not show the typed command:\n{screen}"
    );
    assert!(
        !screen.contains("tutorial-stepper-prompt"),
        "the {shell} window shows the internal marker:\n{screen}"
    );
}

#[test]
fn the_bash_prompt_names_the_window() {
    the_prompt_names_the_window("bash");
}

#[test]
fn the_zsh_prompt_names_the_window() {
    the_prompt_names_the_window("zsh");
}

#[test]
fn the_typed_command_shows_in_the_bash_window() {
    the_typed_command_shows_in_the_window("bash");
}

#[test]
fn the_typed_command_shows_in_the_zsh_window() {
    the_typed_command_shows_in_the_window("zsh");
}

#[test]
fn the_default_run_folder_is_named_after_the_tutorial_under_home() {
    let (_keep, tutorial) =
        named_tutorial("sample", &[("welcome.md", "```bash { window=bob }\necho hi\n```\n")]);
    let face = Face::launch(tutorial, false);
    face.wait_page("Enter runs it");
    let run = face.run_dir();
    assert_eq!(run, face.home.join("sample-run"));
    assert!(run.join(".tutorial-stepper/session.toml").is_file());
    assert!(run.join(".tutorial-stepper/position.toml").is_file());
}
