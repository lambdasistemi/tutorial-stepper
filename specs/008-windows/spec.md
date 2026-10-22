# Feature Specification: Several windows, the set-up script and the needs check

**Feature Branch**: `feat/8-windows`
**Created**: 2026-10-22
**Status**: Specified
**Input**: Issue 8, "Several windows, the set-up script and the needs check"

## User Scenarios & Testing

### User Story 1 — An author plays a tutorial in the windows it declares (Priority: P1)

A tutorial author declares each window in the index, with a working folder, and runs `tutorial-stepper play --auto DIR --transcript FILE`. Each command runs in the window its block names. A command whose block names no window runs in the window of the previous command. The first command that names no window runs in the first window the index declares. The shell for a window starts in that window's folder, under the run folder. The transcript records the window with the command, and a command that prints its working directory shows the folder.

**Why this priority**: A tutorial with several people is several windows. One window already plays.

**Independent Test**: `examples/two-windows` exits 0. Its transcript has one entry in `alice` whose output ends with `/alice`, one in `bob` whose output ends with `/bob`, and a following command that names no window and still ends with `/bob`. `examples/one-window` still plays one command.

**Acceptance Scenarios**:

1. **Given** a tutorial with two windows, **When** the author plays it, **Then** each window is its own bash shell on the stepper's tmux server, started in that window's folder, and a command marked `window=NAME` is typed into that shell.
2. **Given** a command that names no window, **When** an earlier command ran in another window, **Then** the command runs in that earlier window. **When** it is the first command, **Then** it runs in the first window the index declares.
3. **Given** a command that changes directory in one window, **When** the next command runs in another window, **Then** the other window is still in its own folder.
4. **Given** a one-window tutorial that already plays, **When** the author plays it, **Then** the outcome is unchanged: one shell, the same transcript keys plus the window name, and the same refusals for zsh and for a command marked `if=`.
5. **Given** a passing run or a failing run, **When** it ends, **Then** the tmux server is gone.

### Edge Cases

- A block marked `mode=show` or `mode=observe` does not run and does not change which window the next command uses.
- A fence with no braces is not a command.
- zsh is still refused, naming zsh, before a shell starts. A command marked `if=` is still refused, naming that block. No transcript is created in either case.
- The run folder holds one directory per window folder. Without `--run` that folder is temporary and is removed when the run ends, including when the run fails. The transcript is the file the author named, outside that folder.
- `--run DIR` keeps the run folder. Each window's folder is `DIR` joined with the folder from the index, and it is still there after the run so the files a command or the set-up script wrote can be inspected. A `DIR` that already exists and is not empty is refused, and nothing inside it is changed or deleted. A `DIR` that is a file is refused. A missing `DIR` is created. An empty directory is used.
- The stepper still looks up `tmux` and `bash` on `PATH` before starting a shell. The first one missing is named, and no server is started.

## Requirements

### Functional Requirements

- `play --auto` plays every window the index declares. It no longer refuses a second window.
- Each window is one bash shell. The shells share one tmux server of the stepper's own. The server uses its own socket and does not read a tmux configuration file. Each shell loads the same prompt hook. The hook reports that shell's command end and exit status. The stepper does not decide that a command ended by a deadline or by reading the output.
- The window of a command is the fence's `window` when it has one, otherwise the window of the previous command, otherwise the first window in the index.
- The shell starts in `<run folder>/<window folder>`. The stepper creates those directories. A later command in that same shell sees the directory that shell is in, including a directory an earlier command there changed.
- The transcript gains one key, `window`, the window's name. The other keys stay as issue 7 specified them. A run that plays no command still writes an empty file.
- `--run DIR` is optional. When it is absent, the run folder is removed at the end. When it is present, the run folder is kept. A non-empty existing directory is refused before a shell starts, with no transcript, and the directory is left as it was.
- `tutorial-stepper play --help` describes `--run` and `--run-setup`, each with a description, and still gives an example. The help stays within the existing line limit.

### Key Entities

- **Window**: a name, a folder relative to the run folder, and the programs that window needs.
- **Run folder**: the directory that contains every window folder for one play. Temporary unless `--run` names it.
- **Transcript entry**: the issue 7 entry plus the window name.

## User Story 2 — The set-up script runs only when the author asks (Priority: P1)

The index may name a set-up script. In the unattended run the script runs only when `--run-setup` is given. It runs after the window folders exist and before the first page, as the user who started `play`, with the run folder as its working directory. The stepper does not raise privileges and does not invoke `sudo` or `su`. Without `--run-setup` the run stops before any command and names the script. Nothing is typed, and the transcript is not created.

**Why this priority**: The script prepares the windows. Running it unasked would change the learner's machine. Issue 7 already stops and names the script; this issue runs it when asked.

**Independent Test**: A tutorial whose script writes a file into a window folder plays with `--run-setup` and a later command in that window reads the file. The same tutorial without `--run-setup` exits non-zero, names the script, writes no transcript, and does not run the script.

**Acceptance Scenarios**:

