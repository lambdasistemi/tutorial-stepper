//! Read a tutorial directory, report format problems, and print its commands.

use std::fs;
use std::path::Path;

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

struct Loaded {
    pages: usize,
    commands: usize,
    blocks: Vec<Block>,
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

    let index::Tutorial { pages, windows, settings, mut problems } = tutorial;
    let mut commands = 0_usize;
    let mut blocks = Vec::new();
    for page in &pages {
        commands += read_page(dir, page, &windows, &settings, &mut problems, &mut blocks);
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
    Ok(Loaded { pages: pages.len(), commands, blocks })
}

fn read_page(
    dir: &Path,
    page: &index::Page,
    windows: &[index::Window],
    settings: &[index::Setting],
    problems: &mut Vec<Problem>,
    blocks: &mut Vec<Block>,
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
        }
        blocks.push(Block {
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
