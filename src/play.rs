//! One bash shell on a private tmux server, and the transcript of what it ran.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, Value};

use crate::tutorial::PlayCommand;

/// Why a play stopped.
#[derive(Debug)]
pub(crate) enum Failure {
    /// A command's status was not the one the block declares.
    Status {
        /// Page path, as listed in the index.
        page: String,
        /// 1-based line of the opening fence.
        line: usize,
        /// Fence body, as written.
        body: String,
        /// Status the prompt hook reported.
        got: u8,
        /// Status the block declares.
        expected: u8,
    },
    /// The programs, the shell, or the transcript could not be used.
    Report(String),
}

struct Entry {
    page: String,
    line: usize,
    command: String,
    status: u8,
    start: String,
    end: String,
    output: String,
}

struct Session {
    tmux: PathBuf,
    socket: String,
    run_dir: PathBuf,
    window_dir: PathBuf,
    hook_path: PathBuf,
    launcher_path: PathBuf,
    status_path: PathBuf,
    log_path: PathBuf,
    marker: String,
}

/// Play `commands` in `folder` and write `transcript`. The count is how many ran.
pub(crate) fn drive(
    transcript: &Path,
    commands: &[PlayCommand],
    folder: &str,
) -> Result<usize, Failure> {
    let tmux = require_program("tmux")?;
    let bash = require_program("bash")?;
    ensure_transcript_directory(transcript)?;
    let session = Session::prepare(tmux, folder)?;
    session.start(&bash)?;
    session.run(commands, transcript)
}

fn require_program(name: &str) -> Result<PathBuf, Failure> {
    find_program(name).ok_or_else(|| Failure::Report(format!("{name} is not on PATH\n")))
}

fn find_program(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(name);
        let executable = fs::metadata(&candidate)
            .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
        if executable {
            return Some(candidate);
        }
    }
    None
}

fn ensure_transcript_directory(transcript: &Path) -> Result<(), Failure> {
    match transcript.parent() {
        None => Ok(()),
        Some(parent) if parent.as_os_str().is_empty() || parent.is_dir() => Ok(()),
        Some(parent) => Err(Failure::Report(format!(
            "cannot write the transcript: {} is missing\n",
            parent.display()
        ))),
    }
}

impl Session {
    fn prepare(tmux: PathBuf, folder: &str) -> Result<Self, Failure> {
        let suffix = unique_suffix();
        let run_dir = std::env::temp_dir().join(format!("tutorial-stepper-run-{suffix}"));
        fs::create_dir_all(&run_dir).map_err(|error| {
            Failure::Report(format!("cannot create the run directory: {error}\n"))
        })?;
        let session = Self {
            tmux,
            socket: format!("tutorial-stepper-{suffix}"),
            window_dir: run_dir.join(folder),
            hook_path: run_dir.join("hook"),
            launcher_path: run_dir.join("launch"),
            status_path: run_dir.join("status"),
            log_path: run_dir.join("log"),
            marker: format!("tutorial-stepper-prompt-{suffix}"),
            run_dir,
        };
        fs::create_dir_all(&session.window_dir).map_err(|error| {
            Failure::Report(format!("cannot create the window folder: {error}\n"))
        })?;
        Ok(session)
    }

    fn start(&self, bash: &Path) -> Result<(), Failure> {
        self.write_shell(bash)?;
        self.spawn()?;
        // The first status line is the shell becoming ready, not a command.
        self.wait_for_status_lines(1)?;
        self.open_log()
    }

    fn run(&self, commands: &[PlayCommand], transcript: &Path) -> Result<usize, Failure> {
        let mut entries = Vec::new();
        for command in commands {
            let log_from = fs::metadata(&self.log_path).map(|meta| meta.len()).unwrap_or(0);
            let status_from = complete_lines(&self.read_status()).len();
            let start = format_utc(SystemTime::now());
            self.type_command(&command.body)?;
            self.wait_for_status_lines(status_from + 1)?;
            let end = format_utc(SystemTime::now());
            let got = self.latest_status()?;
            let output = self.capture_output(log_from)?;
            let failed = got != command.expected;
            entries.push(Entry {
                page: command.page.clone(),
                line: command.line,
                command: command.body.clone(),
                status: got,
                start,
                end,
                output,
            });
            if failed {
                write_transcript(transcript, &entries)?;
                return Err(Failure::Status {
                    page: command.page.clone(),
                    line: command.line,
                    body: command.body.clone(),
                    got,
                    expected: command.expected,
                });
            }
        }
        write_transcript(transcript, &entries)?;
        Ok(entries.len())
    }

