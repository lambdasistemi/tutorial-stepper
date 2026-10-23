//! The `tutorial.toml` index.

use std::ops::Range;
use std::path::{Component, Path};

use toml_edit::{ImDocument, Item, Table, Value};

pub(crate) struct Problem {
    pub relative: String,
    pub line: usize,
    pub message: String,
}

pub(crate) struct Page {
    pub line: usize,
    pub path: String,
}

pub(crate) struct Need {
    pub program: String,
    pub minimum: Option<String>,
}

pub(crate) struct Window {
    pub name: String,
    pub folder: String,
    pub needs: Vec<Need>,
}

#[derive(Clone, Copy)]
pub(crate) enum Shell {
    Bash,
    Zsh,
}

pub(crate) struct Setting {
    pub name: String,
    pub default: String,
    pub values: Vec<String>,
}

pub(crate) struct TestedPath {
    pub name: String,
    pub values: Vec<(String, String)>,
}

pub(crate) struct Tutorial {
    pub pages: Vec<Page>,
    pub windows: Vec<Window>,
    pub settings: Vec<Setting>,
    pub paths: Vec<TestedPath>,
    pub problems: Vec<Problem>,
    pub shell: Shell,
    pub setup: Option<(String, usize)>,
}

struct Located<'a> {
    line: usize,
    value: &'a str,
}

struct RawPath {
    name: String,
    line: usize,
    values: Option<Vec<(String, String, usize)>>,
}

const INDEX: &str = "tutorial.toml";

pub(crate) fn parse(text: &str) -> Result<Tutorial, Problem> {
    let document = ImDocument::<String>::parse(text.to_owned()).map_err(|error| Problem {
        relative: INDEX.to_string(),
        line: line_number(text, error.span()),
        message: error.message().to_string(),
    })?;
    let mut problems = Vec::new();
    let mut pages = Vec::new();
    let mut windows = Vec::new();
    let mut settings = Vec::new();
    let mut raw_paths = Vec::new();
    let mut saw_pages = false;
    let mut shell = Shell::Bash;
    let mut setup = None;

    for (key_name, item) in document.iter() {
        if item.is_none() {
            continue;
        }
        let key_span = document.key(key_name).and_then(toml_edit::Key::span);
        match key_name {
            "shell" => {
                if let Some(found) = read_shell(item, text, &mut problems) {
                    shell = found;
                }
            }
            "setup" => {
                if let Some(found) = read_setup(item, text, &mut problems) {
                    setup = Some(found);
                }
            }
            "pages" => {
                saw_pages = true;
                pages = read_pages(item, text, &mut problems);
            }
            "window" => windows.extend(read_windows(item, text, &mut problems)),
            "setting" => settings.extend(read_settings(item, text, &mut problems)),
            "path" => raw_paths.extend(read_paths(item, text, &mut problems)),
            other => problems.push(index_problem(
                text,
                key_span.or_else(|| item.span()),
                format!("unknown key \"{other}\""),
            )),
        }
    }

    check_paths(&raw_paths, &settings, &mut problems);
    let paths = raw_paths
        .iter()
        .filter_map(|path| {
            let values = path.values.as_ref()?;
            Some(TestedPath {
                name: path.name.clone(),
                values: values.iter().map(|(key, value, _)| (key.clone(), value.clone())).collect(),
            })
        })
        .collect();
    if !saw_pages {
        problems.push(index_problem(text, None, "pages must list at least one page".to_string()));
    }
    if windows.is_empty() {
        problems.push(index_problem(
            text,
            None,
            "a tutorial needs at least one window".to_string(),
        ));
    }

    Ok(Tutorial { pages, windows, settings, paths, problems, shell, setup })
}

fn read_shell(item: &Item, text: &str, problems: &mut Vec<Problem>) -> Option<Shell> {
    let located = read_string(item, text, "shell", problems)?;
    match located.value {
        "bash" => Some(Shell::Bash),
        "zsh" => Some(Shell::Zsh),
        _ => {
            problems.push(at(located.line, "shell must be bash or zsh".to_string()));
            None
        }
    }
}

fn read_setup(item: &Item, text: &str, problems: &mut Vec<Problem>) -> Option<(String, usize)> {
    let located = read_string(item, text, "setup", problems)?;
    if !is_relative_path(located.value) {
        problems.push(at(located.line, "setup must be a relative path".to_string()));
        return None;
    }
    Some((located.value.to_string(), located.line))
}

