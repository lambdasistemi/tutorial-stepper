//! One shell per window on a private tmux server, and the transcript of what ran.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, Value};

use crate::tutorial::{PlayKind, PlayStep};

/// What one run writes into the windows besides the commands.
pub(crate) struct DriveExtras<'a> {
    /// The `.env` text. Absent when the tutorial declares no settings.
    pub environment: Option<&'a str>,
    /// Tested path name recorded on each transcript command. Absent when the run is not a path.
    pub path: Option<&'a str>,
}

/// Why a play stopped.
#[derive(Debug)]
pub(crate) enum Failure {
    /// A block's status was not the one it declares.
    Status {
        /// Page path, as listed in the index.
        page: String,
        /// 1-based line of the opening fence.
        line: usize,
        /// The line the shell accepted.
        body: String,
        /// Status the prompt hook reported.
        got: u8,
        /// Status the block declares.
        expected: u8,
        /// Command, observation where it appears, or observation at the end.
        kind: PlayKind,
    },
    /// The programs, the shell, or the transcript could not be used.
    Report(String),
}

struct Entry {
    path: Option<String>,
    page: String,
    line: usize,
    window: String,
    command: String,
    status: u8,
    start: String,
    end: String,
    output: String,
    kind: PlayKind,
    position: Option<String>,
}

struct Pane {
    name: String,
    /// tmux pane id of the shell. Empty until the window exists.
    id: String,
    dir: PathBuf,
    status_path: PathBuf,
    log_path: PathBuf,
    launcher_path: PathBuf,
}

/// One private tmux server and the shells it is stepping.
pub(crate) struct Session {
    tmux: PathBuf,
    socket: String,
    run_dir: PathBuf,
    keep_run: bool,
    /// When set, dropping the session leaves the server and its control files.
    retain: bool,
    control_dir: PathBuf,
    panes: Vec<Pane>,
    marker: String,
    shell_name: String,
    env_path: Option<PathBuf>,
    path_name: Option<String>,
}

/// What one step's shell reported. The unattended run and the terminal face both get it from [`Session::run_step`].
pub(crate) struct StepOutcome {
    /// Status the prompt hook reported.
    pub status: u8,
    /// Text the block wrote, without the prompt.
    pub output: String,
    /// UTC time just before the block was typed.
    pub start: String,
    /// UTC time when the hook reported the end.
    pub end: String,
    /// The line the shell accepted, ending in a newline.
    pub command: String,
}

/// What the face wants while a command is still running.
pub(crate) enum Wait {
    /// Keep waiting for the prompt hook.
    Continue,
    /// Send Ctrl-C to the shell, then keep waiting for the hook.
    Interrupt,
}

/// Play `steps` in `windows` and write `transcript`. The count is how many commands ran.
/// Each window is a name and its folder relative to the run folder.
/// `shell` is the shell the index declares.
/// `extras.environment`, when present, is written to `.env` before any window opens.
pub(crate) fn drive(
    transcript: &Path,
    steps: &[PlayStep],
    windows: &[(String, String)],
    run: Option<&Path>,
    setup: Option<(&str, &Path)>,
    shell: crate::index::Shell,
    extras: DriveExtras<'_>,
) -> Result<usize, Failure> {
    if windows.is_empty() {
        return Err(Failure::Report("a tutorial needs at least one window\n".to_string()));
    }
    let tmux = require_program("tmux")?;
    let shell_program = match shell {
        crate::index::Shell::Bash => "bash",
        crate::index::Shell::Zsh => "zsh",
    };
    let shell_path = require_program(shell_program)?;
    ensure_transcript_directory(transcript)?;
    let mut session = Session::prepare(tmux, windows, run, shell_program, &extras, None)?;
    if let Some((name, script)) = setup {
        let bash = if matches!(shell, crate::index::Shell::Bash) {
            shell_path.clone()
        } else {
            require_program("bash")?
        };
        run_setup(&bash, &session.run_dir, name, script)?;
    }
    session.start(&shell_path, shell)?;
    session.run(steps, transcript)
}

/// Run the set-up script in the run folder, as the user who started play.
pub(crate) fn run_setup(
    bash: &Path,
    run_dir: &Path,
    name: &str,
    script: &Path,
) -> Result<(), Failure> {
    if !script.is_file() {
        return Err(Failure::Report(format!("the set-up script {name} is not a file\n")));
    }
    let output = Command::new(bash)
        .arg(script)
        .current_dir(run_dir)
        .env_remove("TMUX")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            Failure::Report(format!("cannot run the set-up script {name}: {error}\n"))
        })?;
    if output.status.success() {
        return Ok(());
    }
    let status = output
        .status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "no status".to_string());
    Err(Failure::Report(format!("the set-up script {name} exited {status}\n")))
}

pub(crate) fn require_program(name: &str) -> Result<PathBuf, Failure> {
    find_program(name).ok_or_else(|| Failure::Report(format!("{name} is not on PATH\n")))
}