1. **Given** a set-up script and no `--run-setup`, **When** the author plays the tutorial, **Then** the process exits non-zero, the message names the script, no shell starts, and the script does not run.
2. **Given** `--run-setup`, **When** the script exits 0, **Then** the page commands run afterwards and can see the files the script wrote in the window folders.
3. **Given** `--run-setup`, **When** the script exits with another status, **Then** the run stops, the message names the script and that status, and no page command runs.
4. **Given** `--run-setup` and a script path that is not a file, **When** the author plays the tutorial, **Then** the run stops and names the missing script.
5. **Given** a tutorial with no set-up script, **When** the author passes `--run-setup`, **Then** the flag changes nothing.
6. **Given** the user who started `play`, **When** the script runs, **Then** its user id is that user's id.

## User Story 3 — A missing program is named before the first command (Priority: P1)

Each window may declare the programs it needs, and a minimum version where one matters. `play` checks every declaration before the set-up script and before the first command. A missing program, a program older than the declared minimum, or a minimum whose program prints no version stops the run. The message names the program and the version that was found. No shell starts and the transcript is not created.

**Why this priority**: A tutorial must not fail halfway because a program is absent. The index already records the needs. `check` does not look them up.

**Independent Test**: A window that needs `tutorial-stepper-missing-program` stops before its command, and the message contains that name and says it was not found. A window that needs `bash >= 99.0.0` stops with the version `bash --version` printed, when that version is older than 99.0.0.

**Acceptance Scenarios**:

1. **Given** a program name that is not on `PATH`, **When** the author plays the tutorial, **Then** the message names the program and says it was not found, and the first command does not run.
2. **Given** a minimum version and a program whose `--version` output contains an older dotted number, **When** the author plays the tutorial, **Then** the message names the program, the dotted number that was found, and the version the index requires.
3. **Given** a minimum version and a program whose `--version` output contains no dotted number, **When** the author plays the tutorial, **Then** the message names the program and says it printed no version.
4. **Given** a program with no minimum version, **When** that program is on `PATH`, **Then** the check passes and `--version` is not required.
5. **Given** a program whose dotted number is at least the minimum, **When** the author plays the tutorial, **Then** the needs check does not stop the run.
6. **Given** several needs and several windows, **When** one of them fails, **Then** the run stops at the first failure, in index order of windows and then of needs.

### How a version is read

The stepper runs `PROGRAM --version` and reads the standard output and the standard error as one text. The version is the first dotted number in that text: digits, a dot, and digits, repeated, matching `[0-9]+(\.[0-9]+)+`. `singular 0.14.0 (commit abc)` yields `0.14.0`. `GNU bash, version 5.2.15(1)-release` yields `5.2.15`.

Two dotted numbers are compared component by component, as integers, the missing components of the shorter one treated as 0. `1.2` equals `1.2.0`. `1.10` is newer than `1.9`. The declared minimum uses the same reading, and the index already requires it to be a version token.

When the text contains no dotted number, no version was found. A need that declares no minimum does not run `--version`.

## Success Criteria

- An author can play `examples/two-windows` and read, in the transcript, which window ran each command and that the working directory was that window's folder.
- An author can pass `--run DIR` and, after the run, list each window folder under `DIR`. A second play into the same non-empty `DIR` is refused and leaves the directory unchanged.
- An author who omits `--run` does not keep a run folder afterwards.
- An author who omits `--run-setup` on a tutorial with a set-up script is stopped, and the message names the script, before any command.
- An author who passes `--run-setup` sees the script run first, and a command in a window can read a file the script wrote there.
- An author whose tutorial needs a missing program, or a newer version than the one installed, is stopped before the first command. The message names the program and the version found, or says that none was found.
- A reader of `docs/play.md` can say how a window is chosen, what `--run` and `--run-setup` do, how a version is read, and what happens when a program prints no version, without reading the source.

## Assumptions

- Issue 9 still owns zsh. This command refuses it.
- Issue 10 still owns settings and tested paths. A command marked `if=` is refused. The index may declare settings; play does not choose them.
- Issue 11 still owns observations in the unattended run. `mode=observe` is not played.
- The set-up script is a bash script. The stepper runs it with the `bash` it found on `PATH`, as `bash <script>`, with the run folder as the working directory. The script's own exit status is the result. The stepper does not pass `--login` and does not change user.
- On success the script's own output is not the stepper's standard output. The confirmation stays `1 command played` or `N commands played`.
- Needs are checked after the refusals for zsh, for `if=`, and for a set-up script the author did not ask to run, and before the run folder is created. A refused run does not create `--run` and does not write the transcript.
- `examples/two-windows`, `examples/setup`, and `examples/needs-missing` are the conformance examples. `examples/setup` declares two windows and a set-up script, so one play with `--run` and `--run-setup` shows both. `examples/needs-missing` is a sound tutorial whose play must fail. A newer installed version is covered by a test that requires `bash >= 99.0.0`, not by a committed example that would start passing on a bash that new.
- The transcript's new `window` key is part of this command. Consumers of the issue 7 transcript that ignore unknown keys still read the other fields.

## Out of scope

- Installing programs, including from the set-up script's point of view: the stepper still installs nothing. The script runs; if it installs, that is the script, and it still must not need root.
- Containers.
- zsh, settings, tested paths, and observations played again at the end.
- The interactive face.
