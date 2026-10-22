//! Read a tutorial directory, report format problems, and print its commands.

use std::fs;
use std::path::{Path, PathBuf};

use crate::index::{self, Problem};

pub(crate) fn check(dir: &Path) -> u8 {
    match load(dir) {
        Ok(loaded) => {
            println!(
                "{}, {}",
                counted(loaded.pages, "page", "pages"),
                counted(loaded.commands, "command", "commands")
            );
            0
        }
        Err(problems) => refuse(dir, &problems),
    }
}

/// Print the runnable commands as one bash script. The value is the process exit code.
/// `path` selects one tested path. Absent means every branch.
pub(crate) fn script(dir: &Path, path: Option<&str>) -> u8 {
    match load(dir) {
        Ok(loaded) => match path_values(&loaded.paths, path) {
            Ok(values) => {
                print!("{}", render(&loaded.blocks, values.as_deref()));
                0
            }
            Err(problem) => {
                emit(dir, &problem);
                1
            }
        },
        Err(problems) => refuse(dir, &problems),
    }
}

/// Play a tutorial in the shell its index declares. The value is the process exit code.
/// `path` selects one tested path. Absent plays every tested path, or one run on the
/// defaults when the tutorial declares none.
pub(crate) fn play(
    dir: &Path,
    transcript: &Path,
    run: Option<&Path>,
    run_setup: bool,
    path: Option<&str>,
) -> u8 {
    play_as(dir, transcript, run, run_setup, None, path)
}

fn play_as(
    dir: &Path,
    transcript: &Path,
    run: Option<&Path>,
    run_setup: bool,
    user: Option<u32>,
    path: Option<&str>,
) -> u8 {
    let loaded = match load(dir) {
        Ok(loaded) => loaded,
        Err(problems) => return refuse(dir, &problems),
    };
    if let Some(problem) = refuse_scope(&loaded, run_setup) {
        emit(dir, &problem);
        return 1;
    }
    let selected = match path_values(&loaded.paths, path) {
        Ok(values) => values,
        Err(problem) => {
            emit(dir, &problem);
            return 1;
        }
    };
    if run_setup {
        if let Some((name, _)) = &loaded.setup {
            match supplied_user(user) {
                Ok(0) => {
                    eprint!("{}", setup_as_root_message(name));
                    return 1;
                }
                Ok(_) => {}
                Err(message) => {
                    eprint!("{message}");
                    return 1;
                }
            }
        }
    }
    if let Some(message) = unmet_needs(&loaded) {
        eprint!("{message}");
        return 1;
    }
    if let Some(run_dir) = run {
        if let Some(message) = run_directory_problem(run_dir) {
            eprint!("{message}");
            return 1;
        }
    }
    if path.is_none() && !loaded.paths.is_empty() {
        return play_paths(dir, &loaded, transcript, run, run_setup);
    }
    let chosen = values_in_force(&loaded, selected.as_deref());
    match play_one(dir, &loaded, transcript, run, run_setup, path, &chosen) {
        Ok(count) => {
            println!("{}", played(count));
            0
        }
        Err(failure) => report(dir, failure),
    }
}

/// Play each tested path in index order. A failure does not stop the next path.
fn play_paths(
    dir: &Path,
    loaded: &Loaded,
    transcript: &Path,
    run: Option<&Path>,
    run_setup: bool,
) -> u8 {
    let mut failed = false;
    let mut combined = String::new();
    let mut started = false;
    for tested in &loaded.paths {
        let part = transcript_part(&tested.name);
        let _ = fs::remove_file(&part);
        let owned_run = run.map(|parent| parent.join(&tested.name));
        match play_one(
            dir,
            loaded,
            &part,
            owned_run.as_deref(),
            run_setup,
            Some(&tested.name),
            &tested.values,
        ) {
            Ok(count) => println!("{}: {}", tested.name, played(count)),
            Err(failure) => {
                eprint!("{}", prefix_first_line(&tested.name, &failure_text(dir, failure)));
                failed = true;
            }
        }
        if part.is_file() {
            match fs::read_to_string(&part) {
                Ok(text) => {
                    started = true;
                    combined.push_str(&text);
                }
                Err(error) => {
                    eprintln!("cannot read the transcript: {error}");
                    failed = true;
                }
            }
        }
        let _ = fs::remove_file(&part);
    }
    if started && !write_combined(transcript, &combined) {
        return 1;
    }
    if failed {
        1
    } else {
        0
    }
}