fn read_pages(item: &Item, text: &str, problems: &mut Vec<Problem>) -> Vec<Page> {
    let Some(array) = item.as_value().and_then(Value::as_array) else {
        problems.push(index_problem(
            text,
            item.span(),
            "pages must be a list of page paths".to_string(),
        ));
        return Vec::new();
    };
    if array.is_empty() {
        problems.push(index_problem(
            text,
            array.span().or_else(|| item.span()),
            "pages must list at least one page".to_string(),
        ));
        return Vec::new();
    }
    let mut pages = Vec::new();
    for entry in array {
        let Some(path) = entry.as_str() else {
            problems.push(index_problem(
                text,
                entry.span(),
                "a page path must be a string".to_string(),
            ));
            continue;
        };
        let entry_line = line_number(text, entry.span());
        if !is_page_path(path) {
            problems.push(at(entry_line, format!("\"{path}\" is not a page path")));
            continue;
        }
        if pages.iter().any(|page: &Page| page.path == path) {
            problems.push(at(entry_line, format!("{path} is listed more than once")));
            continue;
        }
        pages.push(Page { line: entry_line, path: path.to_string() });
    }
    pages
}

fn read_windows(item: &Item, text: &str, problems: &mut Vec<Problem>) -> Vec<Window> {
    let Some(tables) = item.as_array_of_tables() else {
        problems.push(index_problem(
            text,
            item.span(),
            "windows are written as [[window]]".to_string(),
        ));
        return Vec::new();
    };
    let mut windows: Vec<Window> = Vec::new();
    for table in tables {
        reject_unknown_keys(table, &["name", "folder", "needs"], text, problems);
        let Some(name) = required_word(table, "name", "window name", text, problems) else {
            continue;
        };
        let folder = match table.get("folder") {
            None => {
                problems.push(index_problem(text, table.span(), "folder is required".to_string()));
                String::new()
            }
            Some(folder) => match read_string(folder, text, "folder", problems) {
                Some(located) if is_window_folder(located.value) => located.value.to_string(),
                Some(located) => {
                    problems.push(at(located.line, "folder must be a relative path".to_string()));
                    String::new()
                }
                None => String::new(),
            },
        };
        let needs =
            table.get("needs").map(|item| read_needs(item, text, problems)).unwrap_or_default();
        if windows.iter().any(|window| window.name == name.value) {
            problems.push(at(
                name.line,
                format!("window \"{}\" is declared more than once", name.value),
            ));
        } else {
            windows.push(Window { name: name.value.to_string(), folder, needs });
        }
    }
    windows
}

fn read_settings(item: &Item, text: &str, problems: &mut Vec<Problem>) -> Vec<Setting> {
    let Some(tables) = item.as_array_of_tables() else {
        problems.push(index_problem(
            text,
            item.span(),
            "settings are written as [[setting]]".to_string(),
        ));
        return Vec::new();
    };
    let mut settings: Vec<Setting> = Vec::new();
    for table in tables {
        reject_unknown_keys(table, &["name", "default", "values"], text, problems);
        let Some(name) = required_word(table, "name", "setting name", text, problems) else {
            continue;
        };
        let values = read_words(table, "values", text, problems);
        let mut default_value = String::new();
        if let Some(default) = required_word(table, "default", "default", text, problems) {
            if !values.is_empty() && !values.iter().any(|value| value == default.value) {
                problems.push(at(
                    default.line,
                    format!(
                        "default \"{}\" is not one of the values of {}",
                        default.value, name.value
                    ),
                ));
            }
            default_value = default.value.to_string();
        }
        if settings.iter().any(|setting| setting.name == name.value) {
            problems.push(at(
                name.line,
                format!("setting \"{}\" is declared more than once", name.value),
            ));
        } else {
            settings.push(Setting { name: name.value.to_string(), default: default_value, values });
        }
    }
    settings
}

