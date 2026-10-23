//! The terminal face: the page beside the window of the next command.

use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use std::os::unix::process::CommandExt;

use crate::play::{self, Failure, Session, SessionRecord, StepOutcome, Wait};
use crate::render;
use crate::tutorial::{self, FaceOpen, FaceTutorial, PlayKind};

/// Play `dir` in the terminal. `run` overrides the folder under the home directory.
pub(crate) fn play(dir: &Path, run: Option<&Path>) -> u8 {
    let opened = match tutorial::open_for_face(dir) {
        FaceOpen::Problems(problems) => return tutorial::report_problems(dir, &problems),
        FaceOpen::Ready(book) => *book,
    };
    if opened.windows.is_empty() {
        eprintln!("a tutorial needs at least one window");
        return 1;
    }
    let script = opened.setup.as_ref().map(|(name, _)| name.as_str());
    if let Err(message) = tutorial::refuse_setup_as_root(script) {
        eprint!("{message}");
        return 1;
    }
    let needs = opened
        .needs
        .iter()
        .map(|(program, minimum)| (program.as_str(), minimum.as_deref()))
        .collect::<Vec<_>>();
    if let Some(message) = play::unmet_need(&needs) {
        eprint!("{message}");
        return 1;
    }
    let run = match resolve_run(dir, run) {
        Ok(run) => run,
        Err(message) => {
            eprint!("{message}");
            return 2;
        }
    };
    if let Some(message) = run_problem(&run) {
        eprint!("{message}");
        return 1;
    }
    let saved = read_saved_values(&run);
    let book = match &saved {
        Some(values) => match tutorial::open_face_with(dir, Some(values)) {
            FaceOpen::Problems(problems) => return tutorial::report_problems(dir, &problems),
            FaceOpen::Ready(book) => *book,
        },
        None => opened,
    };
    let tmux = match play::require_program("tmux") {
        Ok(tmux) => tmux,
        Err(failure) => return report(failure),
    };
    let had_session = session_path(&run).is_file();
    let resumed = session_alive(&run, &tmux);
    let choosing = !book.settings.is_empty() && saved.is_none() && !resumed;
    let mut session = if resumed {
        match read_session(&run) {
            Ok(session) => session,
            Err(message) => {
                eprint!("{message}");
                return 1;
            }
        }
    } else if choosing {
        let command = match page_command(dir, &run) {
            Ok(command) => command,
            Err(failure) => return report(failure),
        };
        match Session::start_page(tmux, &run, book.shell, &command) {
            Ok(session) => session,
            Err(failure) => {
                if !had_session {
                    let _ = fs::remove_dir_all(&run);
                }
                return report(failure);
            }
        }
    } else {
        match Session::start_kept(
            tmux,
            &book.windows,
            &run,
            book.shell,
            book.environment.as_deref(),
        ) {
            Ok(session) => session,
            Err(failure) => {
                if !had_session {
                    let _ = fs::remove_dir_all(&run);
                }
                return report(failure);
            }
        }
    };
    if !resumed {
        if let Err(message) = write_session(&run, &session) {
            drop(session);
            if !had_session {
                let _ = fs::remove_dir_all(&run);
            }
            eprint!("{message}");
            return 1;
        }
    }
    if !choosing {
        let cursor = load_cursor(&run, &book);
        let window = acting_window(&book, cursor.index);
        if let Err(failure) = ensure_page(&session, dir, &run, &window) {
            if !resumed && !had_session {
                drop(session);
                let _ = fs::remove_dir_all(&run);
            }
            return report(failure);
        }
    }
    let socket = session.socket().to_string();
    let tmux_bin = session.tmux().to_path_buf();
    session.retain();
    drop(session);
    if io::stdin().is_terminal() {
        let error = Command::new(tmux_bin)
            .args(["-f", "/dev/null", "-L", &socket, "attach", "-t", "play"])
            .env_remove("TMUX")
            .exec();
        eprintln!("cannot attach to the tutorial: {error}");
        1
    } else {
        println!(
            "The page and the windows are in tmux: {} -L {socket} attach -t play",
            tmux_bin.display()
        );
        0
    }
}

