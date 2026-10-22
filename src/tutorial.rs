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
pub(crate) fn script(dir: &Path) -> u8 {
    match load(dir) {
        Ok(loaded) => {
            print!("{}", render(&loaded.blocks));
            0
        }
        Err(problems) => refuse(dir, &problems),
    }
}

/// Play a bash tutorial in the windows it declares. The value is the process exit code.
pub(crate) fn play(dir: &Path, transcript: &Path, run: Option<&Path>, run_setup: bool) -> u8 {
    play_as(dir, transcript, run, run_setup, None)
}

fn play_as(
    dir: &Path,
    transcript: &Path,
    run: Option<&Path>,
    run_setup: bool,
    user: Option<u32>,
) -> u8 {
    let loaded = match load(dir) {
        Ok(loaded) => loaded,
        Err(problems) => return refuse(dir, &problems),
    };
    if let Some(problem) = refuse_scope(&loaded, run_setup) {
        emit(dir, &problem);
        return 1;
    }
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
    if let Some(path) = run {
        if let Some(message) = run_directory_problem(path) {
            eprint!("{message}");
            return 1;
        }
    }
    let windows = loaded
        .windows
        .iter()
        .map(|window| (window.name.clone(), window.folder.clone()))
        .collect::<Vec<_>>();
    let setup = if run_setup {
        loaded.setup.as_ref().map(|(name, _)| (name.clone(), absolute_in(dir, name)))
    } else {
        None
    };
    let setup = setup.as_ref().map(|(name, path)| (name.as_str(), path.as_path()));
    match crate::play::drive(transcript, &loaded.play_commands, &windows, run, setup) {
        Ok(count) => {
            println!("{}", played(count));
            0
        }
        Err(failure) => report(dir, failure),
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
    /// The fence carries `if=`, so this command chooses a setting.
    pub chooses_setting: bool,
}

struct Loaded {
    pages: usize,
    commands: usize,
    blocks: Vec<Block>,
    play_commands: Vec<PlayCommand>,
    shell: index::Shell,
    shell_line: Option<usize>,
    setup: Option<(String, usize)>,
    windows: Vec<index::Window>,
}

struct Block {
    page: String,
    window: Option<String>,
    body: String,
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

    let index::Tutorial { pages, windows, settings, mut problems, shell, shell_line, setup } =
        tutorial;
    let mut read = Read {
        blocks: Vec::new(),
        play_commands: Vec::new(),
        current_window: windows.first().map(|window| window.name.clone()).unwrap_or_default(),
    };
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
        play_commands: read.play_commands,
        shell,
        shell_line,
        setup,
        windows,
    })
}

struct Read {
    blocks: Vec<Block>,
    play_commands: Vec<PlayCommand>,
    current_window: String,
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
            let window = fence.window.clone().unwrap_or_else(|| read.current_window.clone());
            read.current_window.clone_from(&window);
            read.play_commands.push(PlayCommand {
                page: page.path.clone(),
                line: fence.line,
                body: fence.body.clone(),
                expected: fence.status.unwrap_or(0),
                window,
                chooses_setting: fence.setting.is_some(),
            });
        }
        read.blocks.push(Block {
            page: page.path.clone(),
            window: fence.window.clone(),
            body: fence.body.clone(),
        });
    }
    commands
}

fn render(blocks: &[Block]) -> String {
    let mut out = String::from("#!/usr/bin/env bash\n");
    for block in blocks {
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
    if matches!(loaded.shell, index::Shell::Zsh) {
        return Some(Problem {
            relative: "tutorial.toml".to_string(),
            line: loaded.shell_line.unwrap_or(1),
            message: "play --auto runs bash; this tutorial declares zsh".to_string(),
        });
    }
    if !run_setup {
        if let Some((path, line)) = &loaded.setup {
            return Some(Problem {
                relative: "tutorial.toml".to_string(),
                line: *line,
                message: format!("play stops before the set-up script {path}"),
            });
        }
    }
    if let Some(command) = loaded.play_commands.iter().find(|command| command.chooses_setting) {
        return Some(Problem {
            relative: command.page.clone(),
            line: command.line,
            message: "play --auto does not choose a setting".to_string(),
        });
    }
    None
}

fn report(dir: &Path, failure: crate::play::Failure) -> u8 {
    match failure {
        crate::play::Failure::Status { page, line, body, got, expected } => {
            let path = dir.join(page);
            eprint!("{}:{line}: command exited {got}, expected {expected}\n{body}", path.display());
            if !body.ends_with('\n') {
                eprintln!();
            }
            1
        }
        crate::play::Failure::Report(message) => {
            if message.ends_with('\n') {
                eprint!("{message}");
            } else {
                eprintln!("{message}");
            }
            1
        }
    }
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
        let code = play_as(&root, &transcript, Some(&run), true, Some(0));
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
        let code = play_as(&root, &transcript, None, true, Some(1000));
        assert_eq!(code, 0, "a non-root user was refused");
        assert_eq!(fs::read_to_string(&marker).expect("the script wrote the marker"), "ran");
    }
}