fn play_one(
    dir: &Path,
    loaded: &Loaded,
    transcript: &Path,
    run: Option<&Path>,
    run_setup: bool,
    path: Option<&str>,
    chosen: &[(String, String)],
) -> Result<usize, crate::play::Failure> {
    let windows = loaded
        .windows
        .iter()
        .map(|window| (window.name.clone(), window.folder.clone()))
        .collect::<Vec<_>>();
    let initial = windows.first().map(|(name, _)| name.as_str()).unwrap_or("");
    let filter = if loaded.settings.is_empty() { None } else { Some(chosen) };
    let commands = resolve_commands(&loaded.runnables, filter, initial);
    let environment = environment_text(&loaded.settings, chosen);
    let setup = if run_setup {
        loaded.setup.as_ref().map(|(name, _)| (name.clone(), absolute_in(dir, name)))
    } else {
        None
    };
    let setup = setup.as_ref().map(|(name, script)| (name.as_str(), script.as_path()));
    crate::play::drive(
        transcript,
        &commands,
        &windows,
        run,
        setup,
        loaded.shell,
        crate::play::DriveExtras { environment: environment.as_deref(), path },
    )
}

fn values_in_force(
    loaded: &Loaded,
    selected: Option<&[(String, String)]>,
) -> Vec<(String, String)> {
    match selected {
        Some(values) => values.to_vec(),
        None => loaded
            .settings
            .iter()
            .map(|setting| (setting.name.clone(), setting.default.clone()))
            .collect(),
    }
}

fn transcript_part(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static PARTS: AtomicU64 = AtomicU64::new(0);
    let n = PARTS.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("tutorial-stepper-{n}-{name}.toml"))
}

fn write_combined(transcript: &Path, text: &str) -> bool {
    if let Some(parent) = transcript.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = fs::create_dir_all(parent) {
                eprintln!("cannot write the transcript: {error}");
                return false;
            }
        }
    }
    if let Err(error) = fs::write(transcript, text) {
        eprintln!("cannot write the transcript: {error}");
        return false;
    }
    true
}

fn prefix_first_line(name: &str, text: &str) -> String {
    match text.split_once('\n') {
        Some((first, rest)) => format!("{name}: {first}\n{rest}"),
        None => format!("{name}: {text}"),
    }
}

/// One command fence: no `mode`, so it is typed into the shell.
pub(crate) struct PlayCommand {
    /// Page path, as listed in the index.
    pub page: String,
    /// 1-based line of the opening fence.
    pub line: usize,
    /// Fence body, as written.
    pub body: String,
    /// Status the block declares. Absent means 0.
    pub expected: u8,
    /// Window this command runs in.
    pub window: String,
}

struct Runnable {
    page: String,
    line: usize,
    body: String,
    expected: u8,
    declared_window: Option<String>,
    setting: Option<(String, String)>,
}

struct Loaded {
    pages: usize,
    commands: usize,
    blocks: Vec<Block>,
    runnables: Vec<Runnable>,
    shell: index::Shell,
    setup: Option<(String, usize)>,
    windows: Vec<index::Window>,
    settings: Vec<index::Setting>,
    paths: Vec<index::TestedPath>,
}

struct Block {
    page: String,
    window: Option<String>,
    body: String,
    setting: Option<(String, String)>,
}