    fn write_shell(&self, bash: &Path) -> Result<(), Failure> {
        fs::write(&self.hook_path, HOOK)
            .map_err(|error| Failure::Report(format!("cannot prepare the shell: {error}\n")))?;
        let launcher = format!(
            "#!/bin/sh\nunset BASH_ENV\nexport TUTORIAL_STEPPER_STATUS={status}\nexport TUTORIAL_STEPPER_PROMPT={prompt}\nexec {bash} --noprofile --rcfile {hook}\n",
            status = sh_single(&self.status_path.display().to_string()),
            prompt = sh_single(&self.marker),
            bash = sh_single(&bash.display().to_string()),
            hook = sh_single(&self.hook_path.display().to_string()),
        );
        fs::write(&self.launcher_path, launcher)
            .map_err(|error| Failure::Report(format!("cannot prepare the shell: {error}\n")))?;
        let mut permissions = fs::metadata(&self.launcher_path)
            .map_err(|error| Failure::Report(format!("cannot prepare the shell: {error}\n")))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&self.launcher_path, permissions)
            .map_err(|error| Failure::Report(format!("cannot prepare the shell: {error}\n")))
    }

    fn spawn(&self) -> Result<(), Failure> {
        let output = Command::new(&self.tmux)
            .args(["-f", "/dev/null", "-L", &self.socket, "new-session", "-d", "-s", "play", "-c"])
            .arg(&self.window_dir)
            .arg(&self.launcher_path)
            .env_remove("TMUX")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| Failure::Report(format!("cannot run tmux: {error}\n")))?;
        if output.status.success() {
            Ok(())
        } else {
            let detail = String::from_utf8_lossy(&output.stderr);
            Err(Failure::Report(format!("tmux new-session failed: {detail}\n")))
        }
    }

    fn open_log(&self) -> Result<(), Failure> {
        let target = format!("cat >> {}", sh_single(&self.log_path.display().to_string()));
        self.tmux_ok(&["pipe-pane", "-o", "-t", "play:0.0", &target])
    }

    fn type_command(&self, body: &str) -> Result<(), Failure> {
        // One trailing newline is the fence. Enter after the paste submits it.
        let typed = body.strip_suffix('\n').unwrap_or(body);
        let output = Command::new(&self.tmux)
            .args(["-f", "/dev/null", "-L", &self.socket, "send-keys", "-l", "-t", "play:0.0"])
            .arg("\u{1b}[200~")
            .arg(typed)
            .arg("\u{1b}[201~")
            .env_remove("TMUX")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| Failure::Report(format!("cannot run tmux: {error}\n")))?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr);
            return Err(Failure::Report(format!("tmux send-keys failed: {detail}\n")));
        }
        self.tmux_ok(&["send-keys", "-t", "play:0.0", "Enter"])
    }

    fn wait_for_status_lines(&self, count: usize) -> Result<(), Failure> {
        loop {
            if complete_lines(&self.read_status()).len() >= count {
                return Ok(());
            }
            if !self.pane_alive() {
                return Err(shell_died());
            }
            thread::sleep(POLL);
        }
    }

    fn latest_status(&self) -> Result<u8, Failure> {
        let text = self.read_status();
        let lines = complete_lines(&text);
        let line = lines.last().copied().ok_or_else(shell_died)?;
        line.parse::<u8>().map_err(|_| {
            Failure::Report(format!("the bash shell reported {line:?}, which is not a status\n"))
        })
    }

    fn capture_output(&self, log_from: u64) -> Result<String, Failure> {
        let start = usize::try_from(log_from).unwrap_or(usize::MAX);
        loop {
            let bytes = fs::read(&self.log_path).unwrap_or_default();
            let slice = if start <= bytes.len() { &bytes[start..] } else { &bytes[..] };
            if let Some(output) = command_output(slice, &self.marker) {
                return Ok(output);
            }
            if !self.pane_alive() {
                return Err(shell_died());
            }
            thread::sleep(POLL);
        }
    }

    fn read_status(&self) -> String {
        fs::read_to_string(&self.status_path).unwrap_or_default()
    }

    fn pane_alive(&self) -> bool {
        Command::new(&self.tmux)
            .args(["-f", "/dev/null", "-L", &self.socket, "has-session", "-t", "play"])
            .env_remove("TMUX")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn tmux_ok(&self, args: &[&str]) -> Result<(), Failure> {
        let output = Command::new(&self.tmux)
            .arg("-f")
            .arg("/dev/null")
            .arg("-L")
            .arg(&self.socket)
            .args(args)
            .env_remove("TMUX")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| Failure::Report(format!("cannot run tmux: {error}\n")))?;
        if output.status.success() {
            Ok(())
        } else {
            let detail = String::from_utf8_lossy(&output.stderr);
            let name = args.first().copied().unwrap_or("command");
            Err(Failure::Report(format!("tmux {name} failed: {detail}\n")))
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = Command::new(&self.tmux)
            .args(["-f", "/dev/null", "-L", &self.socket, "kill-server"])
            .env_remove("TMUX")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if fs::remove_dir_all(&self.run_dir).is_err() {
            thread::sleep(Duration::from_millis(50));
            let _ = fs::remove_dir_all(&self.run_dir);
        }
    }
}