/// The first need that is not met, in the order given. A need with no minimum is presence only.
pub(crate) fn unmet_need(needs: &[(&str, Option<&str>)]) -> Option<String> {
    for (program, minimum) in needs {
        let Some(path) = find_program(program) else {
            return Some(format!("{program} was not found\n"));
        };
        let Some(required) = minimum else {
            continue;
        };
        let text = match version_text(&path) {
            Ok(text) => text,
            Err(message) => return Some(message),
        };
        let Some(found) = first_dotted_number(&text) else {
            return Some(format!("{program} printed no version\n"));
        };
        let Some(required_number) = first_dotted_number(required) else {
            return Some(format!(
                "{program}'s required version {required} is not a dotted number\n"
            ));
        };
        if is_older(found, required_number) {
            return Some(format!("{program} {found} is older than {required}\n"));
        }
    }
    None
}

fn version_text(path: &Path) -> Result<String, String> {
    let output = Command::new(path)
        .arg("--version")
        .env_remove("TMUX")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("cannot run {}: {error}\n", path.display()))?;
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(combined)
}

/// The first `[0-9]+(\.[0-9]+)+` in `text`.
fn first_dotted_number(text: &str) -> Option<&str> {
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

fn is_older(found: &str, required: &str) -> bool {
    let found = version_components(found);
    let required = version_components(required);
    let width = found.len().max(required.len());
    for index in 0..width {
        let left = found.get(index).copied().unwrap_or(0);
        let right = required.get(index).copied().unwrap_or(0);
        if left != right {
            return left < right;
        }
    }
    false
}

fn version_components(version: &str) -> Vec<u64> {
    version.split('.').map(|part| part.parse().unwrap_or(0)).collect()
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
    fn prepare(
        tmux: PathBuf,
        windows: &[(String, String)],
        run: Option<&Path>,
        shell_name: &str,
        extras: &DriveExtras<'_>,
        control: Option<&Path>,
    ) -> Result<Self, Failure> {
        let suffix = unique_suffix();
        let keep_run = run.is_some();
        let run_dir = match run {
            Some(path) => path.to_path_buf(),
            None => std::env::temp_dir().join(format!("tutorial-stepper-run-{suffix}")),
        };
        let control_dir = match control {
            Some(path) => path.to_path_buf(),
            None => std::env::temp_dir().join(format!("tutorial-stepper-control-{suffix}")),
        };
        if let Err(error) = fs::create_dir_all(&run_dir) {
            return Err(Failure::Report(format!("cannot create the run directory: {error}\n")));
        }
        if let Err(error) = fs::create_dir_all(&control_dir) {
            if !keep_run {
                remove_tree(&run_dir);
            }
            return Err(Failure::Report(format!("cannot create the run directory: {error}\n")));
        }
        let env_path = if let Some(text) = extras.environment {
            let path = run_dir.join(".env");
            if let Err(error) = fs::write(&path, text) {
                if !keep_run {
                    remove_tree(&run_dir);
                }
                remove_tree(&control_dir);
                return Err(Failure::Report(format!("cannot write .env: {error}\n")));
            }
            Some(absolute(&path))
        } else {
            None
        };
        let mut panes = Vec::new();
        for (name, folder) in windows {
            let dir = run_dir.join(folder);
            if let Err(error) = fs::create_dir_all(&dir) {
                if !keep_run {
                    remove_tree(&run_dir);
                }
                remove_tree(&control_dir);
                return Err(Failure::Report(format!("cannot create the window folder: {error}\n")));
            }
            panes.push(Pane {
                name: name.clone(),
                id: String::new(),
                dir,
                status_path: control_dir.join(format!("status-{name}")),
                log_path: control_dir.join(format!("log-{name}")),
                launcher_path: control_dir.join(format!("launch-{name}")),
            });
        }
        Ok(Self {
            tmux,
            socket: format!("tutorial-stepper-{suffix}"),
            marker: format!("tutorial-stepper-prompt-{suffix}"),
            run_dir,
            keep_run,
            retain: false,
            control_dir,
            panes,
            shell_name: shell_name.to_string(),
            env_path,
            path_name: extras.path.map(str::to_string),
        })
    }

    fn start(&mut self, shell: &Path, kind: crate::index::Shell) -> Result<(), Failure> {
        self.write_hook(kind)?;
        for index in 0..self.panes.len() {
            self.write_launcher(index, shell, kind)?;
        }
        self.spawn_all()?;
        for index in 0..self.panes.len() {
            let target = pane_target(&self.panes[index].name);
            self.panes[index].id =
                self.tmux_text(&["display-message", "-p", "-t", &target, "#{pane_id}"])?;
        }
        for index in 0..self.panes.len() {
            // The first status line is the shell becoming ready, not a command.
            self.wait_for_status_lines(index, 1)?;
            self.open_log(index)?;
        }
        Ok(())
    }

    fn run(&self, steps: &[PlayStep], transcript: &Path) -> Result<usize, Failure> {
        let mut entries = Vec::new();
        let mut commands_run = 0_usize;
        let mut last_command: Option<(String, usize)> = None;
        for step in steps {
            let outcome = self.run_step(step)?;
            let failed = outcome.status != step.expected;
            let position = match step.kind {
                PlayKind::Command => None,
                PlayKind::Observation => Some(match &last_command {
                    Some((page, line)) => format!("{page}:{line}"),
                    None => "start of path".to_string(),
                }),
                PlayKind::ObservationAtEnd => Some("end of path".to_string()),
            };
            let command = outcome.command;
            entries.push(Entry {
                path: self.path_name.clone(),
                page: step.page.clone(),
                line: step.line,
                window: step.window.clone(),
                command: command.clone(),
                status: outcome.status,
                start: outcome.start,
                end: outcome.end,
                output: outcome.output,
                kind: step.kind,
                position,
            });
            if step.kind == PlayKind::Command {
                commands_run += 1;
                last_command = Some((step.page.clone(), step.line));
            }
            if failed {
                write_transcript(transcript, &entries)?;
                return Err(Failure::Status {
                    page: step.page.clone(),
                    line: step.line,
                    body: command,
                    got: outcome.status,
                    expected: step.expected,
                    kind: step.kind,
                });
            }
        }
        write_transcript(transcript, &entries)?;
        Ok(commands_run)
    }

    fn pane_index(&self, name: &str) -> Result<usize, Failure> {
        self.panes
            .iter()
            .position(|pane| pane.name == name)
            .ok_or_else(|| Failure::Report(format!("window \"{name}\" has no shell\n")))
    }

    fn write_hook(&self, kind: crate::index::Shell) -> Result<(), Failure> {
        let (path, text) = match kind {
            crate::index::Shell::Bash => (self.control_dir.join("hook"), BASH_HOOK),
            crate::index::Shell::Zsh => {
                let dir = self.control_dir.join("zdot");
                fs::create_dir_all(&dir).map_err(|error| {
                    Failure::Report(format!("cannot prepare the shell: {error}\n"))
                })?;
                (dir.join(".zshrc"), ZSH_HOOK)
            }
        };
        fs::write(path, text)
            .map_err(|error| Failure::Report(format!("cannot prepare the shell: {error}\n")))
    }

    fn env_export(&self) -> String {
        match &self.env_path {
            Some(path) => {
                format!("export TUTORIAL_STEPPER_ENV={}\n", sh_single(&path.display().to_string()))
            }
            None => String::new(),
        }
    }

    fn write_launcher(
        &self,
        index: usize,
        shell: &Path,
        kind: crate::index::Shell,
    ) -> Result<(), Failure> {
        let pane = &self.panes[index];
        let status = sh_single(&pane.status_path.display().to_string());
        let command = sh_single(&command_path(&pane.status_path, &pane.name).display().to_string());
        let prompt = sh_single(&self.marker);
        let window = sh_single(&pane.name);
        let program = sh_single(&shell.display().to_string());
        let environment = self.env_export();
        // Bash reads HISTFILE before the hook runs, so the empty target has to
        // be in the environment the shell starts with. The hook then unsets it.
        let launcher = match kind {
            crate::index::Shell::Bash => {
                let hook = sh_single(&self.control_dir.join("hook").display().to_string());
                format!(
                    "#!/bin/sh\nunset BASH_ENV\n{environment}export HISTFILE=/dev/null\nexport TUTORIAL_STEPPER_STATUS={status}\nexport TUTORIAL_STEPPER_COMMAND={command}\nexport TUTORIAL_STEPPER_PROMPT={prompt}\nexport TUTORIAL_STEPPER_WINDOW={window}\nexec {program} --noprofile --rcfile {hook}\n"
                )
            }
            crate::index::Shell::Zsh => {
                let zdot = sh_single(&self.control_dir.join("zdot").display().to_string());
                format!(
                    "#!/bin/sh\nunset ENV\n{environment}export HISTFILE=/dev/null\nexport TUTORIAL_STEPPER_STATUS={status}\nexport TUTORIAL_STEPPER_COMMAND={command}\nexport TUTORIAL_STEPPER_PROMPT={prompt}\nexport TUTORIAL_STEPPER_WINDOW={window}\nexport ZDOTDIR={zdot}\nexec {program} --no-globalrcs\n"
                )
            }
        };
        fs::write(&pane.launcher_path, launcher)
            .map_err(|error| Failure::Report(format!("cannot prepare the shell: {error}\n")))?;
        let mut permissions = fs::metadata(&pane.launcher_path)
            .map_err(|error| Failure::Report(format!("cannot prepare the shell: {error}\n")))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&pane.launcher_path, permissions)
            .map_err(|error| Failure::Report(format!("cannot prepare the shell: {error}\n")))
    }

    fn spawn_all(&self) -> Result<(), Failure> {
        let first = &self.panes[0];
        let output = Command::new(&self.tmux)
            .args(["-f", "/dev/null", "-L", &self.socket, "new-session", "-d", "-s", "play", "-n"])
            .arg(&first.name)
            .arg("-c")
            .arg(&first.dir)
            .arg(&first.launcher_path)
            .env_remove("TMUX")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| Failure::Report(format!("cannot run tmux: {error}\n")))?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr);
            return Err(Failure::Report(format!("tmux new-session failed: {detail}\n")));
        }
        for pane in self.panes.iter().skip(1) {
            let output = Command::new(&self.tmux)
                .args([
                    "-f",
                    "/dev/null",
                    "-L",
                    &self.socket,
                    "new-window",
                    "-d",
                    "-t",
                    "play",
                    "-n",
                ])
                .arg(&pane.name)
                .arg("-c")
                .arg(&pane.dir)
                .arg(&pane.launcher_path)
                .env_remove("TMUX")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .map_err(|error| Failure::Report(format!("cannot run tmux: {error}\n")))?;
            if !output.status.success() {
                let detail = String::from_utf8_lossy(&output.stderr);
                return Err(Failure::Report(format!("tmux new-window failed: {detail}\n")));
            }
        }
        Ok(())
    }

    fn open_log(&self, index: usize) -> Result<(), Failure> {
        let pane = &self.panes[index];
        let target = self.shell_target(index);
        let pipe = format!("cat >> {}", sh_single(&pane.log_path.display().to_string()));
        self.tmux_ok(&["pipe-pane", "-o", "-t", &target, &pipe])
    }

    /// Place `body` at the prompt as one bracketed paste.
    ///
    /// One trailing newline belongs to the fence, not to the line. `submit`
    /// presses Enter. A push leaves the paste unrun.
    fn paste(&self, index: usize, body: &str, submit: bool) -> Result<(), Failure> {
        let typed = body.strip_suffix('\n').unwrap_or(body);
        let target = self.shell_target(index);
        let output = Command::new(&self.tmux)
            .args(["-f", "/dev/null", "-L", &self.socket, "send-keys", "-l", "-t"])
            .arg(&target)
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
        if submit {
            self.tmux_ok(&["send-keys", "-t", &target, "Enter"])?;
        }
        Ok(())
    }

    fn wait_for_status_lines(&self, index: usize, count: usize) -> Result<(), Failure> {
        let window = self.panes[index].name.clone();
        loop {
            if complete_lines(&self.read_status(index)).len() >= count {
                return Ok(());
            }
            if !self.pane_alive(index) {
                return Err(self.shell_died(&window));
            }
            thread::sleep(POLL);
        }
    }

    fn latest_status(&self, index: usize) -> Result<u8, Failure> {
        let window = self.panes[index].name.clone();
        let text = self.read_status(index);
        let lines = complete_lines(&text);
        let line = lines.last().copied().ok_or_else(|| self.shell_died(&window))?;
        line.parse::<u8>().map_err(|_| {
            Failure::Report(format!(
                "the {} shell in window {window} reported {line:?}, which is not a status\n",
                self.shell_name
            ))
        })
    }

    fn capture_output(&self, index: usize, log_from: u64) -> Result<String, Failure> {
        let start = usize::try_from(log_from).unwrap_or(usize::MAX);
        let window = self.panes[index].name.clone();
        loop {
            let bytes = fs::read(&self.panes[index].log_path).unwrap_or_default();
            let slice = if start <= bytes.len() { &bytes[start..] } else { &bytes[..] };
            if let Some(output) = command_output(slice, &self.marker) {
                return Ok(output);
            }
            if !self.pane_alive(index) {
                return Err(self.shell_died(&window));
            }
            thread::sleep(POLL);
        }
    }

    fn read_status(&self, index: usize) -> String {
        fs::read_to_string(&self.panes[index].status_path).unwrap_or_default()
    }

    fn pane_alive(&self, index: usize) -> bool {
        let target = self.shell_target(index);
        let output = Command::new(&self.tmux)
            .args([
                "-f",
                "/dev/null",
                "-L",
                &self.socket,
                "display-message",
                "-t",
                &target,
                "-p",
                "#{pane_dead}",
            ])
            .env_remove("TMUX")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        output.is_ok_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "0"
        })
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

