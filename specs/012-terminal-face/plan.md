# Implementation Plan: Play a tutorial page by page in the terminal

**Branch**: `feat/12-terminal-face` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/012-terminal-face/spec.md`

## Summary

`tutorial-stepper play DIR` becomes the terminal face. It starts the same private tmux server as `play --auto`, one shell per window, and attaches the learner's terminal. A page pane beside the next command's window renders the page with `pulldown-cmark`, wrapped to the pane's width. Enter, skip, interrupt, back and the page turns call the stepper's existing send and wait, not a second copy. `q` closes the page and leaves the server. The run folder defaults to a directory under the home directory named after the tutorial, and `--run DIR` overrides it. The position and the server's name live in that folder, so a later `play` returns to the same windows. `--auto` keeps its flags, its transcript and its habit of stopping the server. `docs/terminal.md` lands with the face.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`
**Primary Dependencies**: the existing crate (clap, pulldown-cmark, toml_edit). No new dependency. `tmux` and the shell named by the index stay runtime programs on `PATH`. The page puts the terminal into single-character mode with `stty`, which is part of the terminal, not a program the stepper installs.
**Storage**: the run folder. Unattended `--run` is unchanged: a non-empty directory is still refused, and the server is still killed at the end. The face keeps the folder. Its private directory inside the run folder holds the server name, the control files the shells already use, and the position.
**Testing**: cargo-nextest, as a flake check. The new test starts a detached tmux session, runs the face in it, and sends keys. The unattended suite stays as it is, including the command that sleeps 120 seconds.
**Target Platform**: Linux x86_64, the same binary `check` already ships
**Project Type**: the existing `play` subcommand. Without `--auto` it is the face. With `--auto` it is the unattended run.
**Performance Goals**: a key is acted on when it is read. A command ends when its own shell's prompt hook says so, as it does for `--auto`.
**Constraints**: attributes stay `lang { key=value }`. The face must not send a command or notice its end by any path the unattended run does not use. The release manifest is not edited by hand. No new continuous-integration job.
**Scale/Scope**: the play command, the page renderer, one end-to-end test, the terminal-face page, and a short pointer from the unattended page and the README.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| I. The Stepper Is the Product | The face calls the stepper's send and wait. `--auto` calls those same functions. The page only renders and reads keys. |
| II. The Source Is Plain CommonMark | The page is rendered from the Markdown file. Nothing about a command is inferred from its text. |
| III. Nothing Runs Unseen | Enter runs the command that is marked on the page. The set-up script is shown and runs only after Enter. A status other than the declared one is shown with a choice. |
| IV. A Tutorial Brings Its Own Environment | The needs check runs before a server starts, and a missing program is named. |
| V. Every Tutorial Is a Test | `--auto` is unchanged, and the face itself is driven from a test. |
| VI. The Shell Says When a Command Ended | The face waits on the same prompt hook. `c` sends Ctrl-C and then waits for that hook. There is no deadline. |
| VII. A Cursor Over the Tutorial | The position is the next command. Back moves it and leaves the effect. Turning the page does not move it. Push and folding stay out, as the issue says. |
| IX. Settings Are Chosen at the Start | The face does not ask. It plays the defaults, shows them, and says once when that combination is not a tested path. Choosing is a later issue. |
| X. No Code for Any One Tutorial | The examples stay the generic ones this repository already has. |
| XI. The Record Outranks the Code | This specification is committed before the command changes. The terminal-face page lands in the same commit as the behaviour it describes. |

No departure. Push, folding, observations on demand and the settings chooser are the following issues, not a weakening of the principles.

## Project Structure

### Documentation (this feature)

```text
specs/012-terminal-face/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
src/lib.rs           # play without --auto is the face; --auto keeps its flags
src/tutorial.rs      # the blocks the face steps, without the run at the end
src/play.rs          # one function sends a command and waits for the hook
src/render.rs        # the page, from the Markdown parser, wrapped to a width
src/face.rs          # the pane, the keys, the run folder, resume
tests/face.rs        # detached tmux, keys sent with tmux
tests/cli.rs         # help names the face first; a bare play is no longer an error
docs/terminal.md     # the keys, the run folder, resume
docs/play.md         # one pointer: the face is the other page
README.md            # the face is what a learner runs
```

The fence parser, the attribute spelling, the unattended transcript and the release manifest stay as they are.

## The page and the stepper

The unattended loop and the face both call one function that types a block as a bracketed paste, presses Enter, and waits until that shell's status file grows by one line. The face passes a check that runs while it waits: `c` sends Ctrl-C to that pane and the wait continues until the hook reports the end. Skip, back and turning the page do not call it.

The page process is a pane of the stepper's server. The process the learner started attaches that server to the learner's terminal, and replaces itself with the attach when the terminal is a terminal. When it is not, it prints the attach command and leaves the server up, which is how a test can own the session.

`q` closes the page pane. The server is not killed. The position is written before the pane exits, including while a status choice is waiting, so a later `play` shows the same choice instead of running the command again.

## The run folder

`<name>` is the tutorial directory's last component. The default folder is the home directory joined with `<name>-run`. `--run` replaces the whole folder, not only the name. A foreign non-empty directory is refused with the same shape of message as an unattended `--run` that is not empty. A folder that already holds this face's state is reused.

The server's socket name, the prompt marker and the control directory are recorded in the run folder because the shells' hook holds those paths. They stay for the life of the server. A dead server is started again, and the control directory is created anew so the old status lines are not read as this run's commands. The position file is kept.
