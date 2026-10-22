# Feature Specification: Observations in the unattended run

**Feature Branch**: `feat/11-observe`
**Created**: 2026-10-23
**Status**: Specified
**Input**: Issue 11, "Observations in the unattended run"

## User Scenarios & Testing

### User Story 1 — An author plays observations with the path (Priority: P1)

A tutorial author marks blocks `mode=observe` and runs `tutorial-stepper play --auto DIR --transcript FILE`. Each observation runs in its window where it stands on the page, like a command. After the path's other blocks, every observation runs again. The transcript marks those runs as observations and records the position each one ran at. A status the block did not declare stops the path and names the observation.

**Why this priority**: This is the command the issue names. The format, `check`, and `script` already accept `mode=observe`. Unattended play still skips those blocks.

**Independent Test**: `examples/observe` plays two commands and one observation. The observation's output is `seen` where it stands and again after the last command. `examples/observe-gone` plays the same observation, then a command removes the file the observation looks at. The run exits 1 and names that observation. `examples/mode` still plays its one command, and now also plays `echo observed` twice. A block marked `mode=show` stays out of the transcript.

**Acceptance Scenarios**:

1. **Given** an observation, **When** the author plays the tutorial, **Then** it runs in its declared window at the point it appears on the page. When it names no window, it runs in the window the next command would inherit. Running it does not change that window.
2. **Given** a path that played through, **When** the last block of that path has finished, **Then** every observation that belongs to the path runs again, in index page order and then fence order, in the same windows as the first time.
3. **Given** an observation whose reported status is not the one it declares (zero when it declares none), **When** that happens on the run at the end, **Then** the path stops, the command exits 1, and the message names the observation, its page and line, the body, the two statuses, and that this was at the end of the path. A later observation is not run.
4. **Given** the same mismatch where the observation appears on the page, **When** the author plays the path, **Then** the path stops there. Later commands are not run, and the observations are not run again at the end. The message names the observation and does not say it was at the end of the path.
5. **Given** a transcript of a path that played an observation, **When** the author reads it, **Then** each run is an `[[observation]]` table, separate from `[[command]]`. The run where it appears records the page and block of the command that ran before it. The run at the end records `end of path`. The two runs are two tables.

### Edge Cases

- An observation before any command of the path records `start of path`. There is no command before it.
- Several observations in a row record the same command: the latest command that ran. An observation does not move the position, so it is not the position of the next observation.
- An observation marked `if=NAME:value` runs, in place and at the end, only when the path has that value. One that does not belong to the path is left out and does not change the inherited window. `mode=show` is still left out.
- The end runs happen inside the path's shells, after the last block and before the server is killed, so they see the effects of the commands. The next tested path starts from its own clean run folder and runs only its own observations.
- A path with no observations writes only `[[command]]` tables, as it does today.
- `N commands played` counts commands, not observations. `check` still does not count an observation as a command.
- The set-up script, the needs check, `.env`, and `--path` are unchanged. A refusal before a shell still writes no transcript.
- A tested path writes `path` as the first key of each observation, the same way it does for a command. A run that is not a tested path writes no `path` key.

## Requirements

### Functional Requirements

- `play --auto` runs each `mode=observe` block that belongs to the path, in page order, in the window it declares or in the inherited window.
- After those blocks, it runs that same list again. A failure on either run stops that path and names the observation.
- The transcript records each of those runs as `[[observation]]`, with the same keys as a command, plus `position`.
- `position` is `PAGE:LINE` of the command that ran before this observation, or the words `start of path` when none has, or the words `end of path` for a run at the end. `PAGE` is the page path from the index. `LINE` is the line of that command's opening fence.
- A block marked `mode=show`, and a fence with no braces, still do not run.
- No new continuous-integration job. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.

### The transcript

A command stays a `[[command]]` table with the keys it has today. An observation is a separate table. For the observation in `examples/observe`, the two tables are:

```toml
[[observation]]
page = "look.md"
window = "bob"
block = 5
command = "test -s note && echo seen\n"
status = 0
start = "2026-10-23T00:00:00.000000000Z"
end = "2026-10-23T00:00:00.000000000Z"
output = "seen\n"
position = "look.md:1"

[[observation]]
page = "look.md"
window = "bob"
block = 5
command = "test -s note && echo seen\n"
status = 0
start = "2026-10-23T00:00:00.000000000Z"
end = "2026-10-23T00:00:00.000000000Z"
output = "seen\n"
position = "end of path"
```

`page` and `block` name the observation on the page. `position` names where the run happened. The command tables for `echo ready > note` and `echo still >> note` sit in run order: the first command, the observation, the second command, the observation again.

A tested path inserts `path` before `page` on every observation table.

### The failure

An observation that exits 1 where the block declares no status, on the run at the end, prints:

```text
look.md:5: observation exited 1, expected 0, at the end of the path
test -s note && echo seen
```

The file is the tutorial directory joined with the page, as it is for a command. A mismatch where the observation appears on the page uses the same line without `, at the end of the path`. A command keeps `command exited`.

### `examples/observe`

One window, `bob`. The page creates `note`, observes that the file is non-empty, then appends to it. The observation prints `seen` both times. The run exits 0 and prints `2 commands played`.

### `examples/observe-gone`

The same observation, and the last command removes `note`. The observation prints `seen` where it appears. The run at the end exits 1. The command exits 1, standard output is empty, and standard error is the failure above. The transcript holds the commands, the successful observation, and the failed observation with `position` set to `end of path`.

### `examples/mode`

The shown block stays out. `echo observed` runs where it appears, with `position` set to `start of path`, and again with `position` set to `end of path`. The run still prints `1 command played`.

## Success Criteria

- An author who plays `examples/observe` can point at two observation tables, one at the command before the block and one at the end of the path, both with output `seen`.
- An author who plays `examples/observe-gone` gets a non-zero exit whose message names that observation and says the failure was at the end of the path.
- A reader of `docs/play.md` can say when an observation runs, what `position` contains, and how an observation failure differs from a command failure.

## Assumptions

- Running an observation on demand, and saying the window is busy, stay in issue 15. This command always waits until the shell's prompt says it can type.
- The author promises that an observation changes nothing the tutorial depends on. The stepper does not check that promise.
- An observation uses the same expected status as a command: the `status` attribute, or 0.
- The shell that runs an observation is the path's own shell for that window, not a new window created for observations.

## Out of scope

- The interactive list of observations.
- A new fence attribute.
- Editing the release manifest.