/// A session the terminal face can reopen without starting or killing it.
pub(crate) struct SessionRecord {
    /// Absolute path of `tmux`.
    pub tmux: PathBuf,
    /// Socket name (`tmux -L`).
    pub socket: String,
    /// Prompt marker the shells were given.
    pub marker: String,
    /// `bash` or `zsh`.
    pub shell_name: String,
    /// The run folder.
    pub run_dir: PathBuf,
    /// Hook, status and log files.
    pub control_dir: PathBuf,
    /// One entry per window: name, directory, status file, log file.
    pub panes: Vec<PaneRecord>,
}

/// One window in a [`SessionRecord`].
pub(crate) struct PaneRecord {
    /// Window name.
    pub name: String,
    /// The window's directory.
    pub dir: PathBuf,
    /// Status file the prompt hook appends to.
    pub status_path: PathBuf,
    /// Pane log.
    pub log_path: PathBuf,
    /// tmux pane id of the shell, for example `%3`.
    pub id: String,
}

impl Session {
    /// Start the shells and keep the server when the caller asks. The control
    /// directory is `run/.tutorial-stepper/control`, replacing any earlier one.
    pub(crate) fn start_kept(
        tmux: PathBuf,
        windows: &[(String, String)],
        run: &Path,
        shell: crate::index::Shell,
        environment: Option<&str>,
    ) -> Result<Self, Failure> {
        let control = run.join(".tutorial-stepper").join("control");
        if control.exists() {
            remove_tree(&control);
        }
        let shell_name = match shell {
            crate::index::Shell::Bash => "bash",
            crate::index::Shell::Zsh => "zsh",
        };
        let shell_path = require_program(shell_name)?;
        let mut session = Self::prepare(
            tmux,
            windows,
            Some(run),
            shell_name,
            &DriveExtras { environment, path: None },
            Some(&control),
        )?;
        session.start(&shell_path, shell)?;
        Ok(session)
    }

