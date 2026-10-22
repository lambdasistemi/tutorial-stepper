# Feature Specification: Settings and tested paths in the unattended run

**Feature Branch**: `feat/10-settings`
**Created**: 2026-10-22
**Status**: Specified
**Input**: Issue 10, "Settings and tested paths in the unattended run"

## User Scenarios & Testing

### User Story 1 — An author plays one tested path (Priority: P1)

A tutorial author declares settings and tested paths in the index, then runs `tutorial-stepper play --auto DIR --transcript FILE --path NAME`. The stepper writes that path's values into `.env` in the run folder before any window opens. Every window loads that file before its first command. Only the blocks that belong to the path run. The author sees the path's values in the windows' environment.

**Why this priority**: This is the command the issue names. The index, the attributes, and `check` already accept settings, paths, and `if`. Unattended play still refuses a block marked `if=`.

**Independent Test**: A tutorial with settings `SIGNING` and `PACE`, and a path `careful` that sets them to `external` and `slow`, plays only the shared block and the blocks marked for those values. The transcript output of the shared block is `external slow`. `.env` in the run folder is the two lines below, in that order. The block marked for the other path is not in the transcript.

**Acceptance Scenarios**:

1. **Given** a tested path named in `--path`, **When** the author plays it, **Then** the run folder contains `.env` before any window opens, and each window has those variables exported before its first command.
2. **Given** a block marked `if=NAME:value`, **When** the path sets `NAME` to that value, **Then** the block runs. **When** the path sets `NAME` to another value, **Then** the block does not run and does not change the window the next command inherits.
3. **Given** a block or page with no `if`, **When** any path plays, **Then** the block runs. A page has no `if` of its own in this format, so the page is shared and its blocks decide.
4. **Given** `--path` names something that is not a tested path, **When** the author plays or prints the tutorial, **Then** the command exits 1, names that path, starts no shell, and writes no transcript and no script.
5. **Given** `tutorial-stepper check` on a block whose `if` names an undeclared setting or a value that setting does not allow, **When** the author runs it, **Then** the command exits 1 and the message names that setting or value. This already holds.

### User Story 2 — An author prints one tested path

A tutorial author runs `tutorial-stepper script DIR --path NAME` and reads one bash script. It contains the commands and observations that belong to that path, in page order, and no others. Without `--path`, the script still contains every branch, as it does today.

**Why this priority**: The issue names this command next to `check`. It uses the same selection as play and needs no shell.

**Independent Test**: `tutorial-stepper script examples/if --path external` prints `echo external` and does not print `echo internal`. Without `--path`, both commands are printed.

**Acceptance Scenarios**:

1. **Given** `--path NAME`, **When** the author prints the script, **Then** a block with no `if` is printed, a block whose `if` matches the path is printed, and a block whose `if` names another value is left out. A block marked `mode=show` stays left out. An observation is printed only when it belongs to the path.
2. **Given** no `--path`, **When** the author prints the script, **Then** every runnable block is printed, including every `if` branch.
3. **Given** a tutorial `check` rejects, **When** the author passes `--path`, **Then** the problems are the same as `check` and standard output is empty.

### User Story 3 — An author plays every tested path

A tutorial author runs `tutorial-stepper play --auto DIR --transcript FILE` and passes no `--path`. The stepper plays every tested path, in the order the index declares them, each from a clean run folder. It reports each path's result. It exits non-zero if any path fails, after the remaining paths have played.

**Why this priority**: Continuous integration runs a tutorial once and needs every combination the author promised to test.

**Independent Test**: `examples/settings` has two settings and two tested paths. `--path careful` exits 0. Without `--path`, both paths play and the command exits non-zero because the `quick` path's last command does not exit 19.

**Acceptance Scenarios**:

1. **Given** tested paths and no `--path`, **When** the author plays the tutorial with `--run DIR`, **Then** `DIR` holds one sub-folder per path, each with its own `.env` and its own window folders, and each path's commands run only in that sub-folder.
2. **Given** one path fails, **When** a later path is declared, **Then** that later path still plays from its own clean run folder. The command exits 1. Standard output names each path that played through, and standard error names each path that failed.
3. **Given** a tutorial with no tested paths, **When** the author plays it without `--path`, **Then** it plays once, as it does today. Settings, if any, take their defaults, and blocks are selected with those defaults.
4. **Given** `examples/if`, **When** the author plays it without `--run-setup`, **Then** the run still stops before a shell and names `setup.sh`.

