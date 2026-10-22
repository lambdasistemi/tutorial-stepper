# Implementation Plan: Several windows, the set-up script and the needs check

**Branch**: `feat/8-windows` | **Date**: 2026-10-22 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/008-windows/spec.md`

## Summary

Extend `tutorial-stepper play --auto` from one bash shell to one bash shell per declared window, on the same private tmux server. A command runs in the window its fence names, or in the previous command's window. The set-up script runs, when `--run-setup` is given, in the run folder before any page command and as the user who started the process. Every `needs` entry is checked before that script and before the first command: the program must be on `PATH`, and a declared minimum is the first dotted number printed by `PROGRAM --version`. `--run DIR` keeps the run folder; without it the folder is still removed at the end. A non-empty `--run` directory is refused. `docs/play.md` and the README land with the behaviour they describe.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`
**Primary Dependencies**: the existing crate (clap, pulldown-cmark, toml_edit). No new dependency. `tmux` and `bash` stay runtime programs on `PATH`.
**Storage**: the transcript the author names, plus the run folder. The run folder is a temporary directory unless `--run` names it. Hook, status and log files live in a control directory that is always removed, so a kept run folder holds the window folders and the files the tutorial wrote.
**Testing**: cargo-nextest, as a flake check, running real tmux and bash. The 120 second command from issue 7 stays in that run.
**Target Platform**: Linux x86_64, the same binary `check` already ships
**Project Type**: the existing `play` subcommand, extended
**Performance Goals**: a short command finishes as soon as its own shell reports it. Other windows' shells do not count as that report.
**Constraints**: attributes stay `lang { key=value }`. The end of a command comes from that shell's prompt hook. The server is not the user's tmux. The release manifest is not edited by hand. No new continuous-integration job.
**Scale/Scope**: the play command, three conformance examples, the play reference page

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| I. The Stepper Is the Product | Several windows are still one play loop. Each shell is a pane the stepper types into. No second stepper is introduced. |
| II. The Source Is Plain CommonMark | The window comes from the fence or from the previous command. Nothing is inferred from the command text. |
| III. Nothing Runs Unseen | The set-up script is the file the index names, and it runs only with `--run-setup`. The stepper does not raise privileges. A status other than the declared one still stops the run. |
| IV. A Tutorial Brings Its Own Environment | The needs in the index are checked before the first page, and a missing program is named. |
| V. Every Tutorial Is a Test | The run still exits non-zero at the first unexpected status and writes a transcript. |
| VI. The Shell Says When a Command Ended | Each bash prompt hook reports its own end and status. tmux hosts the shells. There is no deadline. |
| X. No Code for Any One Tutorial | The examples stay generic: two windows and `pwd`, a script that writes a file, a program name that is not installed. |
| XI. The Record Outranks the Code | This specification is committed before the command changes. `docs/play.md` lands in the same commit as the behaviour it describes. |

No departure. zsh, settings, and observations stay out of this command by the refusals issue 7 already has, which is later issues' scope and not a weakening of the principles.

## Project Structure

### Documentation (this feature)

```text
specs/008-windows/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
src/lib.rs           # --run and --run-setup on play
src/tutorial.rs      # which window a command runs in, the refusals, the needs check
src/index.rs         # each window's needs, kept with the line they came from
src/play.rs          # one shell per window, the run folder, the set-up script
tests/play.rs        # windows, --run, the script, missing and older programs
tests/cli.rs         # play help names the new options
docs/play.md         # windows, the run folder, the script, how a version is read
docs/format.md       # the sentence that points at that version rule
README.md            # unattended play of the windows a tutorial declares
examples/two-windows/
examples/setup/
examples/needs-missing/
```

The fence parser, the release manifest, and the attribute spelling stay as they are. `check` still does not run the script and does not look up programs.

## The shells

The server stays `tmux -f /dev/null -L tutorial-stepper-<id>`. The first window is `new-session`; each further window is `new-window` in that session, with the window's name as the tmux window name and the window folder as its start directory. Targets use the exact-name form so a numeric name is not read as an index.

Each shell has its own status file and its own pane log, both under the control directory. The launcher is the issue 7 launcher with that shell's status path. The stepper waits until that shell's status file grows by one line. A shell that exits first fails the run and names the shell.

A command is sent to its own pane as one bracketed paste, then Enter. The output is that pane's new log text, with the same prompt and paste-mode stripping as issue 7.

The control directory is removed when the run ends. The run folder is removed only when the author did not pass `--run`. The server is killed on every return, including a failed status and a failed set-up script.

## The set-up script and the needs

The needs check and the set-up script both happen before `new-session`. The needs check runs `PROGRAM --version` only when a minimum is declared, takes the first dotted number from both streams, and compares components as integers. The script is `bash <absolute path>` with the current directory set to the run folder, `TMUX` removed from the environment, and the rest of the environment left so `PATH` is the one the needs check used. The script's standard output is discarded on success. A non-zero status is reported with the script's name and that status, and no shell is started.

`tmux` is looked up on `PATH` before `bash`, after the needs check. A tutorial that is refused for zsh, for `if=`, or for a set-up script the author did not ask to run does not reach the needs check, so those messages stay the ones issue 7 prints.