/// The page pane. `run` is the folder the learner's `play` created.
pub(crate) fn page(dir: &Path, run: &Path) -> u8 {
    let owned = absolute(run);
    let run = owned.as_path();
    let defaults = match tutorial::open_for_face(dir) {
        FaceOpen::Problems(problems) => return tutorial::report_problems(dir, &problems),
        FaceOpen::Ready(book) => *book,
    };
    let pane = std::env::var("TMUX_PANE").unwrap_or_default();
    if !pane.is_empty() {
        let _ = write_atomic(&page_pane_path(run), &format!("{pane}\n"));
    }
    let keys = match Keys::enable() {
        Ok(keys) => keys,
        Err(message) => {
            eprint!("{message}");
            return 1;
        }
    };
    let (book, session) = match start_page_session(defaults, dir, run, &pane, &keys) {
        PageStart::Exit(code) => return code,
        PageStart::Ready(started) => *started,
    };
    if book.setup.is_some() && !setup_done_path(run).is_file() && !agree_setup(&book, run, &keys) {
        return 0;
    }
    let mut cursor = load_cursor(run, &book);
    let _ = save_cursor(run, &book, &cursor);
    loop {
        let window = acting_window(&book, cursor.index);
        if !pane.is_empty() {
            if let Err(failure) = session.place_page(&pane, &window) {
                draw(&report_text(&failure), "q quits");
                if keys.next() == b'q' {
                    let _ = save_cursor(run, &book, &cursor);
                    return 0;
                }
                continue;
            }
        }
        draw_frame(&book, &session, &pane, &cursor);
        let key = keys.next();
        cursor.notice = None;
        if cursor.listing.is_some() {
            match key {
                b'o' => cursor.listing = None,
                b'q' => {
                    cursor.listing = None;
                    let _ = save_cursor(run, &book, &cursor);
                    return 0;
                }
                b'0'..=b'9' => {
                    if let Some(typed) = cursor.listing.as_mut() {
                        typed.push(char::from(key));
                    }
                }
                b'\r' | b'\n' => {
                    let typed = cursor.listing.take().unwrap_or_default();
                    if !typed.is_empty() {
                        choose_observation(&book, &session, &mut cursor, run, &keys, &typed);
                    }
                }
                _ => {}
            }
            continue;
        }
        if cursor.pending.is_some() {
            match key {
                b'f' => {
                    toggle_fold(&mut cursor, run, &book);
                    continue;
                }
                b'o' => {
                    cursor.listing = Some(String::new());
                    continue;
                }
                b'\r' | b'\n' => cursor.pending = None,
                b's' => {
                    cursor.pending = None;
                    advance(&mut cursor, &book);
                    let _ = save_cursor(run, &book, &cursor);
                    continue;
                }
                b'q' => {
                    let _ = save_cursor(run, &book, &cursor);
                    return 0;
                }
                _ => continue,
            }
        } else {
            match key {
                b'f' => {
                    toggle_fold(&mut cursor, run, &book);
                    continue;
                }
                b'o' => {
                    cursor.listing = Some(String::new());
                    continue;
                }
                b'\r' | b'\n' => {}
                b'e' if cursor.index < book.steps.len() => {}
                b's' => {
                    if cursor.index < book.steps.len() {
                        advance(&mut cursor, &book);
                        let _ = save_cursor(run, &book, &cursor);
                    }
                    continue;
                }
                b'b' => {
                    back(&mut cursor, &book);
                    let _ = save_cursor(run, &book, &cursor);
                    continue;
                }
                b'n' => {
                    turn(&mut cursor, &book, 1);
                    let _ = save_cursor(run, &book, &cursor);
                    continue;
                }
                b'p' => {
                    turn(&mut cursor, &book, -1);
                    let _ = save_cursor(run, &book, &cursor);
                    continue;
                }
                b'q' => {
                    let _ = save_cursor(run, &book, &cursor);
                    return 0;
                }
                _ => continue,
            }
        }
        if cursor.index >= book.steps.len() {
            continue;
        }
        let push = key == b'e';
        if !push && book.steps[cursor.index].kind == PlayKind::Observation {
            cursor.view = view_for(&book, cursor.index);
            cursor.lost = false;
            cursor.ran = None;
            let step = book.steps[cursor.index].clone();
            let position = position_at(&book, &cursor);
            match run_observation(&session, &step, &position, &observations_path(run), &keys) {
                ObservationRun::Ran(outcome) => {
                    cursor.ran = Some(outcome.command);
                    advance(&mut cursor, &book);
                }
                ObservationRun::Busy => {
                    cursor.notice = Some(format!("window {} is busy", step.window));
                }
                ObservationRun::Mismatch(outcome) => {
                    cursor.pending = Some(Pending {
                        got: outcome.status,
                        expected: step.expected,
                        command: outcome.command,
                    });
                }
                ObservationRun::Broken(failure) => {
                    draw(&report_text(&failure), "q quits");
                    let _ = save_cursor(run, &book, &cursor);
                    if keys.next() == b'q' {
                        return 0;
                    }
                    continue;
                }
            }
            let _ = save_cursor(run, &book, &cursor);
            continue;
        }
        cursor.view = view_for(&book, cursor.index);
        cursor.lost = false;
        cursor.ran = None;
        let window = acting_window(&book, cursor.index);
        draw_frame(&book, &session, &pane, &cursor);
        let step = &book.steps[cursor.index];
        let expected = step.expected;
        let outcome = if push {
            draw_status(&format!("editing in {window} · Enter there runs it"));
            session.push_step(step)
        } else {
            draw_status(&format!("running in {window} · c sends Ctrl-C"));
            session.run_step_until(step, || keys.poll_interrupt())
        };
        match outcome {
            Ok(outcome) if outcome.status == expected => {
                cursor.ran = Some(outcome.command);
                advance(&mut cursor, &book);
            }
            Ok(outcome) => {
                cursor.pending =
                    Some(Pending { got: outcome.status, expected, command: outcome.command });
            }
            Err(failure) => {
                draw(&report_text(&failure), "q quits");
                let _ = save_cursor(run, &book, &cursor);
                if keys.next() == b'q' {
                    return 0;
                }
                continue;
            }
        }
        let _ = save_cursor(run, &book, &cursor);
    }
}

struct Pending {
    got: u8,
    expected: u8,
    /// The line the shell accepted. Empty when a saved position has none.
    command: String,
}

struct Cursor {
    view: usize,
    index: usize,
    pending: Option<Pending>,
    lost: bool,
    /// The line the shell accepted, shown until the next command starts.
    ran: Option<String>,
    /// The explanations are hidden.
    folded: bool,
    /// Digits typed on the observation list. Absent when the page is showing.
    listing: Option<String>,
    /// Shown until the next key: a busy window, or a number that is not on the list.
    notice: Option<String>,
}

fn toggle_fold(cursor: &mut Cursor, run: &Path, book: &FaceTutorial) {
    cursor.folded = !cursor.folded;
    let _ = save_cursor(run, book, cursor);
}

enum PageStart {
    Exit(u8),
    Ready(Box<(FaceTutorial, Session)>),
}