    /// Leave the server running when this value is dropped.
    pub(crate) fn retain(&mut self) {
        self.retain = true;
    }

    /// The socket name.
    pub(crate) fn socket(&self) -> &str {
        &self.socket
    }

    /// Absolute path of `tmux`.
    pub(crate) fn tmux(&self) -> &Path {
        &self.tmux
    }

    /// Enough to reopen this session from another process.
    pub(crate) fn record(&self) -> SessionRecord {
        SessionRecord {
            tmux: self.tmux.clone(),
            socket: self.socket.clone(),
            marker: self.marker.clone(),
            shell_name: self.shell_name.clone(),
            run_dir: self.run_dir.clone(),
            control_dir: self.control_dir.clone(),
            panes: self
                .panes
                .iter()
                .map(|pane| PaneRecord {
                    name: pane.name.clone(),
                    dir: pane.dir.clone(),
                    status_path: pane.status_path.clone(),
                    log_path: pane.log_path.clone(),
                    id: pane.id.clone(),
                })
                .collect(),
        }
    }

    /// Reopen a session. Dropping the result does not kill the server.
    pub(crate) fn reopen(record: SessionRecord) -> Self {
        let panes = record
            .panes
            .into_iter()
            .map(|pane| Pane {
                launcher_path: record.control_dir.join(format!("launch-{}", pane.name)),
                name: pane.name,
                id: pane.id,
                dir: pane.dir,
                status_path: pane.status_path,
                log_path: pane.log_path,
            })
            .collect();
        Self {
            tmux: record.tmux,
            socket: record.socket,
            run_dir: record.run_dir,
            keep_run: true,
            retain: true,
            control_dir: record.control_dir,
            panes,
            marker: record.marker,
            shell_name: record.shell_name,
            env_path: None,
            path_name: None,
        }
    }