// No `-i`: that flag makes this bash exit before the hook runs. The pane is a
// terminal, so the shell is interactive without it. The hook appends `$?`
// before bash draws PS1, which is why the output wait follows the status line.
const HOOK: &str = r#"
PROMPT_COMMAND='status=$?; printf "%s\n" "$status" >> "$TUTORIAL_STEPPER_STATUS"'
PS1=$'\n'"$TUTORIAL_STEPPER_PROMPT"$'\n'
PS2=
stty -echo
bind 'set enable-bracketed-paste on'
unset HISTFILE
"#;

const POLL: Duration = Duration::from_millis(30);

fn shell_died() -> Failure {
    Failure::Report("the bash shell exited before the command finished\n".to_string())
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{}-{nanos}", std::process::id())
}

fn sh_single(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn complete_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    for line in text.split_inclusive('\n') {
        if let Some(stripped) = line.strip_suffix('\n') {
            if !stripped.is_empty() {
                lines.push(stripped);
            }
        }
    }
    lines
}

/// Output is the new log text with the prompt marker removed.
/// Bash wraps a command in bracketed-paste mode switches; those are not output.
/// Carriage-return newline pairs become newlines. A lone carriage return stays.
fn command_output(bytes: &[u8], marker: &str) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let crlf = format!("\r\n{marker}\r\n");
    let lf = format!("\n{marker}\n");
    let head = if let Some(index) = text.find(&crlf) {
        &text[..index]
    } else {
        let index = text.find(&lf)?;
        &text[..index]
    };
    Some(strip_paste_mode(head).replace("\r\n", "\n"))
}

/// Drop the bracketed-paste switches bash writes around a command.
/// The switches are a prefix and a suffix. A command that prints the same
/// bytes in the middle keeps them.
fn strip_paste_mode(text: &str) -> &str {
    const OFF: &str = "\u{1b}[?2004l";
    const ON: &str = "\u{1b}[?2004h";
    let Some(after_off) = text.strip_prefix(OFF) else {
        return text.strip_suffix(ON).unwrap_or(text);
    };
    let after_off = after_off
        .strip_prefix("\r\r\n")
        .or_else(|| after_off.strip_prefix("\r\n"))
        .or_else(|| after_off.strip_prefix('\n'))
        .unwrap_or(after_off);
    after_off.strip_suffix(ON).unwrap_or(after_off)
}