struct ChoiceFailure {
    message: String,
    session: Option<Box<Session>>,
}

enum Key {
    Byte(u8),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Quit,
}

const CHOICE_STATUS: &str = "Up and k, down and j move between settings. \
Left and h, right and l change the value. \
Enter starts the tutorial. q quits.";

const CONTINUE_STATUS: &str = "Enter continues this run\nn starts a new run and keeps this folder";

fn start_page_session(
    defaults: FaceTutorial,
    dir: &Path,
    run: &Path,
    pane: &str,
    keys: &Keys,
) -> PageStart {
    if defaults.settings.is_empty() {
        return read_ready(defaults, run);
    }
    if read_saved_values(run).is_none() {
        return choose(&defaults, dir, run, pane, keys, None);
    }
    let saved = read_saved_values(run);
    let book = match tutorial::open_face_with(dir, saved.as_deref()) {
        FaceOpen::Problems(problems) => {
            return PageStart::Exit(tutorial::report_problems(dir, &problems))
        }
        FaceOpen::Ready(book) => *book,
    };
    let session = match read_session(run) {
        Ok(session) => session,
        Err(message) => {
            eprint!("{message}");
            return PageStart::Exit(1);
        }
    };
    resume(book, defaults, dir, run, pane, keys, session)
}

fn read_ready(book: FaceTutorial, run: &Path) -> PageStart {
    match read_session(run) {
        Ok(session) => PageStart::Ready(Box::new((book, session))),
        Err(message) => {
            eprint!("{message}");
            PageStart::Exit(1)
        }
    }
}

fn resume(
    book: FaceTutorial,
    defaults: FaceTutorial,
    dir: &Path,
    run: &Path,
    pane: &str,
    keys: &Keys,
    session: Session,
) -> PageStart {
    let mut session = Some(session);
    loop {
        draw_continue(&book);
        match keys.next_key() {
            Key::Enter => {
                let Some(session) = session.take() else {
                    return PageStart::Exit(1);
                };
                return PageStart::Ready(Box::new((book, session)));
            }
            Key::Quit => return PageStart::Exit(0),
            Key::Byte(b'n') => break,
            _ => {}
        }
    }
    let Some(session) = session.take() else {
        return PageStart::Exit(1);
    };
    match move_run_aside(session, run, pane) {
        Ok(session) => choose(&defaults, dir, run, pane, keys, Some(session)),
        Err(message) => {
            draw(&message, "q quits");
            let _ = keys.next();
            PageStart::Exit(0)
        }
    }
}

fn choose(
    defaults: &FaceTutorial,
    dir: &Path,
    run: &Path,
    pane: &str,
    keys: &Keys,
    mut existing: Option<Session>,
) -> PageStart {
    let mut picker = Picker::new(defaults);
    draw_choice(defaults, &picker);
    loop {
        match keys.next_key() {
            Key::Quit => {
                abandon_choice(run);
                return PageStart::Exit(0);
            }
            Key::Enter => {
                match confirm_choice(defaults, &picker, dir, run, pane, existing.take()) {
                    Ok((book, session)) => return PageStart::Ready(Box::new((book, session))),
                    Err(failed) => {
                        existing = failed.session.map(|session| *session);
                        draw(&failed.message, "Enter tries again · q quits");
                    }
                }
            }
            Key::Up | Key::Byte(b'k') => {
                picker.shift_setting(defaults, -1);
                draw_choice(defaults, &picker);
            }
            Key::Down | Key::Byte(b'j') => {
                picker.shift_setting(defaults, 1);
                draw_choice(defaults, &picker);
            }
            Key::Left | Key::Byte(b'h') => {
                picker.shift_value(defaults, -1);
                draw_choice(defaults, &picker);
            }
            Key::Right | Key::Byte(b'l') => {
                picker.shift_value(defaults, 1);
                draw_choice(defaults, &picker);
            }
            Key::Byte(_) => {}
        }
    }
}