    /// Whether `pane` still exists.
    ///
    /// `display-message` exits 0 for a pane id that is already gone, so the
    /// check is a pane listing, which reports a missing pane.
    pub(crate) fn pane_open(&self, pane: &str) -> bool {
        self.tmux_output(&["list-panes", "-t", pane, "-F", "#{pane_id}"])
            .is_ok_and(|output| output.status.success())
    }

    /// The pane's width in columns, or 80 when tmux cannot say.
    pub(crate) fn pane_width(&self, pane: &str) -> usize {
        self.tmux_text(&["display-message", "-p", "-t", pane, "#{pane_width}"])
            .ok()
            .and_then(|text| text.parse().ok())
            .filter(|width| *width > 0)
            .unwrap_or(80)
    }

    /// Open the page as a pane to the left of `window`, running `command`.
    pub(crate) fn split_page(&self, window: &str, command: &str) -> Result<(), Failure> {
        let target = self.shell_in(window)?;
        self.tmux_ok(&["split-window", "-h", "-b", "-l", "45%", "-t", &target, command])
    }

    /// Put `pane` to the left of `window` and select it.
    pub(crate) fn place_page(&self, pane: &str, window: &str) -> Result<(), Failure> {
        let target = self.shell_in(window)?;
        let current = self.tmux_text(&["display-message", "-p", "-t", pane, "#{window_name}"])?;
        if current != window {
            self.tmux_ok(&["join-pane", "-h", "-b", "-l", "45%", "-s", pane, "-t", &target])?;
        }
        self.tmux_ok(&["select-window", "-t", &pane_target(window)])?;
        self.tmux_ok(&["select-pane", "-t", pane])?;
        Ok(())
    }

    /// Type `step` and wait until that shell's prompt hook reports the end.
    pub(crate) fn run_step(
        &self,
        step: &crate::tutorial::PlayStep,
    ) -> Result<StepOutcome, Failure> {
        self.run_step_until(step, || Wait::Continue)
    }

    /// [`Session::run_step`], calling `poll` while the shell has not finished.
    /// [`Wait::Interrupt`] sends Ctrl-C once; the end is still the prompt hook.
    pub(crate) fn run_step_until(
        &self,
        step: &crate::tutorial::PlayStep,
        mut poll: impl FnMut() -> Wait,
    ) -> Result<StepOutcome, Failure> {
        self.drive_step(step, true, &mut poll)
    }

