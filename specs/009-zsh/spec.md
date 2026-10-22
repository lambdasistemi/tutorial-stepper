# Feature Specification: zsh shells

**Feature Branch**: `feat/9-zsh`
**Created**: 2026-10-22
**Status**: Specified
**Input**: Issue 9, "zsh shells"

## User Scenarios & Testing

### User Story 1 — An author declares zsh and the tutorial plays (Priority: P1)

A tutorial author sets `shell = "zsh"` in the index and runs `tutorial-stepper play --auto DIR --transcript FILE`. Each window is a zsh. The prompt hook reports the end of each command and its exit status, the same report the bash hook already makes. The transcript has the same keys. A command whose status is not the one the block declares stops the run and names that command. When the index omits `shell`, or sets `shell = "bash"`, the windows are bash, as they are today.

**Why this priority**: macOS learners use zsh. The index already accepts the declaration. Unattended play still refuses it.

**Independent Test**: A one-window tutorial with `shell = "zsh"` and `echo hi` exits 0. The transcript records that command, the output `hi`, and status 0. The same page with no `shell` key still exits 0 under bash. `examples/status` with `shell = "zsh"` exits non-zero and names `echo not yet`.

**Acceptance Scenarios**:

1. **Given** `shell = "zsh"`, **When** the author plays the tutorial, **Then** each window is a zsh on the stepper's tmux server, the prompt hook reports each command's end and exit status, and the transcript has the same keys as a bash run: `page`, `window`, `block`, `command`, `status`, `start`, `end`, `output`.
2. **Given** a command written over several lines, **When** the author plays it under zsh, **Then** the shell runs it as one command, sent as one bracketed paste, and the transcript has one entry.
3. **Given** no `shell` key, or `shell = "bash"`, **When** the author plays the tutorial, **Then** each window is bash and the outcome is unchanged.
4. **Given** a command whose reported status is not the one the block declares, **When** the tutorial's shell is zsh, **Then** the run stops, standard error names the page, the line, the command, the status the shell reported, and the status the block declared, and later commands do not run.
5. **Given** zsh is not on `PATH`, **When** the tutorial declares zsh, **Then** the message names zsh, no server starts, and the transcript is not created.
6. **Given** the valid examples under `examples/`, **When** continuous integration runs, **Then** each example is played under bash and under zsh and ends with the outcome below.

### Edge Cases

- The fence language is not the shell. A fence tagged `bash` still runs in zsh when the index says zsh. `tutorial-stepper script` still prints one bash script and does not consult `shell`.
- A set-up script is still bash. With `--run-setup` it runs before any page command, including when the windows are zsh. Without `--run-setup` the run still stops and names the script, including `examples/if`, which declares zsh. That tutorial is no longer refused for declaring zsh.
- A command marked `if=` is still refused before a shell starts. An observation and a block marked `mode=show` are still not played.
- A missing program named in `needs` is still reported before a shell starts, whether the index says bash or zsh.
- A command that sleeps is waited out under zsh. The stepper does not decide that it ended.
- The transcript's `output` is the text the command wrote, without the prompt and without the command line the shell displayed.
- The server is gone when the run ends, whether the run passed or failed.

## Requirements

### Functional Requirements

- `play --auto` no longer refuses `shell = "zsh"`.
- The index shell is the shell of every window. An absent `shell` is bash. `check` already enforces that the value is `bash` or `zsh`.
- Each window loads the stepper's hook for that shell. The bash hook stays `PROMPT_COMMAND`. The zsh hook is `precmd`. Both append one line, the exit status of the command that just ended, before the prompt is drawn. The first line is the shell becoming ready, not a command.
- A command is sent as one bracketed paste, then Enter, in either shell. A fence body of several lines is one paste.
- The stepper looks up `tmux`, then the declared shell. The first one missing is named, and nothing is started. The set-up script still looks up `bash`, because that script is bash.
- The transcript keys do not change. `status` is the status the hook reported.
- Continuous integration plays every valid example under both shells. The test environment provides zsh. No new job is added.

### The examples under both shells

| Example | Outcome under either shell |
| --- | --- |
| `one-window`, `continued`, `declared-status`, `mode`, `two-windows` | Exit 0. The same commands, statuses, and outputs, except a working directory, which ends with that window's folder. |
| `slow` | Exit 0. The command is waited out. |
| `setup`, played with `--run-setup` | Exit 0. The script runs with bash, then the page commands run in the tutorial's shell. |
| `status` | Exit 1. The message names `echo not yet`. The transcript holds that command. |
| `needs-missing` | Exit 1 before a shell starts. The message names `tutorial-stepper-missing-program`. No transcript. |
| `if` | Exit 1 before a shell starts. The message names `setup.sh`. No transcript. |

## Success Criteria

- An author who sets `shell = "zsh"` can play a tutorial unattended and read, in the transcript, each command's status and output.
- An author who leaves `shell` unset gets the bash run this command already gives.
- A reader of `docs/play.md` can say which shell starts, what the zsh hook reports, and that a missing zsh is named before anything starts.

## Assumptions

- Issue 10 still owns settings and tested paths. A command marked `if=` is refused.
- Issue 11 still owns observations played again at the end. `mode=observe` is not played.
- The set-up script stays a bash script. The stepper does not rewrite it for zsh.
- Fish and any other shell stay out. An unknown shell is still a format error at the index line.

## Out of scope

- The interactive terminal face.
- Changing the printed script.
- Installing zsh for the learner. The stepper still installs nothing. Continuous integration is the place that provides zsh for the examples.