fn confirm_choice(
    defaults: &FaceTutorial,
    picker: &Picker,
    dir: &Path,
    run: &Path,
    pane: &str,
    existing: Option<Session>,
) -> Result<(FaceTutorial, Session), ChoiceFailure> {
    let chosen = picker.chosen(defaults);
    let book = match tutorial::open_face_with(dir, Some(&chosen)) {
        FaceOpen::Ready(book) => *book,
        FaceOpen::Problems(problems) => {
            let message = problems
                .iter()
                .map(|problem| {
                    format!("{}:{}: {}", problem.relative, problem.line, problem.message)
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Err(ChoiceFailure { message, session: existing.map(Box::new) });
        }
    };
    let mut session = match existing {
        Some(session) => session,
        None => match read_session(run) {
            Ok(session) => session,
            Err(message) => return Err(ChoiceFailure { message, session: None }),
        },
    };
    if let Err(failure) = session.open_shells(&book.windows, book.environment.as_deref()) {
        return Err(ChoiceFailure {
            message: report_text(&failure),
            session: Some(Box::new(session)),
        });
    }
    if let Err(message) = write_session(run, &session) {
        return Err(ChoiceFailure { message, session: Some(Box::new(session)) });
    }
    let window = first_window(&book);
    if !pane.is_empty() && !window.is_empty() && session.place_page(pane, &window).is_ok() {
        session.release_holding();
    }
    Ok((book, session))
}

/// The page pane dies with the window that holds it, so the pane moves first.
/// The process keeps the renamed directory as its working directory, so later
/// files are written at the original path.
fn move_run_aside(mut session: Session, run: &Path, pane: &str) -> Result<Session, String> {
    if !pane.is_empty() {
        session.park_page(pane).map_err(|failure| report_text(&failure))?;
    }
    session.close_other_windows().map_err(|failure| report_text(&failure))?;
    let aside = aside_path(run);
    fs::rename(run, &aside).map_err(|error| format!("cannot keep the earlier run: {error}\n"))?;
    if let Err(failure) = session.retarget(run) {
        let _ = fs::rename(&aside, run);
        return Err(report_text(&failure));
    }
    session.remember_page(pane);
    write_atomic(&page_pane_path(run), &format!("{pane}\n"))?;
    write_session(run, &session)?;
    Ok(session)
}

fn aside_path(run: &Path) -> PathBuf {
    let name = run.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let mut number = 1_u32;
    loop {
        let candidate = run.with_file_name(format!("{name}.{number}"));
        if !candidate.exists() {
            return candidate;
        }
        number += 1;
    }
}

fn abandon_choice(run: &Path) {
    let text = fs::read_to_string(session_path(run)).ok();
    let document = text.as_deref().and_then(|text| text.parse::<toml_edit::DocumentMut>().ok());
    let socket = document
        .as_ref()
        .and_then(|document| document.get("socket"))
        .and_then(toml_edit::Item::as_str)
        .map(str::to_string);
    let tmux = document
        .as_ref()
        .and_then(|document| document.get("tmux"))
        .and_then(toml_edit::Item::as_str)
        .map(PathBuf::from);
    let _ = fs::remove_dir_all(state_dir(run));
    let (Some(tmux), Some(socket)) = (tmux, socket) else {
        return;
    };
    let _ = Command::new(tmux)
        .args(["-f", "/dev/null", "-L", &socket, "kill-server"])
        .env_remove("TMUX")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn();
}

fn read_saved_values(run: &Path) -> Option<Vec<(String, String)>> {
    let text = fs::read_to_string(run.join(".env")).ok()?;
    if text.is_empty() {
        return None;
    }
    let mut values = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            return None;
        }
        let (name, value) = line.split_once('=')?;
        if name.is_empty() || value.is_empty() {
            return None;
        }
        values.push((name.to_string(), value.to_string()));
    }
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

struct Picker {
    setting: usize,
    value: Vec<usize>,
}

impl Picker {
    fn new(book: &FaceTutorial) -> Self {
        Self {
            setting: 0,
            value: book
                .settings
                .iter()
                .map(|setting| {
                    setting.values.iter().position(|value| value == &setting.default).unwrap_or(0)
                })
                .collect(),
        }
    }

    fn chosen(&self, book: &FaceTutorial) -> Vec<(String, String)> {
        book.settings
            .iter()
            .enumerate()
            .map(|(index, setting)| {
                let selected = self.value.get(index).copied().unwrap_or(0);
                let text = setting
                    .values
                    .get(selected)
                    .cloned()
                    .unwrap_or_else(|| setting.default.clone());
                (setting.name.clone(), text)
            })
            .collect()
    }

    fn shift_setting(&mut self, book: &FaceTutorial, delta: isize) {
        self.setting = shift(self.setting, book.settings.len(), delta);
    }

    fn shift_value(&mut self, book: &FaceTutorial, delta: isize) {
        let len = book.settings.get(self.setting).map(|setting| setting.values.len()).unwrap_or(0);
        if let Some(slot) = self.value.get_mut(self.setting) {
            *slot = shift(*slot, len, delta);
        }
    }
}

fn shift(index: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        0
    } else {
        (index as isize + delta).rem_euclid(len as isize) as usize
    }
}

fn draw_choice(book: &FaceTutorial, picker: &Picker) {
    let mut body = String::from("Choose the settings\n");
    for (index, setting) in book.settings.iter().enumerate() {
        body.push('\n');
        body.push_str(&setting.name);
        body.push('\n');
        let selected = picker.value.get(index).copied().unwrap_or(0);
        for (value_index, value) in setting.values.iter().enumerate() {
            if value_index == selected {
                body.push_str("> ");
            } else {
                body.push_str("  ");
            }
            body.push_str(value);
            body.push('\n');
        }
    }
    if untested(&book.tested, &picker.chosen(book)) {
        body.push_str("\nThis combination is not a tested path.\n");
    }
    draw(&body, CHOICE_STATUS);
}

fn draw_continue(book: &FaceTutorial) {
    let line = book
        .values
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(" ");
    draw(&format!("This run is using {line}\n"), CONTINUE_STATUS);
}

fn untested(tested: &[Vec<(String, String)>], chosen: &[(String, String)]) -> bool {
    !tested.is_empty() && !tested.iter().any(|path| path.as_slice() == chosen)
}

fn notes_for(book: &FaceTutorial, page: &str) -> Vec<(usize, String)> {
    book.steps
        .iter()
        .filter(|step| step.page == page)
        .filter_map(|step| {
            let beside = tutorial::values_beside(&step.body, &book.values);
            if beside.is_empty() {
                None
            } else {
                Some((step.line, beside))
            }
        })
        .collect()
}

fn first_window(book: &FaceTutorial) -> String {
    book.steps
        .first()
        .map(|step| step.window.clone())
        .or_else(|| book.windows.first().map(|(name, _)| name.clone()))
        .unwrap_or_default()
}