    /// Paste `step` without Enter, select its shell, and wait for the learner's Enter.
    ///
    /// The page keeps waiting through a prompt redrawn by Ctrl-C. The end is
    /// still the next status line, and the recorded command is the line the
    /// shell accepted.
    pub(crate) fn push_step(
        &self,
        step: &crate::tutorial::PlayStep,
    ) -> Result<StepOutcome, Failure> {
        self.drive_step(step, false, &mut || Wait::Continue)
    }

    fn drive_step(
        &self,
        step: &crate::tutorial::PlayStep,
        submit: bool,
        poll: &mut impl FnMut() -> Wait,
    ) -> Result<StepOutcome, Failure> {
        let index = self.pane_index(&step.window)?;
        let log_from =
            fs::metadata(&self.panes[index].log_path).map(|meta| meta.len()).unwrap_or(0);
        let status_from = complete_lines(&self.read_status(index)).len();
        let start = format_utc(SystemTime::now());
        self.paste(index, &step.body, submit)?;
        if !submit {
            let target = self.shell_target(index);
            self.tmux_ok(&["select-pane", "-t", &target])?;
        }
        self.wait_until(index, status_from + 1, poll)?;
        let end = format_utc(SystemTime::now());
        let status = self.latest_status(index)?;
        let output = self.capture_output(index, log_from)?;
        let command = self.recorded_command(index)?;
        Ok(StepOutcome { status, output, start, end, command })
    }

    /// The line written beside the status file, with one trailing newline.
    ///
    /// The hook writes it from a shell variable after the command, immediately
    /// before the status line, so this is read only once that line is complete.
    fn recorded_command(&self, index: usize) -> Result<String, Failure> {
        let pane = &self.panes[index];
        let path = command_path(&pane.status_path, &pane.name);
        let mut text = fs::read_to_string(&path).map_err(|_| {
            Failure::Report(format!(
                "the {} shell in window {} reported no command\n",
                self.shell_name, pane.name
            ))
        })?;
        if !text.ends_with('\n') {
            text.push('\n');
        }
        Ok(text)
    }

    fn wait_until(
        &self,
        index: usize,
        count: usize,
        poll: &mut impl FnMut() -> Wait,
    ) -> Result<(), Failure> {
        let window = self.panes[index].name.clone();
        let mut interrupted = false;
        loop {
            if complete_lines(&self.read_status(index)).len() >= count {
                return Ok(());
            }
            if !interrupted && matches!(poll(), Wait::Interrupt) {
                interrupted = true;
                self.send_interrupt(index)?;
            }
            if !self.pane_alive(index) {
                return Err(self.shell_died(&window));
            }
            thread::sleep(POLL);
        }
    }

    fn send_interrupt(&self, index: usize) -> Result<(), Failure> {
        let target = self.shell_target(index);
        self.tmux_ok(&["send-keys", "-t", &target, "C-c"])
    }

    /// The shell pane. A window target would be the page once that pane is selected.
    fn shell_target(&self, index: usize) -> String {
        let pane = &self.panes[index];
        if pane.id.is_empty() {
            pane_target(&pane.name)
        } else {
            pane.id.clone()
        }
    }

    fn shell_in(&self, window: &str) -> Result<String, Failure> {
        let index = self.pane_index(window)?;
        Ok(self.shell_target(index))
    }

    fn tmux_output(&self, args: &[&str]) -> Result<std::process::Output, Failure> {
        Command::new(&self.tmux)
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
            .map_err(|error| Failure::Report(format!("cannot run tmux: {error}\n")))
    }

    fn tmux_text(&self, args: &[&str]) -> Result<String, Failure> {
        let output = self.tmux_output(args)?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let detail = String::from_utf8_lossy(&output.stderr);
            let name = args.first().copied().unwrap_or("command");
            Err(Failure::Report(format!("tmux {name} failed: {detail}\n")))
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.retain {
            let _ = Command::new(&self.tmux)
                .args(["-f", "/dev/null", "-L", &self.socket, "kill-server"])
                .env_remove("TMUX")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            remove_tree(&self.control_dir);
        }
        if !self.keep_run {
            remove_tree(&self.run_dir);
        }
    }
}

fn pane_target(name: &str) -> String {
    format!("play:={name}")
}

fn command_path(status_path: &Path, name: &str) -> PathBuf {
    status_path.with_file_name(format!("command-{name}"))
}

fn remove_tree(path: &Path) {
    if fs::remove_dir_all(path).is_err() {
        thread::sleep(Duration::from_millis(50));
        let _ = fs::remove_dir_all(path);
    }
}