fn load(dir: &Path) -> Result<Loaded, Vec<Problem>> {
    if !dir.is_dir() {
        let message = if dir.exists() {
            "tutorial directory is not a directory"
        } else {
            "tutorial directory is missing"
        };
        return Err(vec![Problem {
            relative: String::new(),
            line: 1,
            message: message.to_string(),
        }]);
    }

    let index_path = dir.join("tutorial.toml");
    let text = match fs::read_to_string(&index_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(vec![Problem {
                relative: "tutorial.toml".to_string(),
                line: 1,
                message: "tutorial.toml is missing".to_string(),
            }]);
        }
        Err(_) => {
            return Err(vec![Problem {
                relative: "tutorial.toml".to_string(),
                line: 1,
                message: "cannot read tutorial.toml".to_string(),
            }]);
        }
    };

    let tutorial = match index::parse(&text) {
        Ok(tutorial) => tutorial,
        Err(problem) => return Err(vec![problem]),
    };

    let index::Tutorial { pages, windows, settings, paths, mut problems, shell, setup } = tutorial;
    let mut read = Read { blocks: Vec::new(), runnables: Vec::new() };
    let mut commands = 0_usize;
    for page in &pages {
        commands += read_page(dir, page, &windows, &settings, &mut problems, &mut read);
    }
    match markdown_pages(dir) {
        Ok(found) => {
            for relative in found {
                if pages.iter().any(|page| page.path == relative) {
                    continue;
                }
                problems.push(Problem {
                    relative: relative.clone(),
                    line: 1,
                    message: format!("{relative} is not listed in tutorial.toml"),
                });
            }
        }
        Err(()) => problems.push(Problem {
            relative: String::new(),
            line: 1,
            message: "cannot read the tutorial directory".to_string(),
        }),
    }

    if !problems.is_empty() {
        return Err(problems);
    }
    Ok(Loaded {
        pages: pages.len(),
        commands,
        blocks: read.blocks,
        runnables: read.runnables,
        shell,
        setup,
        windows,
        settings,
        paths,
    })
}

struct Read {
    blocks: Vec<Block>,
    runnables: Vec<Runnable>,
}

fn read_page(
    dir: &Path,
    page: &index::Page,
    windows: &[index::Window],
    settings: &[index::Setting],
    problems: &mut Vec<Problem>,
    read: &mut Read,
) -> usize {
    let path = dir.join(&page.path);
    if !path.is_file() {
        problems.push(Problem {
            relative: "tutorial.toml".to_string(),
            line: page.line,
            message: format!("{} is listed but missing", page.path),
        });
        return 0;
    }
    let markdown = match fs::read_to_string(&path) {
        Ok(markdown) => markdown,
        Err(_) => {
            problems.push(Problem {
                relative: page.path.clone(),
                line: 1,
                message: "cannot read this page".to_string(),
            });
            return 0;
        }
    };
    let (fences, errors) = crate::fence::fence_attributes(&markdown);
    for error in errors {
        problems.push(Problem {
            relative: page.path.clone(),
            line: error.line,
            message: error.message,
        });
    }
    let mut commands = 0;
    for fence in &fences {
        if let Some(window) = &fence.window {
            if !windows.iter().any(|known| known.name == *window) {
                problems.push(Problem {
                    relative: page.path.clone(),
                    line: fence.line,
                    message: format!("window \"{window}\" is not declared"),
                });
            }
        }
        if let Some((name, value)) = &fence.setting {
            match settings.iter().find(|setting| setting.name == *name) {
                None => problems.push(Problem {
                    relative: page.path.clone(),
                    line: fence.line,
                    message: format!("setting \"{name}\" is not declared"),
                }),
                Some(setting) if !setting.values.iter().any(|allowed| allowed == value) => {
                    problems.push(Problem {
                        relative: page.path.clone(),
                        line: fence.line,
                        message: format!("value \"{value}\" is not allowed for {name}"),
                    });
                }
                Some(_) => {}
            }
        }
        if fence.mode == Some(crate::fence::Mode::Show) {
            continue;
        }
        if fence.mode.is_none() {
            commands += 1;
            read.runnables.push(Runnable {
                page: page.path.clone(),
                line: fence.line,
                body: fence.body.clone(),
                expected: fence.status.unwrap_or(0),
                declared_window: fence.window.clone(),
                setting: fence.setting.clone(),
            });
        }
        read.blocks.push(Block {
            page: page.path.clone(),
            window: fence.window.clone(),
            body: fence.body.clone(),
            setting: fence.setting.clone(),
        });
    }
    commands
}