fn agree_setup(book: &FaceTutorial, run: &Path, keys: &Keys) -> bool {
    let Some((name, script)) = &book.setup else {
        return true;
    };
    loop {
        let body = fs::read_to_string(script)
            .unwrap_or_else(|_| format!("the set-up script {name} is not a file\n"));
        draw(&format!("Set-up script {name}\n\n{body}"), "Enter runs the set-up script · q quits");
        match keys.next() {
            b'\r' | b'\n' => {}
            b'q' => return false,
            _ => continue,
        }
        let bash = match play::require_program("bash") {
            Ok(bash) => bash,
            Err(failure) => {
                draw(&report_text(&failure), "q quits");
                if keys.next() == b'q' {
                    return false;
                }
                continue;
            }
        };
        match play::run_setup(&bash, run, name, script) {
            Ok(()) => {
                let _ = write_atomic(&setup_done_path(run), "");
                return true;
            }
            Err(failure) => {
                draw(&report_text(&failure), "Enter tries again · q quits");
                if keys.next() == b'q' {
                    return false;
                }
            }
        }
    }
}

fn draw_frame(book: &FaceTutorial, session: &Session, pane: &str, cursor: &Cursor) {
    if cursor.listing.is_some() {
        draw_list(book, cursor);
        return;
    }
    let width = if pane.is_empty() { 80 } else { session.pane_width(pane).max(8) };
    let mut header = String::new();
    if cursor.lost {
        header.push_str(
            "The saved command is no longer in the tutorial. Starting at the beginning.\n\n",
        );
    }
    let highlight = book.pages.get(cursor.view).and_then(|page| {
        book.steps
            .get(cursor.index)
            .filter(|step| step.page == page.path && cursor.pending.is_none())
            .map(|step| step.line)
    });
    let body = book
        .pages
        .get(cursor.view)
        .map(|page| {
            let notes = notes_for(book, &page.path);
            if cursor.folded {
                let commands = book
                    .steps
                    .iter()
                    .filter(|step| step.page == page.path)
                    .map(|step| render::FoldedCommand {
                        line: step.line,
                        window: step.window.clone(),
                        body: step.body.clone(),
                        observation: step.kind == PlayKind::Observation,
                        beside: tutorial::values_beside(&step.body, &book.values),
                    })
                    .collect::<Vec<_>>();
                render::render_folded(&commands, width, highlight)
            } else {
                let values =
                    if book.settings.is_empty() { None } else { Some(book.values.as_slice()) };
                render::render(&page.markdown, width, highlight, &notes, values)
            }
        })
        .unwrap_or_default();
    let mut extra = String::new();
    if let Some(notice) = &cursor.notice {
        extra.push('\n');
        extra.push_str(notice);
        extra.push('\n');
    }
    if let (Some(pending), Some(step)) = (&cursor.pending, book.steps.get(cursor.index)) {
        let body =
            if pending.command.is_empty() { step.body.as_str() } else { pending.command.as_str() };
        extra.push_str(&format!(
            "\n{}:{}: {} exited {}, expected {}\n{}",
            step.page,
            step.line,
            noun(step.kind),
            pending.got,
            pending.expected,
            body
        ));
    } else if cursor.notice.is_none() {
        if let Some(ran) = &cursor.ran {
            extra.push_str(&format!("\nThe shell ran:\n{ran}"));
        }
    }
    draw(&format!("{header}{body}{extra}"), &status_line(book, cursor));
}

fn draw_status(status: &str) {
    let mut stdout = io::stdout().lock();
    let _ = write!(stdout, "\r\u{1b}[K{status}");
    let _ = stdout.flush();
}

fn status_line(book: &FaceTutorial, cursor: &Cursor) -> String {
    let line = if cursor.pending.is_some() {
        "Enter retries · s skips · q quits"
    } else if cursor.index >= book.steps.len() {
        "That was the tutorial. The windows stay. q closes the page · b back · n next page · p previous"
    } else {
        let step = &book.steps[cursor.index];
        let on_page = book.pages.get(cursor.view).is_some_and(|page| page.path == step.page);
        if !on_page {
            "n/p page · Enter runs the next command · e edits · q quits"
        } else if step.kind == PlayKind::Observation {
            "Enter runs this observation · e edits · s skips · b back · n/p · q quits"
        } else {
            "Enter runs it · e edits · s skips · b back · n/p page · q quits"
        }
    };
    let fold = if cursor.folded { "f unfolds" } else { "f folds" };
    format!("{line}\n{fold} · o observes")
}

fn draw_list(book: &FaceTutorial, cursor: &Cursor) {
    let reached = book
        .steps
        .iter()
        .take(cursor.index)
        .filter(|step| step.kind == PlayKind::Observation)
        .collect::<Vec<_>>();
    let mut body = if reached.is_empty() {
        "No observation has been reached yet.\n".to_string()
    } else {
        let mut text = String::from("Observations already reached\n");
        for (number, step) in reached.iter().enumerate() {
            let first = step.body.lines().next().unwrap_or("").trim_end();
            text.push_str(&format!("{}  {}  {first}\n", number + 1, step.window));
        }
        text
    };
    if let Some(notice) = &cursor.notice {
        body.push('\n');
        body.push_str(notice);
        body.push('\n');
    }
    let typed = cursor.listing.as_deref().unwrap_or("");
    let status = if typed.is_empty() {
        "Type a number and press Enter · o closes the list".to_string()
    } else {
        format!("{typed} · Enter runs it · o closes the list")
    };
    draw(&body, &status);
}

enum ObservationRun {
    Ran(StepOutcome),
    Busy,
    Mismatch(StepOutcome),
    Broken(Failure),
}

