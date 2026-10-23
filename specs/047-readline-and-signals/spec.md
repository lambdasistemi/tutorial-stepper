# Feature Specification: Readline check and signal cleanup

**Feature Branch**: `fix/47-readline-and-signals`
**Created**: 2026-10-23
**Status**: Specified
**Input**: Issue 47, "play hangs when bash has no readline, and leaves its tmux server when killed"

## User Scenarios & Testing

### User Story 1 — A shell without readline is refused before the first command (Priority: P1)

As a tutorial author whose `PATH` puts a bash without readline first (for example `stdenv`'s bash, which a Nix development shell puts first), I run `play --auto` and it stops at once with a message naming the shell and what it lacks, instead of hanging forever waiting for a prompt report that never comes.

**Why this priority**: This is the first half of the issue. The Singular demo's development shell had this bash, and the run hung for 45 seconds with nothing printed.

**Independent Test**: With `stdenv`'s bash first on `PATH`, `tutorial-stepper play --auto examples/one-window --transcript t.toml` exits non-zero at once, names bash and readline, creates no transcript, and leaves no `tmux -L tutorial-stepper-…` server.

**Acceptance Scenarios**:

1. **Given** a bash without readline first on `PATH` (`bind` fails), **When** the author runs `play --auto`, **Then** it exits 1 before the first command with `bash at <path> lacks readline: the stepper's prompt hook needs bind`, and no server is started.
2. **Given** the usual interactive bash or zsh, **When** the author runs `play --auto`, **Then** the check passes and the run plays as before.
3. **Given** a refused run, **When** it stops, **Then** no transcript file is created and `ps` shows no stepper server.

### User Story 2 — A killed run leaves no tmux server (Priority: P1)

As a tutorial author who interrupts (`SIGINT`) or terminates (`SIGTERM`) a `play --auto` run (for example via `timeout -s INT 3` on a tutorial with a long sleep), I see the stepper remove its tmux server, as it does on a normal exit.

**Why this priority**: This is the second half of the issue. Killing the run left the server running.

**Independent Test**: `timeout -s INT 3 tutorial-stepper play --auto examples/slow --transcript t.toml` (and the same with `-s TERM`) leaves no `tmux -L tutorial-stepper-…` server afterwards.

**Acceptance Scenarios**:

1. **Given** a running `play --auto` (and the same in the terminal face setup), **When** `SIGINT` arrives, **Then** the server is killed and the process exits non-zero.
2. **Given** a running `play --auto`, **When** `SIGTERM` arrives, **Then** the server is killed and the process exits non-zero.
3. **Given** a normal exit, **When** the run ends passed or failed, **Then** the server is killed as before (unchanged).

## Requirements

### Functional Requirements

- **FR-001**: Before the first command (before any server, run folder, or transcript), `play` checks the shell can run the stepper's hook: for bash, `bind` works (readline present). On failure it exits 1 with `bash at <path> lacks readline: the stepper's prompt hook needs bind`.
- **FR-002**: The check runs in unattended `drive` and in the terminal face (`start_kept`, `open_shells`), before directories are created.
- **FR-003**: On `SIGINT`/`SIGTERM`, the run kills its tmux server and removes its control directory (and the run directory when temporary, as on a normal exit), then exits 130/143.
- **FR-004**: `docs/play.md` documents `TUTORIAL_STEPPER_WINDOW` as a variable every window's shell has, set to that window's name.
- **FR-005**: Tests cover the missing readline, `SIGINT`, and `SIGTERM` (all in `nix flake check`).

### Key Entities

- **Readline check**: `bash --noprofile --norc -c 'bind -v'` must succeed; `stdenv`'s bash fails it (`bind: command not found`, exit 127).
- **Signal guard**: one thread per process waiting on `SIGINT`/`SIGTERM`, armed with the current server's socket and directories; kills the server and exits.

## Success Criteria

- **SC-001**: `nix flake check` passes.
- **SC-002**: With `stdenv`'s bash first on `PATH`, `play --auto examples/one-window` stops at once with the message and no server is left.
- **SC-003**: `timeout -s INT 3` and `timeout -s TERM 3` on `play --auto examples/slow` each leave no stepper server.
- **SC-004**: A reader of `docs/play.md` can say what `TUTORIAL_STEPPER_WINDOW` holds.

## Assumptions

- Only bash is checked for readline; zsh needs no `bind` and always passes.
- The guard kills the current server only; retained terminal-face servers after `exec attach` belong to `tmux`, not to this process.
- Exit codes 130 (`SIGINT`) and 143 (`SIGTERM`) follow the shell convention.

## Out of scope

- Supporting shells without readline (the run is refused, not worked around).
- Changing the fence spelling, the index, or the transcript format.