// No `-i`: that flag makes bash exit before the hook runs. The pane is a
// terminal, so the shell is interactive without it. The environment file is
// sourced before the prompt hook, so those assignments are not commands.
// The accepted line is kept in a shell variable and written beside the status
// file after the command, immediately before the status line: a file written
// at accept time could be replaced by the command itself. A prompt redrawn
// by Ctrl-C at the prompt writes no status, so cancelling an edit does not
// look like a finished command. The first prompt is still the shell becoming
// ready. zsh reads the editing buffer, because an empty Enter does not call
// preexec. History expansion is off. The launcher points HISTFILE at
// /dev/null before the shell starts, and the hook unsets it so nothing is
// written. The visible prompt is the window's name. The marker is a terminal
// title: the pane log records it and the screen does not show it. The title
// is where the output is cut.
const BASH_HOOK: &str = r#"
if [ -n "${TUTORIAL_STEPPER_ENV:-}" ]; then
  set -a
  . "$TUTORIAL_STEPPER_ENV"
  set +a
fi
TUTORIAL_STEPPER_ACCEPTED=
tutorial_stepper_accept_line() {
  TUTORIAL_STEPPER_LINE=$READLINE_LINE
  TUTORIAL_STEPPER_ACCEPTED=1
}
bind -x '"\C-x\C-a": tutorial_stepper_accept_line'
bind '"\C-m": "\C-x\C-a\C-j"'
tutorial_stepper_prompt() {
  local status=$1
  if [ -n "$TUTORIAL_STEPPER_ACCEPTED" ] || [ ! -s "$TUTORIAL_STEPPER_STATUS" ]; then
    if [ -n "$TUTORIAL_STEPPER_ACCEPTED" ]; then
      printf '%s' "$TUTORIAL_STEPPER_LINE" > "$TUTORIAL_STEPPER_COMMAND"
      TUTORIAL_STEPPER_ACCEPTED=
    fi
    printf '%s\n' "$status" >> "$TUTORIAL_STEPPER_STATUS"
  fi
  printf '\033]0;%s\007' "$TUTORIAL_STEPPER_PROMPT"
}
PROMPT_COMMAND='tutorial_stepper_prompt $?'
PS1="$TUTORIAL_STEPPER_WINDOW"'$ '
PS2=
bind 'set enable-bracketed-paste on'
set +o histexpand
unset HISTFILE
"#;

const ZSH_HOOK: &str = r#"
if [ -n "${TUTORIAL_STEPPER_ENV:-}" ]; then
  set -a
  . "$TUTORIAL_STEPPER_ENV"
  set +a
fi
TUTORIAL_STEPPER_ACCEPTED=
tutorial_stepper_accept_line() {
  TUTORIAL_STEPPER_LINE=$BUFFER
  TUTORIAL_STEPPER_ACCEPTED=1
  zle .accept-line
}
zle -N accept-line tutorial_stepper_accept_line
precmd() {
  local exit_status=$?
  if [[ -n $TUTORIAL_STEPPER_ACCEPTED ]] || [[ ! -s $TUTORIAL_STEPPER_STATUS ]]; then
    if [[ -n $TUTORIAL_STEPPER_ACCEPTED ]]; then
      printf '%s' "$TUTORIAL_STEPPER_LINE" > "$TUTORIAL_STEPPER_COMMAND"
      TUTORIAL_STEPPER_ACCEPTED=
    fi
    print -r -- "$exit_status" >> "$TUTORIAL_STEPPER_STATUS"
  fi
  print -n -- $'\e]0;'"$TUTORIAL_STEPPER_PROMPT"$'\a'
}
PS1="$TUTORIAL_STEPPER_WINDOW"'$ '
PS2=
unset RPS1
unsetopt prompt_cr prompt_sp banghist
unset HISTFILE
setopt no_beep
zle_highlight=(none)
"#;

const POLL: Duration = Duration::from_millis(30);

impl Session {
    fn shell_died(&self, window: &str) -> Failure {
        Failure::Report(format!(
            "the {} shell in window {window} exited before the command finished\n",
            self.shell_name
        ))
    }
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{}-{nanos}", std::process::id())
}

pub(crate) fn sh_single(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir().map(|cwd| cwd.join(path)).unwrap_or_else(|_| path.to_path_buf())
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

/// Output is the new log text, cut at the prompt marker.
/// The marker is a terminal title (`ESC ] 0 ; marker BEL`). The screen does
/// not show it. The shell turns bracketed paste off before the command's
/// output, so text echoed before that switch, including the typed command,
/// is not part of the output. Carriage-return newline pairs become newlines.
/// A lone carriage return stays.
fn command_output(bytes: &[u8], marker: &str) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let index = text.find(marker)?;
    let head = text[..index].strip_suffix("\u{1b}]0;").unwrap_or(&text[..index]);
    Some(strip_paste_mode(head).replace("\r\n", "\n"))
}

/// Drop the shell's bracketed-paste switches and zsh's prompt redraw.
/// The off switch follows the typed command. A command that prints the same
/// bytes in the middle keeps them. bash writes one carriage return after the
/// off switch; zsh writes a carriage return, a carriage return and a newline.
fn strip_paste_mode(text: &str) -> &str {
    const OFF: &str = "\u{1b}[?2004l";
    const ON: &str = "\u{1b}[?2004h";
    const CLEAR: &str = "\u{1b}[0m\u{1b}[27m\u{1b}[24m\u{1b}[J";
    let after_off = if let Some(index) = text.find(OFF) {
        let rest = &text[index + OFF.len()..];
        rest.strip_prefix("\r\r\n")
            .or_else(|| rest.strip_prefix("\r\n"))
            .or_else(|| rest.strip_prefix('\n'))
            .or_else(|| rest.strip_prefix('\r'))
            .unwrap_or(rest)
    } else {
        text
    };
    let trimmed = after_off.strip_suffix(ON).unwrap_or(after_off);
    trimmed.strip_suffix(CLEAR).unwrap_or(trimmed)
}