fn run_observation(
    session: &Session,
    step: &tutorial::PlayStep,
    position: &str,
    record: &Path,
    keys: &Keys,
) -> ObservationRun {
    match session.window_is_busy(&step.window) {
        Ok(true) => ObservationRun::Busy,
        Err(failure) => ObservationRun::Broken(failure),
        Ok(false) => {
            draw_status(&format!("running in {} · c sends Ctrl-C", step.window));
            match session.run_step_until(step, || keys.poll_interrupt()) {
                Ok(outcome) => match play::record_observation(record, step, &outcome, position) {
                    Ok(()) if outcome.status == step.expected => ObservationRun::Ran(outcome),
                    Ok(()) => ObservationRun::Mismatch(outcome),
                    Err(failure) => ObservationRun::Broken(failure),
                },
                Err(failure) => ObservationRun::Broken(failure),
            }
        }
    }
}

fn choose_observation(
    book: &FaceTutorial,
    session: &Session,
    cursor: &mut Cursor,
    run: &Path,
    keys: &Keys,
    typed: &str,
) {
    let Some(number) = typed.parse::<usize>().ok().filter(|number| *number > 0) else {
        cursor.listing = Some(String::new());
        cursor.notice = Some(format!("There is no observation {typed}."));
        return;
    };
    let Some(step) = book
        .steps
        .iter()
        .take(cursor.index)
        .filter(|step| step.kind == PlayKind::Observation)
        .nth(number - 1)
        .cloned()
    else {
        cursor.listing = Some(String::new());
        cursor.notice = Some(format!("There is no observation {number}."));
        return;
    };
    let position = position_at(book, cursor);
    match run_observation(session, &step, &position, &observations_path(run), keys) {
        ObservationRun::Ran(outcome) => cursor.ran = Some(outcome.command),
        ObservationRun::Busy => {
            cursor.notice = Some(format!("window {} is busy", step.window));
        }
        ObservationRun::Mismatch(outcome) => {
            cursor.notice = Some(format!(
                "{}:{}: observation exited {}, expected {}\n{}",
                step.page, step.line, outcome.status, step.expected, outcome.command
            ));
        }
        ObservationRun::Broken(failure) => cursor.notice = Some(report_text(&failure)),
    }
}

fn position_at(book: &FaceTutorial, cursor: &Cursor) -> String {
    if cursor.index >= book.steps.len() {
        "done".to_string()
    } else {
        let step = &book.steps[cursor.index];
        format!("{}:{}", step.page, step.line)
    }
}

fn noun(kind: PlayKind) -> &'static str {
    match kind {
        PlayKind::Command => "command",
        PlayKind::Observation | PlayKind::ObservationAtEnd => "observation",
    }
}

fn advance(cursor: &mut Cursor, book: &FaceTutorial) {
    if cursor.index < book.steps.len() {
        cursor.index += 1;
    }
    cursor.pending = None;
    cursor.lost = false;
    cursor.view = view_for(book, cursor.index);
}

fn back(cursor: &mut Cursor, book: &FaceTutorial) {
    cursor.pending = None;
    cursor.lost = false;
    if cursor.index > 0 {
        cursor.index -= 1;
    }
    cursor.view = view_for(book, cursor.index);
}

fn turn(cursor: &mut Cursor, book: &FaceTutorial, delta: isize) {
    let next = cursor.view as isize + delta;
    if next >= 0 && (next as usize) < book.pages.len() {
        cursor.view = next as usize;
    }
}

fn view_for(book: &FaceTutorial, index: usize) -> usize {
    let path = book
        .steps
        .get(index)
        .map(|step| step.page.as_str())
        .or_else(|| book.steps.last().map(|step| step.page.as_str()));
    path.and_then(|path| page_index(book, path)).unwrap_or(0)
}

fn page_index(book: &FaceTutorial, path: &str) -> Option<usize> {
    book.pages.iter().position(|page| page.path == path)
}

fn acting_window(book: &FaceTutorial, index: usize) -> String {
    book.steps
        .get(index)
        .or_else(|| book.steps.last())
        .map(|step| step.window.clone())
        .or_else(|| book.windows.first().map(|(name, _)| name.clone()))
        .unwrap_or_default()
}

fn draw(body: &str, status: &str) {
    let mut out = String::from("\u{1b}[H\u{1b}[2J");
    out.push_str(body);
    if !body.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(status);
    let mut stdout = io::stdout().lock();
    let _ = stdout.write_all(out.as_bytes());
    let _ = stdout.flush();
}

struct Keys {
    saved: String,
    rx: Receiver<u8>,
}

impl Keys {
    fn enable() -> Result<Self, String> {
        let stty = play::require_program("stty").map_err(|failure| report_text(&failure))?;
        let saved = command_output(&stty, &["-g"])?;
        command_status(&stty, &["-echo", "-icanon", "-isig", "min", "1", "time", "0"])?;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut input = io::stdin().lock();
            let mut buf = [0_u8; 1];
            while let Ok(1) = input.read(&mut buf) {
                if tx.send(buf[0]).is_err() {
                    break;
                }
            }
        });
        Ok(Self { saved, rx })
    }

    fn next(&self) -> u8 {
        self.rx.recv().unwrap_or(b'q')
    }

    fn next_key(&self) -> Key {
        let byte = self.next();
        if byte != 0x1b {
            return match byte {
                b'\r' | b'\n' => Key::Enter,
                b'q' => Key::Quit,
                other => Key::Byte(other),
            };
        }
        let second = match self.rx.recv_timeout(Duration::from_millis(150)) {
            Ok(second) => second,
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                return Key::Byte(byte);
            }
        };
        if second != b'[' && second != b'O' {
            return Key::Byte(byte);
        }
        loop {
            match self.rx.recv_timeout(Duration::from_millis(150)) {
                Ok(b'A') => return Key::Up,
                Ok(b'B') => return Key::Down,
                Ok(b'C') => return Key::Right,
                Ok(b'D') => return Key::Left,
                Ok(digit) if digit.is_ascii_digit() || digit == b';' => {}
                Ok(_) | Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                    return Key::Byte(byte);
                }
            }
        }
    }

    fn poll_interrupt(&self) -> Wait {
        let mut interrupt = false;
        while let Ok(byte) = self.rx.try_recv() {
            if byte == b'c' {
                interrupt = true;
            }
        }
        if interrupt {
            Wait::Interrupt
        } else {
            Wait::Continue
        }
    }
}