fn path_values(
    paths: &[index::TestedPath],
    path: Option<&str>,
) -> Result<Option<Vec<(String, String)>>, Problem> {
    let Some(name) = path else {
        return Ok(None);
    };
    match paths.iter().find(|candidate| candidate.name == name) {
        Some(found) => Ok(Some(found.values.clone())),
        None => Err(Problem {
            relative: "tutorial.toml".to_string(),
            line: 1,
            message: format!("path \"{name}\" is not a tested path"),
        }),
    }
}

fn belongs(setting: &Option<(String, String)>, values: Option<&[(String, String)]>) -> bool {
    let Some(values) = values else {
        return true;
    };
    match setting {
        None => true,
        Some((name, value)) => values.iter().any(|(key, chosen)| key == name && chosen == value),
    }
}

fn render(blocks: &[Block], values: Option<&[(String, String)]>) -> String {
    let mut out = String::from("#!/usr/bin/env bash\n");
    for block in blocks {
        if !belongs(&block.setting, values) {
            continue;
        }
        out.push('\n');
        match &block.window {
            Some(window) => {
                out.push_str("# ");
                out.push_str(&block.page);
                out.push_str(", window ");
                out.push_str(window);
                out.push('\n');
            }
            None => {
                out.push_str("# ");
                out.push_str(&block.page);
                out.push_str(", window not named\n");
            }
        }
        out.push_str(&block.body);
        if !block.body.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn refuse(dir: &Path, problems: &[Problem]) -> u8 {
    for problem in problems {
        emit(dir, problem);
    }
    1
}

fn markdown_pages(dir: &Path) -> Result<Vec<String>, ()> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = fs::read_dir(&current).map_err(|_| ())?;
        for entry in entries {
            let entry = entry.map_err(|_| ())?;
            let file_type = entry.file_type().map_err(|_| ())?;
            let path = entry.path();
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file()
                && path.extension().is_some_and(|extension| extension == "md")
            {
                found.push(relative_name(dir, &path));
            }
        }
    }
    found.sort();
    Ok(found)
}

fn relative_name(dir: &Path, path: &Path) -> String {
    path.strip_prefix(dir)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn run_directory_problem(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }
    if !path.is_dir() {
        return Some(format!("{} is not a directory\n", path.display()));
    }
    match fs::read_dir(path) {
        Ok(mut entries) => entries.next().map(|_| format!("{} is not empty\n", path.display())),
        Err(error) => Some(format!("cannot read {}: {error}\n", path.display())),
    }
}

fn setup_as_root_message(script: &str) -> String {
    format!("play does not run the set-up script {script} as root\n")
}

fn supplied_user(user: Option<u32>) -> Result<u32, String> {
    if let Some(user) = user {
        return Ok(user);
    }
    current_user_id()
}

fn current_user_id() -> Result<u32, String> {
    let output = std::process::Command::new("id")
        .arg("-u")
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|error| format!("cannot read the user id: {error}\n"))?;
    if !output.status.success() {
        return Err("cannot read the user id\n".to_string());
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|_| "cannot read the user id\n".to_string())
}

fn unmet_needs(loaded: &Loaded) -> Option<String> {
    let needs = loaded
        .windows
        .iter()
        .flat_map(|window| window.needs.iter())
        .map(|need| (need.program.as_str(), need.minimum.as_deref()))
        .collect::<Vec<_>>();
    crate::play::unmet_need(&needs)
}

fn absolute_in(dir: &Path, relative: &str) -> PathBuf {
    let joined = dir.join(relative);
    if joined.is_absolute() {
        return joined;
    }
    std::env::current_dir().map(|cwd| cwd.join(joined)).unwrap_or(dir.join(relative))
}

fn refuse_scope(loaded: &Loaded, run_setup: bool) -> Option<Problem> {
    if !run_setup {
        if let Some((script, line)) = &loaded.setup {
            return Some(Problem {
                relative: "tutorial.toml".to_string(),
                line: *line,
                message: format!("play stops before the set-up script {script}"),
            });
        }
    }
    None
}

