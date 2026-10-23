# Implementation Plan: Push a command to edit it before it runs

**Branch**: `feat/13-push` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/013-push/spec.md`

## Summary

`e` on the terminal face pastes the marked block into its window and does not press Enter. The learner edits in that window and presses Enter. The bash and zsh prompt hooks report the line the shell accepted, and the stepper stores that line for every command it runs, typed or pushed. The page shows it, and the transcript's `command` is it. `docs/terminal.md` and `docs/play.md` change with that behaviour.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`
**Primary Dependencies**: the existing crate. No new dependency. The hooks are the shell's own: bash reads the line readline is about to accept, zsh reads `preexec`. `fc` and `history` are the wrong source. On bash 5.3 a paste of several lines is stored as one history entry per line, and a continued line is stored with the continuation already joined, so the record would drop the learner's edit.
**Storage**: the run folder, unchanged. The hook keeps the accepted line in a shell variable and writes it beside the status file when the command ends, before the status line. A command on the page cannot overwrite that variable by writing a file.
**Testing**: cargo-nextest, as a flake check. The new test starts a detached tmux session, sends `e`, edits the shell pane, and reads the page. The unattended suite stays, including the comparison of bash and zsh transcripts.
**Target Platform**: Linux x86_64, the same binary `play` already ships
**Project Type**: the existing terminal face and the same prompt hook `--auto` uses
**Performance Goals**: a pushed command runs when the learner presses Enter, and ends when the prompt hook says so
**Constraints**: attributes stay `lang { key=value }`. The face still sends and waits through the stepper. The shell is bash 5.1 or newer, or zsh. The release manifest is not edited by hand. No new continuous-integration job.
**Scale/Scope**: the prompt hooks, the paste that does not submit, the page's record of the line that ran, one end-to-end test, and the two reader pages that describe play.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| I. The Stepper Is the Product | The face asks the stepper to paste and to wait. It does not decide that a command ended. |
| II. The Source Is Plain CommonMark | The fence is unchanged. The record is not a new attribute. |
| III. Nothing Runs Unseen | The pushed command is the block marked on the page. An edit is shown after it runs. A status other than the declared one is still shown. |
| VI. The Shell Says When a Command Ended | The same prompt hook reports the end, the status, and the line. There is no deadline. |
| VII. A Cursor Over the Tutorial | `e` pushes. Enter runs. The record holds the line that ran, including an edit. Back is unchanged. |
| XI. The Record Outranks the Code | This specification is committed before the behaviour. The pages that describe push and the transcript land in the same commits as the behaviour. |

No departure. Folding the explanations and choosing settings stay later issues.

## Project Structure

### Documentation (this feature)

```text
specs/013-push/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
src/play.rs          # paste without Enter; the hook reports the accepted line
src/face.rs          # e pushes; the page shows the line that ran
src/lib.rs           # play --help names e
tests/face.rs        # detached tmux: paste, edit, recorded text
tests/play.rs        # the transcript command is the line the shell ran
docs/terminal.md     # e, and the line the page shows
docs/play.md         # the transcript command comes from the hook
```

The fence parser, the attribute spelling, and the release manifest stay as they are.

## The line the shell accepted

The unattended run and the face both paste a block as one bracketed paste. Enter submits it. Push is that paste without Enter. The shell's own bracketed paste, which bash 5.1 turns on and zsh already has, keeps a newline in the paste from running a line early.

Bash records the readline buffer at the moment Enter is accepted, then the existing `PROMPT_COMMAND` writes that text and the status. zsh records the argument of `preexec`, and `precmd` writes it with the status. The first prompt is still only the shell becoming ready. A prompt redrawn because the learner pressed Ctrl-C at the prompt writes no status, so the page keeps waiting until Enter submits a line.

The stored command is that text, plus a final newline when the shell did not include one. A block the stepper typed, and nobody edited, is then the fence body under both shells. The transcript and the page both take this text. The error that names a status takes it too, so the line the learner sees is the line that ran.

The tutorial shell does not read or write the learner's history file. History expansion is off, so the line that runs is the line that was accepted.