impl Drop for Keys {
    fn drop(&mut self) {
        if let Ok(stty) = play::require_program("stty") {
            let _ = Command::new(stty).arg(&self.saved).status();
        }
    }
}

/// Run `program` with the page's terminal on standard input.
/// `Command::output` replaces that terminal with a closed pipe, which stty refuses.
fn run_on_terminal(program: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let child = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run {}: {error}\n", program.display()))?;
    child.wait_with_output().map_err(|error| format!("cannot run {}: {error}\n", program.display()))
}

fn command_output(program: &Path, args: &[&str]) -> Result<String, String> {
    let output = run_on_terminal(program, args)?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cannot run {} {}: {detail}\n", program.display(), args.join(" ")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn command_status(program: &Path, args: &[&str]) -> Result<(), String> {
    let output = run_on_terminal(program, args)?;
    if output.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr);
        Err(format!("cannot run {} {}: {detail}\n", program.display(), args.join(" ")))
    }
}

fn ensure_page(session: &Session, dir: &Path, run: &Path, window: &str) -> Result<(), Failure> {
    let pane_file = page_pane_path(run);
    if let Ok(id) = fs::read_to_string(&pane_file) {
        let id = id.trim();
        if !id.is_empty() && session.pane_open(id) {
            return session.place_page(id, window);
        }
    }
    let _ = fs::remove_file(&pane_file);
    let command = page_command(dir, run)?;
    session.split_page(window, &command)
}

fn page_command(dir: &Path, run: &Path) -> Result<String, Failure> {
    let exe = std::env::current_exe()
        .map_err(|error| Failure::Report(format!("cannot find this program: {error}\n")))?;
    Ok(format!(
        "exec {} play --page --run {} {}",
        play::sh_single(&exe.display().to_string()),
        play::sh_single(&absolute(run).display().to_string()),
        play::sh_single(&absolute(dir).display().to_string()),
    ))
}

fn resolve_run(dir: &Path, run: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(run) = run {
        return Ok(run.to_path_buf());
    }
    let canonical = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let Some(name) = canonical.file_name() else {
        return Err("the tutorial directory has no name; pass --run DIR\n".to_string());
    };
    let Some(home) = std::env::var_os("HOME").filter(|home| !home.is_empty()) else {
        return Err("the home directory is not set; pass --run DIR\n".to_string());
    };
    Ok(PathBuf::from(home).join(format!("{}-run", name.to_string_lossy())))
}

fn run_problem(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }
    if !path.is_dir() {
        return Some(format!("{} is not a directory\n", path.display()));
    }
    if session_path(path).is_file() {
        return None;
    }
    match fs::read_dir(path) {
        Ok(mut entries) => entries.next().map(|_| format!("{} is not empty\n", path.display())),
        Err(error) => Some(format!("cannot read {}: {error}\n", path.display())),
    }
}

