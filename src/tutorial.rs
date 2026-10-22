//! Read a tutorial directory and report format problems.

use std::fs;
use std::path::Path;

use crate::index::{self, Problem};

pub(crate) fn check(dir: &Path) -> u8 {
    if !dir.is_dir() {
        let message = if dir.exists() {
            "tutorial directory is not a directory"
        } else {
            "tutorial directory is missing"
        };
        eprintln!("{}:1: {message}", dir.display());
        return 1;
    }

    let index_path = dir.join("tutorial.toml");
    let text = match fs::read_to_string(&index_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("{}:1: tutorial.toml is missing", index_path.display());
            return 1;
        }
        Err(_) => {
            eprintln!("{}:1: cannot read tutorial.toml", index_path.display());
            return 1;
        }
    };

    let tutorial = match index::parse(&text) {
        Ok(tutorial) => tutorial,
        Err(problem) => {
            emit(dir, &problem);
            return 1;
        }
    };

    let index::Tutorial { pages, windows, settings, mut problems } = tutorial;
    let mut commands = 0_usize;
    for page in &pages {
        commands += check_page(dir, page, &windows, &settings, &mut problems);
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
        for problem in &problems {
            emit(dir, problem);
        }
        return 1;
    }

    println!(
        "{}, {}",
        counted(pages.len(), "page", "pages"),
        counted(commands, "command", "commands")
    );
    0
}

fn check_page(
    dir: &Path,
    page: &index::Page,
    windows: &[index::Window],
    settings: &[index::Setting],
    problems: &mut Vec<Problem>,
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
        if fence.mode.is_none() {
            commands += 1;
        }
    }
    commands
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