fn write_transcript(path: &Path, entries: &[Entry]) -> Result<(), Failure> {
    if entries.is_empty() {
        return fs::write(path, "")
            .map_err(|error| Failure::Report(format!("cannot write the transcript: {error}\n")));
    }
    let mut tables = ArrayOfTables::new();
    for entry in entries {
        let mut table = Table::new();
        table.insert("page", item_str(&entry.page));
        table.insert("block", item_int(i64::try_from(entry.line).unwrap_or(0)));
        table.insert("command", item_str(&entry.command));
        table.insert("status", item_int(i64::from(entry.status)));
        table.insert("start", item_str(&entry.start));
        table.insert("end", item_str(&entry.end));
        table.insert("output", item_str(&entry.output));
        tables.push(table);
    }
    let mut document = DocumentMut::new();
    document.insert("command", Item::ArrayOfTables(tables));
    let mut rendered = document.to_string();
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    fs::write(path, rendered)
        .map_err(|error| Failure::Report(format!("cannot write the transcript: {error}\n")))
}

fn item_str(text: &str) -> Item {
    Item::Value(Value::from(text))
}

fn item_int(value: i64) -> Item {
    Item::Value(Value::from(value))
}

/// UTC `YYYY-MM-DDTHH:MM:SS.NNNNNNNNNZ`, fixed width 30.
fn format_utc(time: SystemTime) -> String {
    let since = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let days = i64::try_from(since.as_secs() / 86_400).unwrap_or(0);
    let time_of_day = since.as_secs() % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = time_of_day / 3_600;
    let minute = (time_of_day % 3_600) / 60;
    let second = time_of_day % 60;
    let nanos = since.subsec_nanos();
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{nanos:09}Z")
}

/// Howard Hinnant's civil-from-days, days since the Unix epoch.
fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    year += i64::from(month <= 2);
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_epoch_formats_as_a_fixed_width_utc_time() {
        assert_eq!(format_utc(UNIX_EPOCH), "1970-01-01T00:00:00.000000000Z");
        assert_eq!(
            format_utc(UNIX_EPOCH + Duration::from_secs(86_400)),
            "1970-01-02T00:00:00.000000000Z"
        );
        assert_eq!(
            format_utc(UNIX_EPOCH + Duration::from_secs(946_684_800)),
            "2000-01-01T00:00:00.000000000Z"
        );
        assert_eq!(format_utc(UNIX_EPOCH).len(), 30);
    }

    #[test]
    fn command_output_drops_the_prompt_and_carriage_returns() {
        assert_eq!(command_output(b"hi\r\n\r\nMARKER\r\n", "MARKER").as_deref(), Some("hi\n"));
        assert_eq!(command_output(b"\r\nMARKER\r\n", "MARKER").as_deref(), Some(""));
        assert_eq!(
            command_output(b"hello world\r\n\r\nMARKER\r\n", "MARKER").as_deref(),
            Some("hello world\n")
        );
        assert_eq!(command_output(b"a\rb\r\n\r\nMARKER\r\n", "MARKER").as_deref(), Some("a\rb\n"));
        assert_eq!(command_output(b"hi\r\n", "MARKER"), None);
        let wrapped = b"\x1b[?2004l\r\r\nhi\r\n\x1b[?2004h\r\nMARKER\r\n";
        assert_eq!(command_output(wrapped, "MARKER").as_deref(), Some("hi\n"));
        let quiet = b"\x1b[?2004l\r\r\n\x1b[?2004h\r\nMARKER\r\n";
        assert_eq!(command_output(quiet, "MARKER").as_deref(), Some(""));
    }

    #[test]
    fn an_empty_run_writes_an_empty_transcript() {
        let path = std::env::temp_dir()
            .join(format!("tutorial-stepper-empty-transcript-{}", std::process::id()));
        write_transcript(&path, &[]).expect("the transcript is written");
        let text = fs::read_to_string(&path).expect("the transcript can be read");
        let _ = fs::remove_file(&path);
        assert_eq!(text, "");
    }
}