fn session_alive(run: &Path, tmux: &Path) -> bool {
    let Ok(text) = fs::read_to_string(session_path(run)) else {
        return false;
    };
    let Ok(document) = text.parse::<toml_edit::DocumentMut>() else {
        return false;
    };
    let Some(socket) = document.get("socket").and_then(toml_edit::Item::as_str) else {
        return false;
    };
    Command::new(tmux)
        .args(["-f", "/dev/null", "-L", socket, "has-session", "-t", "play"])
        .env_remove("TMUX")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn write_session(run: &Path, session: &Session) -> Result<(), String> {
    let record = session.record();
    write_atomic(&session_path(run), &session_text(&record))
}

fn session_text(record: &SessionRecord) -> String {
    let mut text = String::new();
    text.push_str(&format!("socket = {}\n", toml_string(&record.socket)));
    text.push_str(&format!("marker = {}\n", toml_string(&record.marker)));
    text.push_str(&format!("tmux = {}\n", toml_string(&record.tmux.display().to_string())));
    text.push_str(&format!("shell = {}\n", toml_string(&record.shell_name)));
    text.push_str(&format!(
        "control = {}\n",
        toml_string(&record.control_dir.display().to_string())
    ));
    for pane in &record.panes {
        text.push_str("\n[[window]]\n");
        text.push_str(&format!("name = {}\n", toml_string(&pane.name)));
        text.push_str(&format!("directory = {}\n", toml_string(&pane.dir.display().to_string())));
        text.push_str(&format!(
            "status = {}\n",
            toml_string(&pane.status_path.display().to_string())
        ));
        text.push_str(&format!("log = {}\n", toml_string(&pane.log_path.display().to_string())));
        text.push_str(&format!("pane = {}\n", toml_string(&pane.id)));
    }
    text
}

fn read_session(run: &Path) -> Result<Session, String> {
    let text = fs::read_to_string(session_path(run))
        .map_err(|error| format!("cannot read the run: {error}\n"))?;
    let document: toml_edit::DocumentMut =
        text.parse().map_err(|error| format!("cannot read the run: {error}\n"))?;
    let socket = required_str(&document, "socket")?;
    let marker = required_str(&document, "marker")?;
    let tmux = PathBuf::from(required_str(&document, "tmux")?);
    let shell_name = required_str(&document, "shell")?;
    let control_dir = PathBuf::from(required_str(&document, "control")?);
    let mut panes = Vec::new();
    let Some(windows) = document.get("window").and_then(toml_edit::Item::as_array_of_tables) else {
        return Err("cannot read the run: no windows\n".to_string());
    };
    for window in windows {
        panes.push(play::PaneRecord {
            name: required_table_str(window, "name")?,
            dir: PathBuf::from(required_table_str(window, "directory")?),
            status_path: PathBuf::from(required_table_str(window, "status")?),
            log_path: PathBuf::from(required_table_str(window, "log")?),
            id: required_table_str(window, "pane")?,
        });
    }
    Ok(Session::reopen(SessionRecord {
        tmux,
        socket,
        marker,
        shell_name,
        run_dir: absolute(run),
        control_dir,
        panes,
    }))
}

fn required_str(document: &toml_edit::DocumentMut, key: &str) -> Result<String, String> {
    document
        .get(key)
        .and_then(toml_edit::Item::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("cannot read the run: {key} is missing\n"))
}

fn required_table_str(table: &toml_edit::Table, key: &str) -> Result<String, String> {
    table
        .get(key)
        .and_then(toml_edit::Item::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("cannot read the run: {key} is missing\n"))
}

fn load_cursor(run: &Path, book: &FaceTutorial) -> Cursor {
    let mut cursor = Cursor {
        view: view_for(book, 0),
        index: 0,
        pending: None,
        lost: false,
        ran: None,
        folded: false,
        listing: None,
        notice: None,
    };
    let Ok(text) = fs::read_to_string(position_path(run)) else {
        return cursor;
    };
    let Ok(document) = text.parse::<toml_edit::DocumentMut>() else {
        return cursor;
    };
    cursor.folded = document.get("folded").and_then(toml_edit::Item::as_bool).unwrap_or(false);
    cursor.view = document
        .get("view")
        .and_then(toml_edit::Item::as_str)
        .and_then(|path| page_index(book, path))
        .unwrap_or(0);
    if document.get("done").and_then(toml_edit::Item::as_bool) == Some(true) {
        cursor.index = book.steps.len();
        return cursor;
    }
    let Some(page) = document.get("page").and_then(toml_edit::Item::as_str) else {
        return cursor;
    };
    let Some(line) = document.get("line").and_then(toml_edit::Item::as_integer) else {
        return cursor;
    };
    let Ok(line) = usize::try_from(line) else {
        return cursor;
    };
    let Some(index) = book.steps.iter().position(|step| step.page == page && step.line == line)
    else {
        cursor.index = 0;
        cursor.lost = true;
        return cursor;
    };
    cursor.index = index;
    cursor.pending = pending_from(&document);
    cursor
}

fn pending_from(document: &toml_edit::DocumentMut) -> Option<Pending> {
    let got = document.get("got").and_then(toml_edit::Item::as_integer)?;
    let expected = document.get("expected").and_then(toml_edit::Item::as_integer)?;
    let command =
        document.get("command").and_then(toml_edit::Item::as_str).unwrap_or("").to_string();
    Some(Pending { got: u8::try_from(got).ok()?, expected: u8::try_from(expected).ok()?, command })
}

fn save_cursor(run: &Path, book: &FaceTutorial, cursor: &Cursor) -> Result<(), String> {
    let view = book.pages.get(cursor.view).map(|page| page.path.as_str()).unwrap_or("");
    let mut text = format!("view = {}\nfolded = {}\n", toml_string(view), cursor.folded);
    if cursor.index >= book.steps.len() {
        text.push_str("done = true\n");
    } else {
        let step = &book.steps[cursor.index];
        text.push_str("done = false\n");
        text.push_str(&format!("page = {}\n", toml_string(&step.page)));
        text.push_str(&format!("line = {}\n", step.line as i64));
    }
    if let Some(pending) = &cursor.pending {
        text.push_str(&format!("got = {}\nexpected = {}\n", pending.got, pending.expected));
        text.push_str(&format!("command = {}\n", toml_string(&pending.command)));
    }
    write_atomic(&position_path(run), &text)
}

fn toml_string(value: &str) -> String {
    toml_edit::Value::from(value).to_string()
}

fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot write {}: {error}\n", path.display()))?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, text)
        .map_err(|error| format!("cannot write {}: {error}\n", path.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("cannot write {}: {error}\n", path.display()))?;
    Ok(())
}

fn state_dir(run: &Path) -> PathBuf {
    run.join(".tutorial-stepper")
}

fn session_path(run: &Path) -> PathBuf {
    state_dir(run).join("session.toml")
}

fn position_path(run: &Path) -> PathBuf {
    state_dir(run).join("position.toml")
}

fn page_pane_path(run: &Path) -> PathBuf {
    state_dir(run).join("page-pane")
}

fn setup_done_path(run: &Path) -> PathBuf {
    state_dir(run).join("setup-done")
}

fn observations_path(run: &Path) -> PathBuf {
    state_dir(run).join("observations.toml")
}

fn absolute(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir().map(|cwd| cwd.join(path)).unwrap_or_else(|_| path.to_path_buf())
}

fn report(failure: Failure) -> u8 {
    eprint!("{}", report_text(&failure));
    1
}

fn report_text(failure: &Failure) -> String {
    match failure {
        Failure::Report(message) => message.clone(),
        Failure::Status { page, line, body, got, expected, .. } => {
            format!("{page}:{line}: exited {got}, expected {expected}\n{body}")
        }
    }
}
