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
    fn shell_pane(&self, window: &str) -> Option<String> {
        let text =
            fs::read_to_string(self.run_dir().join(".tutorial-stepper/session.toml")).ok()?;
        let document: toml_edit::DocumentMut = text.parse().ok()?;
        let windows = document.get("window").and_then(toml_edit::Item::as_array_of_tables)?;
        for table in windows {
            let Some(name) = table.get("name").and_then(toml_edit::Item::as_str) else {
                continue;
            };
            if name == window {
                return table.get("pane").and_then(toml_edit::Item::as_str).map(str::to_string);
            }
        }
        None
    }

    fn shell_screen(&self, window: &str) -> Option<String> {
        let pane = self.shell_pane(window)?;
        let socket = self.socket()?;
        self.capture(&socket, &pane)
    }

    fn send_shell(&self, window: &str, keys: &[&str]) {
        let socket = self.socket().unwrap_or_else(|| panic!("no server yet\n{}", self.dump()));
        let pane = self
            .shell_pane(window)
            .unwrap_or_else(|| panic!("no shell pane for {window}\n{}", self.dump()));
        let mut args = vec!["send-keys", "-t", pane.as_str()];
        args.extend(keys);
        let output = self.tmux(&socket, &args);
        assert!(
            output.status.success(),
            "send-keys {keys:?} failed: {}\n{}",
            String::from_utf8_lossy(&output.stderr),
            self.dump()
        );
    }

    fn send_shell_text(&self, window: &str, text: &str) {
        let socket = self.socket().unwrap_or_else(|| panic!("no server yet\n{}", self.dump()));
        let pane = self
            .shell_pane(window)
            .unwrap_or_else(|| panic!("no shell pane for {window}\n{}", self.dump()));
        let output = self.tmux(&socket, &["send-keys", "-l", "-t", &pane, text]);
        assert!(
            output.status.success(),
            "send-keys text failed: {}\n{}",
            String::from_utf8_lossy(&output.stderr),
            self.dump()
        );
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

fn push_leaves_several_lines_unrun(shell: &str) {
    let page = "\
```bash { window=bob }
printf 'pushed-first\\n'
printf 'pushed-second\\n'
```
";
    let (_keep, tutorial) = tutorial_with_shell("push-wait", shell, page);
    let face = Face::launch(tutorial, true);
    face.wait_page("Enter runs it");
    let before = face.position();
    face.send(&["e"]);
    let screen =
        face.wait_until(Duration::from_secs(15), "the unrun command at the prompt", |face| {
            let screen = face.shell_screen("bob")?;
            (screen.contains("printf 'pushed-first") && screen.contains("printf 'pushed-second"))
                .then_some(screen)
        });
    thread::sleep(Duration::from_millis(500));
    let screen = face.shell_screen("bob").unwrap_or(screen);
    assert!(
        !screen.lines().any(|line| line.trim() == "pushed-first" || line.trim() == "pushed-second"),
        "the {shell} command ran before Enter:\n{screen}"
    );
    assert_eq!(face.position(), before, "e moved the next command");
}

fn an_edited_push_is_what_the_page_shows(shell: &str) {
    let page = "```bash { window=bob }\necho hi\n```\n";
    let (_keep, tutorial) = tutorial_with_shell("push-edit", shell, page);
    let face = Face::launch(tutorial, true);
    face.wait_page("Enter runs it");
    face.send(&["e"]);
    face.wait_until(Duration::from_secs(15), "the command at the prompt", |face| {
        let screen = face.shell_screen("bob")?;
        let pasted = screen.contains("echo hi");
        let ran = screen.lines().any(|line| line.trim() == "hi");
        (pasted && !ran).then_some(())
    });
    face.send_shell("bob", &["BSpace", "BSpace"]);
    face.send_shell_text("bob", "edited");
    face.send_shell("bob", &["Enter"]);
    let page = face.wait_page("echo edited");
    assert!(page.contains("The shell ran"), "the {shell} page does not show the record:\n{page}");
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
fn push_leaves_several_bash_lines_unrun() {
    push_leaves_several_lines_unrun("bash");
}

#[test]
fn push_leaves_several_zsh_lines_unrun() {
    push_leaves_several_lines_unrun("zsh");
}

#[test]
fn an_edited_bash_push_is_what_the_page_shows() {
    an_edited_push_is_what_the_page_shows("bash");
}

#[test]
fn an_edited_zsh_push_is_what_the_page_shows() {
    an_edited_push_is_what_the_page_shows("zsh");
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

fn explanations_tutorial() -> (TutorialGuard, PathBuf) {
    let parent = std::env::temp_dir().join(format!(
        "tutorial-stepper-face-src-{}-{}",
        std::process::id(),
        unique()
    ));
    let dir = parent.join("fold");
    fs::create_dir_all(&dir).expect("the tutorial directory is created");
    fs::write(
        dir.join("tutorial.toml"),
        "\
pages = [\"welcome.md\", \"later.md\"]

[[window]]
name = \"alice\"
folder = \"alice\"

[[window]]
name = \"bob\"
folder = \"bob\"
",
    )
    .expect("the index is written");
    fs::write(
        dir.join("welcome.md"),
        "\
# Explanation heading

This explanation folds away.

```bash { window=alice }
echo fold-first
```

```text { mode=show }
shown-not-a-command
```

```
echo ordinary-block
```

```bash { }
echo fold-inherited
```

```bash { window=bob }
echo fold-second
```
",
    )
    .expect("the first page is written");
    fs::write(
        dir.join("later.md"),
        "\
# Later heading

Later explanation.

```bash { window=bob }
echo fold-third
```
",
    )
    .expect("the second page is written");
    (TutorialGuard(parent), dir)
}

fn explanations_are_hidden(page: &str) -> bool {
    !page.contains("Explanation heading")
        && !page.contains("This explanation folds away")
        && !page.contains("shown-not-a-command")
        && !page.contains("ordinary-block")
}

fn wait_folded(face: &Face) -> String {
    face.wait_until(Duration::from_secs(15), "the folded page", |face| {
        let socket = face.socket()?;
        let page_id = face.page_id()?;
        let text = face.capture(&socket, &page_id)?;
        (text.contains("f unfolds") && explanations_are_hidden(&text)).then_some(text)
    })
}

#[test]
fn f_hides_the_explanations_and_shows_each_command_with_its_window() {
    let (_keep, tutorial) = explanations_tutorial();
    let face = Face::launch(tutorial, true);
    let full = face.wait_page("Explanation heading");
    assert!(full.contains("This explanation folds away"), "{full}");
    assert!(full.contains("shown-not-a-command"), "{full}");
    assert!(full.contains("ordinary-block"), "{full}");
    assert!(full.contains("fold-first"), "{full}");
    face.send(&["f"]);
    let folded = wait_folded(&face);
    assert!(folded.contains("alice"), "{folded}");
    assert!(folded.contains("fold-first"), "{folded}");
    assert!(folded.contains("fold-inherited"), "{folded}");
    assert!(folded.contains("bob"), "{folded}");
    assert!(folded.contains("fold-second"), "{folded}");
    assert!(folded.contains("next"), "{folded}");
    assert!(face.position().contains("folded = true"), "{}", face.position());
    assert_eq!(face.window_of_page(), "alice");
    face.send(&["f"]);
    let back = face.wait_page("Explanation heading");
    assert!(back.contains("This explanation folds away"), "{back}");
    assert!(back.contains("shown-not-a-command"), "{back}");
    assert!(back.contains("ordinary-block"), "{back}");
    assert!(back.contains("f folds"), "{back}");
    assert!(face.position().contains("folded = false"), "{}", face.position());
}

#[test]
fn folded_stepping_runs_skips_goes_back_and_unfolds() {
    let (_keep, tutorial) = explanations_tutorial();
    let face = Face::launch(tutorial, true);
    face.wait_page("Explanation heading");
    face.send(&["f"]);
    wait_folded(&face);
    let before = face.position();
    face.send(&["e"]);
    face.wait_until(Duration::from_secs(15), "the unrun command at the prompt", |face| {
        let screen = face.shell_screen("alice")?;
        let pasted = screen.contains("echo fold-first");
        let ran = screen.lines().any(|line| line.trim() == "fold-first");
        (pasted && !ran).then_some(())
    });
    assert_eq!(face.position(), before, "e moved the next command");
    let editing = face.wait_page("editing in alice");
    assert!(explanations_are_hidden(&editing), "{editing}");
    face.send_shell("alice", &["Enter"]);
    let after_run = face.wait_until(Duration::from_secs(15), "the inherited command", |face| {
        let socket = face.socket()?;
        let page_id = face.page_id()?;
        let text = face.capture(&socket, &page_id)?;
        (face.position().contains("line = 17\n")
            && text.contains("f unfolds")
            && text.contains("fold-inherited")
            && explanations_are_hidden(&text))
        .then_some(text)
    });
    assert!(face.log("alice").contains("fold-first"), "{}", face.log("alice"));
    assert!(after_run.contains("alice"), "{after_run}");
    face.send(&["s"]);
    let skipped = face.wait_until(Duration::from_secs(10), "the second window's command", |face| {
        let socket = face.socket()?;
        let page_id = face.page_id()?;
        let text = face.capture(&socket, &page_id)?;
        (face.position().contains("line = 21\n")
            && text.contains("f unfolds")
            && text.contains("fold-second")
            && face.window_of_page() == "bob")
            .then_some(text)
    });
    assert!(!face.log("alice").contains("fold-inherited"), "{}", face.log("alice"));
    assert!(skipped.contains("bob"), "{skipped}");
    face.send(&["b"]);
    face.wait_until(Duration::from_secs(10), "the page to move back", |face| {
        let socket = face.socket()?;
        let page_id = face.page_id()?;
        let text = face.capture(&socket, &page_id)?;
        (face.position().contains("line = 17\n")
            && text.contains("f unfolds")
            && text.contains("fold-inherited")
            && explanations_are_hidden(&text)
            && face.window_of_page() == "alice")
            .then_some(())
    });
    face.send(&["f"]);
    let back = face.wait_page("Explanation heading");
    assert!(back.contains("fold-inherited"), "{back}");
    assert!(face.position().contains("folded = false"), "{}", face.position());
}

#[test]
fn the_fold_holds_across_pages_and_across_quitting() {
    let (_keep, tutorial) = explanations_tutorial();
    let face = Face::launch(tutorial, true);
    face.wait_page("Explanation heading");
    face.send(&["f"]);
    wait_folded(&face);
    face.send(&["n"]);
    let later = face.wait_until(Duration::from_secs(15), "the next page folded", |face| {
        let socket = face.socket()?;
        let page_id = face.page_id()?;
        let text = face.capture(&socket, &page_id)?;
        (text.contains("f unfolds")
            && text.contains("fold-third")
            && !text.contains("Later heading")
            && !text.contains("Later explanation"))
        .then_some(text)
    });
    assert!(later.contains("bob"), "{later}");
    let saved = face.position();
    assert!(saved.contains("view = \"later.md\""), "{saved}");
    assert!(saved.contains("welcome.md"), "{saved}");
    assert!(saved.contains("folded = true"), "{saved}");
    assert!(saved.contains("line = 5\n"), "{saved}");
    face.send(&["q"]);
    let page = face.page_id().expect("the page was recorded");
    face.wait_until(Duration::from_secs(10), "the page to close", |face| {
        (!face.pane_open(&page) && face.server_up()).then_some(())
    });
    face.attach_again();
    let resumed =
        face.wait_until(Duration::from_secs(20), "the folded page after resume", |face| {
            let socket = face.socket()?;
            let page_id = face.page_id()?;
            let text = face.capture(&socket, &page_id)?;
            (text.contains("f unfolds")
                && text.contains("fold-third")
                && !text.contains("Later heading"))
            .then_some(text)
        });
    assert!(resumed.contains("bob"), "{resumed}");
    assert!(!face.log("alice").contains("fold-first"), "resume ran the command");
    face.send(&["f"]);
    let full = face.wait_page("Later heading");
    assert!(full.contains("Later explanation"), "{full}");
    assert!(face.position().contains("folded = false"), "{}", face.position());
}

fn observations_tutorial() -> (TutorialGuard, PathBuf) {
    let parent = std::env::temp_dir().join(format!(
        "tutorial-stepper-face-src-{}-{}",
        std::process::id(),
        unique()
    ));
    let dir = parent.join("observe");
    fs::create_dir_all(&dir).expect("the tutorial directory is created");
    fs::write(
        dir.join("tutorial.toml"),
        "\
pages = [\"welcome.md\", \"later.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"

[[window]]
name = \"alice\"
folder = \"alice\"
",
    )
    .expect("the index is written");
    fs::write(
        dir.join("welcome.md"),
        "\
# First page

```bash { window=bob }
echo before-observe
```

```bash { window=bob mode=observe }
echo seen-early
```

```bash { window=bob }
echo after-observe
```
",
    )
    .expect("the first page is written");
    fs::write(
        dir.join("later.md"),
        "\
# Second page

A later explanation.

```bash { window=alice mode=observe }
echo seen-later
```
",
    )
    .expect("the second page is written");
    (TutorialGuard(parent), dir)
}

fn observation_record(face: &Face) -> String {
    fs::read_to_string(face.run_dir().join(".tutorial-stepper/observations.toml"))
        .unwrap_or_default()
}

fn open_observation_list(face: &Face) -> String {
    face.send(&["o"]);
    face.wait_page("o closes the list")
}

fn close_observation_list(face: &Face) {
    face.send(&["o"]);
    face.wait_page("First page");
}

/// The opening fence of `echo before-observe`.
const BEFORE_OBSERVE: &str = "line = 3\n";
/// The opening fence of `echo seen-early`.
const EARLY_OBSERVE: &str = "line = 7\n";
/// The opening fence of `echo after-observe`.
const AFTER_OBSERVE: &str = "line = 11\n";

#[test]
fn the_list_holds_only_observations_already_reached() {
    let (_keep, tutorial) = observations_tutorial();
    let face = Face::launch(tutorial, true);
    let page = face.wait_page("First page");
    assert!(page.contains("o observes"), "{page}");
    let empty = open_observation_list(&face);
    assert!(empty.contains("No observation has been reached yet."), "{empty}");
    assert!(!empty.contains("echo seen-early"), "{empty}");
    assert!(!empty.contains("echo seen-later"), "{empty}");
    assert!(!empty.contains("First page"), "{empty}");
    close_observation_list(&face);
    assert!(face.position().contains(BEFORE_OBSERVE), "{}", face.position());

    face.send(&["s"]);
    face.wait_until(Duration::from_secs(10), "the observation to be next", |face| {
        face.position().contains(EARLY_OBSERVE).then_some(())
    });
    let still_early = open_observation_list(&face);
    assert!(still_early.contains("No observation has been reached yet."), "{still_early}");
    assert!(!still_early.contains("echo seen-early"), "{still_early}");
    close_observation_list(&face);

    face.send(&["s"]);
    face.wait_until(Duration::from_secs(10), "the command after the observation", |face| {
        face.position().contains(AFTER_OBSERVE).then_some(())
    });
    let reached = open_observation_list(&face);
    assert!(reached.contains("echo seen-early"), "{reached}");
    assert!(reached.contains("bob"), "{reached}");
    assert!(!reached.contains("echo seen-later"), "{reached}");
    assert!(!reached.contains("First page"), "{reached}");
    close_observation_list(&face);

    face.send(&["b"]);
    face.wait_until(Duration::from_secs(10), "the observation to be next again", |face| {
        face.position().contains(EARLY_OBSERVE).then_some(())
    });
    let gone = open_observation_list(&face);
    assert!(gone.contains("No observation has been reached yet."), "{gone}");
    assert!(!gone.contains("echo seen-early"), "{gone}");
    close_observation_list(&face);

    face.send(&["s"]);
    face.wait_until(Duration::from_secs(10), "the command after the observation again", |face| {
        face.position().contains(AFTER_OBSERVE).then_some(())
    });
    face.send(&["n"]);
    let turned = face.wait_page("Second page");
    assert!(turned.contains("echo seen-later"), "{turned}");
    let listed = open_observation_list(&face);
    assert!(listed.contains("echo seen-early"), "{listed}");
    assert!(!listed.contains("echo seen-later"), "{listed}");
    assert!(!listed.contains("Second page"), "{listed}");
    assert!(face.position().contains(AFTER_OBSERVE), "{}", face.position());
}

#[test]
fn choosing_an_observation_runs_it_without_moving_the_position() {
    let (_keep, tutorial) = observations_tutorial();
    let face = Face::launch(tutorial, true);
    face.wait_page("First page");
    face.send(&["s"]);
    face.send(&["s"]);
    face.wait_until(Duration::from_secs(10), "the command after the observation", |face| {
        face.position().contains(AFTER_OBSERVE).then_some(())
    });
    let before = face.position();
    let listed = open_observation_list(&face);
    assert!(listed.contains("echo seen-early"), "{listed}");
    face.send(&["1", "Enter"]);
    let ran = face.wait_until(Duration::from_secs(15), "the observation to run", |face| {
        let socket = face.socket()?;
        let page_id = face.page_id()?;
        let text = face.capture(&socket, &page_id)?;
        let record = observation_record(face);
        (text.contains("The shell ran:")
            && text.contains("echo seen-early")
            && face.log("bob").contains("seen-early")
            && record.contains("position = \"welcome.md:11\"")
            && record.contains("[[observation]]"))
        .then_some(text)
    });
    assert_eq!(face.position(), before, "choosing an observation moved the next command");
    assert!(ran.contains("echo after-observe") || face.position().contains(AFTER_OBSERVE), "{ran}");
    assert!(!face.log("alice").contains("seen-later"), "{}", face.log("alice"));
    let again = open_observation_list(&face);
    assert!(again.contains("echo seen-early"), "{again}");
    face.send(&["Enter"]);
    face.wait_page("echo after-observe");
    assert_eq!(face.position(), before, "closing the list moved the next command");
    assert_eq!(observation_record(&face).matches("[[observation]]").count(), 1);
}

#[test]
fn a_busy_window_is_named_and_the_observation_does_not_run() {
    let (_keep, tutorial) = observations_tutorial();
    let face = Face::launch(tutorial, true);
    face.wait_page("First page");
    face.send(&["s"]);
    face.send(&["s"]);
    face.wait_until(Duration::from_secs(10), "the command after the observation", |face| {
        face.position().contains(AFTER_OBSERVE).then_some(())
    });
    let before = face.position();
    face.send_shell_text("bob", "touch started-busy && sleep 30");
    face.send_shell("bob", &["Enter"]);
    face.wait_until(Duration::from_secs(15), "the shell to be inside the command", |face| {
        face.run_dir().join("bob/started-busy").is_file().then_some(())
    });
    let listed = open_observation_list(&face);
    assert!(listed.contains("echo seen-early"), "{listed}");
    face.send(&["1", "Enter"]);
    let page = face.wait_page("window bob is busy");
    assert!(!page.contains("The shell ran:"), "{page}");
    assert!(!face.log("bob").contains("seen-early"), "{}", face.log("bob"));
    assert!(
        !observation_record(&face).contains("[[observation]]"),
        "{}",
        observation_record(&face)
    );
    assert_eq!(face.position(), before, "a busy window moved the next command");
    assert!(face.run_dir().join("bob/started-busy").is_file(), "the command was no longer running");
}

#[test]
fn an_observation_run_twice_is_recorded_at_two_positions() {
    let (_keep, tutorial) = observations_tutorial();
    let face = Face::launch(tutorial, true);
    face.wait_page("First page");
    face.send(&["s"]);
    face.send(&["s"]);
    face.wait_until(Duration::from_secs(10), "the command after the observation", |face| {
        face.position().contains(AFTER_OBSERVE).then_some(())
    });
    open_observation_list(&face);
    face.send(&["1", "Enter"]);
    face.wait_until(Duration::from_secs(15), "the first observation record", |face| {
        let record = observation_record(face);
        (record.contains("[[observation]]") && record.contains("position = \"welcome.md:11\""))
            .then_some(())
    });
    let first = face.position();
    face.send(&["s"]);
    face.send(&["s"]);
    face.wait_until(Duration::from_secs(10), "the tutorial to be finished", |face| {
        face.position().contains("done = true").then_some(())
    });
    assert_ne!(face.position(), first);
    let both = open_observation_list(&face);
    assert!(both.contains("echo seen-early"), "{both}");
    assert!(both.contains("echo seen-later"), "{both}");
    face.send(&["1", "Enter"]);
    let record =
        face.wait_until(Duration::from_secs(15), "the second observation record", |face| {
            let record = observation_record(face);
            (record.matches("[[observation]]").count() == 2
                && record.contains("position = \"done\""))
            .then_some(record)
        });
    assert!(record.contains("position = \"welcome.md:11\""), "{record}");
    assert!(face.position().contains("done = true"), "{}", face.position());
    assert_eq!(face.shown_lines("bob", "seen-early"), 2, "{}", face.log("bob"));
}