fn read_paths(item: &Item, text: &str, problems: &mut Vec<Problem>) -> Vec<RawPath> {
    let Some(tables) = item.as_array_of_tables() else {
        problems.push(index_problem(
            text,
            item.span(),
            "paths are written as [[path]]".to_string(),
        ));
        return Vec::new();
    };
    let mut paths = Vec::new();
    for table in tables {
        reject_unknown_keys(table, &["name", "values"], text, problems);
        let Some(name) = required_word(table, "name", "path name", text, problems) else {
            continue;
        };
        let values = match table.get("values") {
            None => {
                problems.push(index_problem(text, table.span(), "values is required".to_string()));
                None
            }
            Some(item) => Some(read_path_values(item, text, problems)),
        };
        paths.push(RawPath { name: name.value.to_string(), line: name.line, values });
    }
    paths
}

fn read_path_values(
    item: &Item,
    text: &str,
    problems: &mut Vec<Problem>,
) -> Vec<(String, String, usize)> {
    let Some(table) = item.as_value().and_then(Value::as_inline_table) else {
        problems.push(index_problem(
            text,
            item.span(),
            "values must be a table of setting names".to_string(),
        ));
        return Vec::new();
    };
    let mut pairs = Vec::new();
    for (key, value) in table.iter() {
        let key_span = table.key(key).and_then(toml_edit::Key::span);
        let pair_line = line_number(text, value.span().or(key_span));
        let Some(setting_value) = value.as_str() else {
            problems.push(at(pair_line, format!("value of {key} must be a string")));
            continue;
        };
        if !crate::fence::is_word(setting_value) {
            problems.push(at(pair_line, format!("value of {key} must be a single word")));
            continue;
        }
        pairs.push((key.to_string(), setting_value.to_string(), pair_line));
    }
    pairs
}

fn check_paths(paths: &[RawPath], settings: &[Setting], problems: &mut Vec<Problem>) {
    let mut seen = Vec::new();
    for path in paths {
        if seen.iter().any(|name: &String| name == &path.name) {
            problems
                .push(at(path.line, format!("path \"{}\" is declared more than once", path.name)));
            continue;
        }
        seen.push(path.name.clone());
        let Some(values) = &path.values else {
            continue;
        };
        for setting in settings {
            if !values.iter().any(|(key, _, _)| key == &setting.name) {
                problems.push(at(
                    path.line,
                    format!("path \"{}\" does not set {}", path.name, setting.name),
                ));
            }
        }
        for (key, value, pair_line) in values {
            match settings.iter().find(|setting| setting.name == *key) {
                None => problems.push(at(
                    *pair_line,
                    format!("path \"{}\" sets \"{key}\", which is not a setting", path.name),
                )),
                Some(setting) if !setting.values.iter().any(|allowed| allowed == value) => {
                    problems.push(at(
                        *pair_line,
                        format!(
                            "path \"{}\" sets {key} to \"{value}\", which is not allowed",
                            path.name
                        ),
                    ));
                }
                Some(_) => {}
            }
        }
    }
}

fn read_needs(item: &Item, text: &str, problems: &mut Vec<Problem>) -> Vec<Need> {
    let Some(array) = item.as_value().and_then(Value::as_array) else {
        problems.push(index_problem(
            text,
            item.span(),
            "needs must be a list of program names".to_string(),
        ));
        return Vec::new();
    };
    let mut needs = Vec::new();
    for entry in array {
        let Some(need) = entry.as_str() else {
            problems.push(index_problem(
                text,
                entry.span(),
                "a needs entry must be a string".to_string(),
            ));
            continue;
        };
        if !valid_need(need) {
            problems.push(index_problem(
                text,
                entry.span(),
                format!(
                    "needs entry \"{need}\" must name a program, optionally followed by >= and a version"
                ),
            ));
            continue;
        }
        needs.push(parse_need(need));
    }
    needs
}

fn parse_need(entry: &str) -> Need {
    match entry.split_once(" >= ") {
        Some((program, version)) => {
            Need { program: program.to_string(), minimum: Some(version.to_string()) }
        }
        None => Need { program: entry.to_string(), minimum: None },
    }
}