fn resolve_commands(
    commands: &[Runnable],
    values: Option<&[(String, String)]>,
    initial: &str,
) -> Vec<PlayCommand> {
    let mut current = initial.to_string();
    let mut resolved = Vec::new();
    for command in commands {
        if !belongs(&command.setting, values) {
            continue;
        }
        if let Some(window) = &command.declared_window {
            current.clone_from(window);
        }
        resolved.push(PlayCommand {
            page: command.page.clone(),
            line: command.line,
            body: command.body.clone(),
            expected: command.expected,
            window: current.clone(),
        });
    }
    resolved
}

fn environment_text(settings: &[index::Setting], values: &[(String, String)]) -> Option<String> {
    if settings.is_empty() {
        return None;
    }
    let mut text = String::new();
    for setting in settings {
        let value = values
            .iter()
            .find(|(name, _)| name == &setting.name)
            .map(|(_, value)| value.as_str())
            .unwrap_or(setting.default.as_str());
        text.push_str(&setting.name);
        text.push('=');
        text.push_str(value);
        text.push('\n');
    }
    Some(text)
}

fn failure_text(dir: &Path, failure: crate::play::Failure) -> String {
    match failure {
        crate::play::Failure::Status { page, line, body, got, expected } => {
            let path = dir.join(page);
            let mut text = format!(
                "{}:{line}: command exited {got}, expected {expected}\n{body}",
                path.display()
            );
            if !body.ends_with('\n') {
                text.push('\n');
            }
            text
        }
        crate::play::Failure::Report(message) => {
            if message.ends_with('\n') {
                message
            } else {
                format!("{message}\n")
            }
        }
    }
}

fn report(dir: &Path, failure: crate::play::Failure) -> u8 {
    eprint!("{}", failure_text(dir, failure));
    1
}

fn played(count: usize) -> String {
    if count == 1 {
        "1 command played".to_string()
    } else {
        format!("{count} commands played")
    }
}

fn counted(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

fn emit(dir: &Path, problem: &Problem) {
    let path =
        if problem.relative.is_empty() { dir.to_path_buf() } else { dir.join(&problem.relative) };
    eprintln!("{}:{}: {}", path.display(), problem.line, problem.message);
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::play_as;

    fn temp(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("tutorial-stepper-{name}"));
        let _ = fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn the_set_up_script_is_refused_when_the_user_is_root() {
        let root = temp("root-setup");
        fs::create_dir_all(&root).expect("the tutorial directory");
        fs::write(
            root.join("tutorial.toml"),
            "\
setup = \"setup.sh\"
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
",
        )
        .expect("the index");
        fs::write(root.join("welcome.md"), "```bash { window=bob }\necho hi\n```\n")
            .expect("the page");
        let marker = root.join("marker");
        fs::write(root.join("setup.sh"), format!("printf ran > {}\n", marker.display()))
            .expect("the set-up script");
        let run = temp("root-setup-run");
        let transcript = root.join("transcript.toml");
        let code = play_as(&root, &transcript, Some(&run), true, Some(0), None);
        assert_eq!(code, 1, "play ran the set-up script as root");
        let message = super::setup_as_root_message("setup.sh");
        assert!(message.contains("setup.sh"), "{message}");
        assert!(message.contains("as root"), "{message}");
        assert!(!marker.exists(), "the set-up script ran");
        assert!(!transcript.exists(), "a refused run wrote a transcript");
        assert!(!run.exists(), "a refused run created the run directory");
    }

    #[test]
    fn a_set_up_script_still_runs_for_a_user_who_is_not_root() {
        let root = temp("user-setup");
        fs::create_dir_all(&root).expect("the tutorial directory");
        fs::write(
            root.join("tutorial.toml"),
            "\
setup = \"setup.sh\"
pages = [\"welcome.md\"]

[[window]]
name = \"bob\"
folder = \"bob\"
",
        )
        .expect("the index");
        fs::write(root.join("welcome.md"), "```bash { window=bob }\necho hi\n```\n")
            .expect("the page");
        let marker = root.join("marker");
        fs::write(root.join("setup.sh"), format!("printf ran > {}\n", marker.display()))
            .expect("the set-up script");
        let transcript = root.join("transcript.toml");
        let code = play_as(&root, &transcript, None, true, Some(1000), None);
        assert_eq!(code, 0, "a non-root user was refused");
        assert_eq!(fs::read_to_string(&marker).expect("the script wrote the marker"), "ran");
    }
}
