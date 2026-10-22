# Implementation Plan: Play a one-window tutorial unattended

**Branch**: `feat/7-play-auto` | **Date**: 2026-10-22 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/007-play-auto/spec.md`

## Summary

Add `tutorial-stepper play --auto DIR --transcript FILE`. It reads a tutorial with the format rules `check` already enforces. A sound one-window bash tutorial with no set-up script and no command marked `if=` is played: the stepper starts a tmux server on its own socket, with no tmux configuration, and one bash shell whose prompt hook appends each command's exit status to a file. Each command fence is typed as one bracketed paste. The stepper waits until the hook reports the end. There is no deadline. The output is the text the pane produced for that command, without the prompt. The transcript is TOML, one `[[command]]` table per command that ran. The first status that differs from the block stops the run and names the command. The server is killed when the run ends. `docs/play.md` and the README land with the command.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`
**Primary Dependencies**: the existing crate (clap, pulldown-cmark, toml_edit). No new dependency. `tmux` and `bash` are runtime programs on `PATH`, not crate dependencies.
**Storage**: the transcript file the author names, plus a temporary run directory removed at the end
**Testing**: cargo-nextest, as a flake check, running real tmux and bash. A command that sleeps 120 seconds is one of those tests. If the Nix build sandbox cannot host a tmux server, that test moves to a continuous-integration job outside the sandbox, and the `COMPLETE` line says so.
**Target Platform**: Linux x86_64, the same binary `check` already ships
**Project Type**: one command added to the existing binary
**Performance Goals**: a short command finishes as soon as the shell reports it. A long command is waited out. No throughput target.
**Constraints**: attributes stay `lang { key=value }`. The end of a command comes from the prompt hook. The server is not the user's tmux. The release manifest is not edited by hand.
**Scale/Scope**: one subcommand, the sound one-window examples, the refusals, one reference page

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| I. The Stepper Is the Product | Unattended play is a face on the stepper. The same command loop types the command and waits. No second stepper is introduced. |
| II. The Source Is Plain CommonMark | The command reads the fences `check` already accepts. A fence without braces stays an ordinary code block. Nothing is inferred from the command text. |
| III. Nothing Runs Unseen | The text typed into the shell is the fence body. The set-up script is not run. A status other than the declared one stops the run. |
| V. Every Tutorial Is a Test | The run exits non-zero at the first unexpected status and writes a transcript with output, status, and time. |
| VI. The Shell Says When a Command Ended | The bash prompt hook reports the end and the status. tmux hosts the shell. The stepper does not reimplement a terminal and does not use a deadline. |
| X. No Code for Any One Tutorial | The examples stay generic: an echo, a declared status, a continued line, a sleep. |
| XI. The Record Outranks the Code | This specification is committed before the command. `docs/play.md` and the README sentence land in the same commit as the behaviour they describe. |
| Development Workflow | Spec Kit acceptance before code. Conventional Commits. A new continuous-integration job is added only if the sandboxed test run cannot host tmux, and that job is named in `COMPLETE`. |

No departure. zsh, several windows, the set-up script, settings, and observations stay out of this command by refusing them or leaving them unplayed, which is the issue's scope and not a weakening of the principles.

## Project Structure

### Documentation (this feature)

```text
specs/007-play-auto/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
src/lib.rs           # the play subcommand, its help, and the exit status
src/tutorial.rs      # shared reading: the fences, the shell, the window folder
src/index.rs         # the shell, the set-up path, and each window's folder
src/play.rs          # the tmux server, the prompt hook, the transcript
tests/cli.rs         # overview and play help
tests/play.rs        # the plays, the refusals, the 120 second command
docs/play.md         # the command's reference, including the transcript
README.md            # unattended play of a one-window tutorial
examples/continued/  # one command continued over two lines
examples/declared-status/  # a command that exits 19
examples/slow/       # a command that sleeps 120 seconds
```

The fence parser, the release manifest, and the attribute spelling stay as they are. `play` calls the same read `check` uses, then drives tmux.

## The shell

The server is `tmux -f /dev/null -L tutorial-stepper-<pid>-<n>`. The session is one interactive bash, started with `--noprofile` and the stepper's hook as its rc file, in the window's folder. The hook sets `PROMPT_COMMAND` to append `$?` to a status file, sets the prompt to a per-run marker, turns off echo, and turns on bracketed paste. The stepper waits until that file grows by one line. While the pane is alive there is no deadline. If the pane dies first, the run fails and names the shell.

A fence body is sent as one bracketed paste, then Enter. The pane is piped to a log. The command's output is the new log text, with the prompt marker removed and the terminal's carriage returns dropped. The transcript is built with `toml_edit` and written before the server is killed. A guard kills the server on every return, including a failed status.

`tmux` is looked up on `PATH` before `bash`. The first one missing is named, and no server is started.