fn read_words(table: &Table, key: &str, text: &str, problems: &mut Vec<Problem>) -> Vec<String> {
    let Some(item) = table.get(key) else {
        problems.push(index_problem(text, table.span(), format!("{key} is required")));
        return Vec::new();
    };
    let Some(array) = item.as_value().and_then(Value::as_array) else {
        problems.push(index_problem(text, item.span(), format!("{key} must be a list of words")));
        return Vec::new();
    };
    if array.is_empty() {
        problems.push(index_problem(
            text,
            array.span().or_else(|| item.span()),
            format!("{key} must list at least one value"),
        ));
        return Vec::new();
    }
    let mut words = Vec::new();
    for entry in array {
        let Some(word) = entry.as_str() else {
            problems.push(index_problem(
                text,
                entry.span(),
                format!("a {key} entry must be a string"),
            ));
            continue;
        };
        if !crate::fence::is_word(word) {
            problems.push(index_problem(
                text,
                entry.span(),
                format!("{key} entry \"{word}\" must be a single word"),
            ));
            continue;
        }
        words.push(word.to_string());
    }
    words
}

fn required_word<'a>(
    table: &'a Table,
    key: &str,
    what: &str,
    text: &str,
    problems: &mut Vec<Problem>,
) -> Option<Located<'a>> {
    let Some(item) = table.get(key) else {
        problems.push(index_problem(text, table.span(), format!("{what} is required")));
        return None;
    };
    let located = read_string(item, text, what, problems)?;
    if !crate::fence::is_word(located.value) {
        problems.push(at(located.line, format!("{what} must be a single word")));
        return None;
    }
    Some(located)
}

fn read_string<'a>(
    item: &'a Item,
    text: &str,
    what: &str,
    problems: &mut Vec<Problem>,
) -> Option<Located<'a>> {
    let Some(value) = item.as_value() else {
        problems.push(index_problem(text, item.span(), format!("{what} must be a string")));
        return None;
    };
    let Some(string) = value.as_str() else {
        problems.push(index_problem(
            text,
            value.span().or_else(|| item.span()),
            format!("{what} must be a string"),
        ));
        return None;
    };
    Some(Located { line: line_number(text, value.span()), value: string })
}

fn reject_unknown_keys(table: &Table, allowed: &[&str], text: &str, problems: &mut Vec<Problem>) {
    for (key, item) in table.iter() {
        if item.is_none() || allowed.contains(&key) {
            continue;
        }
        let key_span = table.key(key).and_then(toml_edit::Key::span);
        problems.push(index_problem(
            text,
            key_span.or_else(|| item.span()),
            format!("unknown key \"{key}\""),
        ));
    }
}

fn valid_need(entry: &str) -> bool {
    match entry.split_once(" >= ") {
        Some((program, version)) => is_program(program) && is_version(version),
        None => is_program(entry),
    }
}

fn is_program(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '+')
        })
}

fn is_version(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '+')
        })
}

fn is_page_path(value: &str) -> bool {
    is_relative_path(value) && value.ends_with(".md")
}

fn is_window_folder(value: &str) -> bool {
    value == "." || is_relative_path(value)
}

fn is_relative_path(value: &str) -> bool {
    if value.is_empty() || value.contains('\0') {
        return false;
    }
    let path = Path::new(value);
    !path.is_absolute()
        && path.components().all(|component| matches!(component, Component::Normal(_)))
}

fn index_problem(text: &str, span: Option<Range<usize>>, message: String) -> Problem {
    Problem { relative: INDEX.to_string(), line: line_number(text, span), message }
}

fn at(line: usize, message: String) -> Problem {
    Problem { relative: INDEX.to_string(), line, message }
}

fn line_number(text: &str, span: Option<Range<usize>>) -> usize {
    let Some(span) = span else {
        return 1;
    };
    text.get(..span.start)
        .map_or(1, |prefix| prefix.bytes().filter(|byte| *byte == b'\n').count() + 1)
}

#[cfg(test)]
mod tests {
    use super::{is_relative_path, is_window_folder};

    #[test]
    fn dot_is_a_window_folder_but_not_a_relative_path() {
        assert!(is_window_folder("."));
        assert!(!is_relative_path("."));
    }

    #[test]
    fn window_folders_accept_normal_paths_and_refuse_escapes() {
        for accepted in [".", "alice", "bob", "a/b", "chain"] {
            assert!(is_window_folder(accepted), "{accepted}");
        }
        for refused in ["", "/bob", "..", "a/../b", "a/..", "../a"] {
            assert!(!is_window_folder(refused), "{refused}");
            assert!(!is_relative_path(refused), "{refused}");
        }
    }
}
