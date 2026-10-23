# Implementation Plan: Run an observation at any time

**Branch**: `feat/15-observe-face` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/015-observe-face/spec.md`

## Summary

`o` on the terminal face lists the observations that stand before the next command. A number and Enter runs the chosen one through the stepper, in that observation's window, and leaves the next command where it was. The face asks the stepper whether the window's shell has reported the end of the command it was last given. When it has not, the page says the window is busy and nothing is typed. Each run is appended to the run folder as an observation, with the position at which it ran. `docs/terminal.md` changes with that behaviour.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`
**Primary Dependencies**: the existing crate. No new dependency.
**Storage**: the run folder gains `.tutorial-stepper/observations.toml`, appended one table at a time. The position file is unchanged. The shell's prompt hook gains a file written when a line is accepted, before the command runs, so the stepper can tell a command given from a command ended. The first status line remains the shell becoming ready, not a command.
**Testing**: cargo-nextest, as a flake check. The new tests start a detached tmux session, send `o`, and read the page pane and the run folder. The unattended suite stays.
**Target Platform**: Linux x86_64, the same binary `play` already ships
**Project Type**: the existing terminal face
**Performance Goals**: opening the list redraws the page. It does not start a command. A busy window returns without typing.
**Constraints**: attributes stay `lang { key=value }`. The face still sends and waits through the stepper, and it does not read the screen to decide that a window is busy. The release manifest is not edited by hand. No new continuous-integration job.
**Scale/Scope**: one list, one question to the stepper, one record file, the end-to-end tests, and the reader page that describes the face.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| I. The Stepper Is the Product | The face asks the stepper to type the observation and to say whether the window is busy. It does not step on its own and it does not guess from the screen. |
| II. The Source Is Plain CommonMark | No new attribute. `mode=observe` stays the spelling already specified. |
| VI. The Shell Says When a Command Ended | Busy means the prompt hook has not reported the end of the command the shell was given. There is no timeout and no reading of the pane. |
| VII. A Cursor Over the Tutorial | Observe does not move the position. Enter, `e`, `s`, `c`, `b`, `n`, `p`, `f` and `q` stay as they are. |
| VIII. Effects Can Be Observed at Any Time | The list holds the observations already reached. A run is recorded with the position. A busy window runs nothing. |
| XI. The Record Outranks the Code | This specification is committed before the behaviour. The page that describes the list lands in the same commit as the behaviour. |

No departure. Choosing settings stays a later issue.

## Project Structure

### Documentation (this feature)

```text
specs/015-observe-face/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
src/play.rs          # the shell records a command when it is given; the stepper answers whether it has ended; one observation table is appended
src/face.rs          # o lists, a number runs, a busy window is said
src/lib.rs           # play --help names o
tests/face.rs        # detached tmux: the list, a run, a busy window, two positions
docs/terminal.md     # o, the list, the busy window, and the record in the run folder
```

The fence parser, the attribute spelling, the unattended path, and the release manifest stay as they are.

## What the face asks the stepper

The prompt hook writes one line when a command is accepted, before that command runs, and the status line it already writes when the command ends. The first status line is still the shell becoming ready. A window is busy when the lines given outnumber the endings: the ready line accounts for one status line that is not an ending. The face calls that question by the window's name and, when the answer is busy, types nothing.

The observations reached are the face's steps that stand before the next command and are observations where they appear. The run at the end of an unattended path is not in that list. Rows are numbered from 1. The number and Enter call the same step the unattended run uses. The position recorded is the next command's page and line, or `done` when the tutorial is finished. The table is the unattended observation table, without a tested-path name, appended to `.tutorial-stepper/observations.toml`.

An in-place Enter of an observation asks the same busy question, records the run at that observation's own page and line, and then advances as it does today. A mismatch there still waits for retry or skip. A mismatch on a run from the list is shown and recorded, and the next command stays.