fn write_transcript(path: &Path, entries: &[Entry]) -> Result<(), Failure> {
    if entries.is_empty() {
        return fs::write(path, "")
            .map_err(|error| Failure::Report(format!("cannot write the transcript: {error}\n")));
    }
    let mut rendered = String::new();
    for entry in entries {
        rendered.push_str(&render_entry(entry));
    }
    fs::write(path, rendered)
        .map_err(|error| Failure::Report(format!("cannot write the transcript: {error}\n")))
}

/// One table. Tables are written one at a time so a command and an observation
/// stay in the order they ran. One document would group every table of one key.
fn render_entry(entry: &Entry) -> String {
    let mut table = Table::new();
    if let Some(path) = &entry.path {
        table.insert("path", item_str(path));
    }
    table.insert("page", item_str(&entry.page));
    table.insert("window", item_str(&entry.window));
    table.insert("block", item_int(i64::try_from(entry.line).unwrap_or(0)));
    table.insert("command", item_str(&entry.command));
    table.insert("status", item_int(i64::from(entry.status)));
    table.insert("start", item_str(&entry.start));
    table.insert("end", item_str(&entry.end));
    table.insert("output", item_str(&entry.output));
    if let Some(position) = &entry.position {
        table.insert("position", item_str(position));
    }
    let key = match entry.kind {
        PlayKind::Command => "command",
        PlayKind::Observation | PlayKind::ObservationAtEnd => "observation",
    };
    let mut tables = ArrayOfTables::new();
    tables.push(table);
    let mut document = DocumentMut::new();
    document.insert(key, Item::ArrayOfTables(tables));
    let mut rendered = document.to_string();
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
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
    fn command_output_drops_the_prompt_and_the_typed_command() {
        let bash = "\u{1b}[7mecho hi\u{1b}[27m\r\u{1b}[C\u{1b}[C\u{1b}[C\u{1b}[C\u{1b}[Cecho hi\r\n\u{1b}[?2004l\rhi\r\n\u{1b}]0;MARKER\u{7}\u{1b}[?2004hbob$ ";
        assert_eq!(command_output(bash.as_bytes(), "MARKER").as_deref(), Some("hi\n"));
        let quiet = "\u{1b}[7mtrue\u{1b}[27m\u{8}\u{8}\u{8}\u{8}true\r\n\u{1b}[?2004l\r\u{1b}]0;MARKER\u{7}\u{1b}[?2004hbob$ ";
        assert_eq!(command_output(quiet.as_bytes(), "MARKER").as_deref(), Some(""));
        let words = "\u{1b}[?2004l\rhello world\r\n\u{1b}]0;MARKER\u{7}";
        assert_eq!(command_output(words.as_bytes(), "MARKER").as_deref(), Some("hello world\n"));
        let cr = "\u{1b}[?2004l\ra\rb\r\n\u{1b}]0;MARKER\u{7}";
        assert_eq!(command_output(cr.as_bytes(), "MARKER").as_deref(), Some("a\rb\n"));
        assert_eq!(command_output(b"hi\r\n", "MARKER"), None);
    }

    #[test]
    fn command_output_drops_the_zsh_prompt_redraw() {
        let hi = "echo hi\u{1b}[?2004l\r\r\nhi\r\n\u{1b}]0;MARKER\u{7}\u{1b}[0m\u{1b}[27m\u{1b}[24m\u{1b}[Jbob$ \u{1b}[K\u{1b}[?2004h";
        assert_eq!(command_output(hi.as_bytes(), "MARKER").as_deref(), Some("hi\n"));
        let quiet = "false\u{1b}[?2004l\r\r\n\u{1b}]0;MARKER\u{7}\u{1b}[0m\u{1b}[27m\u{1b}[24m\u{1b}[Jbob$ \u{1b}[K\u{1b}[?2004h";
        assert_eq!(command_output(quiet.as_bytes(), "MARKER").as_deref(), Some(""));
        let lines =
            "echo one\r\r\necho two\u{1b}[K\u{1b}[?2004l\r\r\none\r\ntwo\r\n\u{1b}]0;MARKER\u{7}";
        assert_eq!(command_output(lines.as_bytes(), "MARKER").as_deref(), Some("one\ntwo\n"));
        let colour = "printf\u{1b}[?2004l\r\r\n\u{1b}[31mred\u{1b}[0m\r\n\u{1b}]0;MARKER\u{7}";
        assert_eq!(
            command_output(colour.as_bytes(), "MARKER").as_deref(),
            Some("\u{1b}[31mred\u{1b}[0m\n")
        );
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