### Edge Cases

- `.env` is one `NAME=value` line per setting, in the order the settings are declared. There is no `export`, no quotes, and no blank line. A newline follows each line. Values are single words, which is why nothing is quoted.
- The shell sources `.env` while it is starting, with every assignment exported, and only then installs the prompt hook. The load is not a command on a page and does not appear in the transcript.
- A tutorial with no settings writes no `.env`. Its windows source nothing.
- A skipped block does not change the inherited window. `mode=show` and `mode=observe` still do not change it. `mode=observe` is still not played.
- The set-up script, when `--run-setup` is given, runs inside each path's run folder, after `.env` is written and before the windows open. Without `--run-setup`, a declared script still stops the run before any path, and the message names the script.
- Needs are checked once, before any path. A missing program stops the run before a shell and before `.env`.
- `--run DIR` for a single run (`--path`, or a tutorial with no tested paths) is that run's folder, as today. For every tested path, `DIR` itself must be missing or empty, and each path uses `DIR/<path name>`.
- A path that plays no command still starts its windows, writes an empty stretch of transcript, and reports zero commands.
- The transcript of a tested path adds `path` to each command. A tutorial with no tested paths keeps the transcript keys it has today.
- `check` counts every command fence, including commands that belong to only one path.

## Requirements

### Functional Requirements

- `play --auto` accepts `--path NAME`. That name is a tested path. The run uses that path's values.
- Without `--path`, a tutorial that declares tested paths plays each of them, in index order, from a clean run folder. The command exits 0 only when every path exits 0.
- Each path's result is reported when that path finishes. A path that played through prints `<name>: N commands played` on standard output. A path that fails prints the failure this command already prints, with `<name>: ` before the first line, on standard error.
- One `--path`, and a tutorial with no tested paths, keep the report this command already prints: `N commands played`, or the failure on standard error with an empty standard output. Those runs do not prefix the path name.
- `.env` holds the values in force, one `NAME=value` line per setting, and is written in the run folder before any window opens. Each window loads it before its first command.
- A block marked `if=NAME:value` runs only when `NAME` has that value. A block with no `if` always runs. Version 0 of the format does not mark a page: a page is shared.
- `script --path NAME` prints that path's commands and observations. Without `--path` it prints every branch.
- `check` is unchanged. An `if` that names an undeclared setting or a value that is not allowed is still a format error at that fence.
- An unknown `--path` is `tutorial.toml:1: path "NAME" is not a tested path` on standard error, exit 1, with nothing started and nothing printed.
- The refusal `play --auto does not choose a setting` is removed.
- No new continuous-integration job. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.

### The environment file

For a path that sets `SIGNING` to `external` and `PACE` to `slow`, with the settings declared in that order, the file is exactly:

```text
SIGNING=external
PACE=slow
```

### `examples/settings`

| Path | Values | What runs | Outcome |
| --- | --- | --- | --- |
| `careful` | `SIGNING=external`, `PACE=slow` | the shared command, the external command, and the slow command | Exit 0. The shared command prints `external slow`. |
| `quick` | `SIGNING=internal`, `PACE=fast` | the shared command, the internal command, and the fast command | The fast command exits 0 and the block declares 19, so this path fails. |

Without `--path`, `careful` plays through and `quick` fails. The command exits 1. With `--path careful`, only `careful` plays and the command exits 0.

## Success Criteria

- An author who passes `--path NAME` can read that path's values in `.env` and in every window, and can see that the other path's blocks did not run.
- An author who omits `--path` gets one result per tested path and a non-zero exit when any path fails.
- An author who runs `script --path NAME` reads only that path's commands.
- A reader of `docs/play.md` can say what `.env` contains, when it is written, and how a window loads it. A reader of `docs/script.md` can say what `--path` leaves out.

## Assumptions

- Choosing values interactively stays in issue 16. This command does not ask.
- A setting does not change in the middle of a run.
- Issue 11 still owns observations played again at the end. `mode=observe` is not played.
- A tutorial with settings and no tested paths plays once, on the defaults. The index allows that shape, and `check` already accepts it.
- The settled format marks `if` on a fence, not on a page. No new page syntax is added.
- A setting name is the left-hand side of a shell assignment. The settings this command's examples use are shell names. A name the shell cannot assign stops that window before the first command.

## Out of scope

- The interactive choice of values.
- A way to mark a whole page with `if`.
- Playing observations again at the end of a path.
- Editing the release manifest.
