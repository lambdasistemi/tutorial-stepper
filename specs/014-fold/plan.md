# Implementation Plan: Fold the explanations away

**Branch**: `feat/14-fold` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/014-fold/spec.md`

## Summary

`f` on the terminal face toggles a folded view of the page the learner is looking at. The folded view lists that page's commands and observations, each with the window the stepper will run it in, and marks the next command. Prose, a block marked `mode=show`, and a fence without braces are left out. The stepping keys are the same match they already are. The choice is a flag beside the position in the run folder, so a later `play` draws the same view. `docs/terminal.md` changes with that behaviour.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`
**Primary Dependencies**: the existing crate. No new dependency. The folded lines are drawn by the same renderer as the full page.
**Storage**: the position file in the run folder gains `folded`. Absent means unfolded, so a run folder written by an earlier build resumes with the explanations showing.
**Testing**: cargo-nextest, as a flake check. The new test starts a detached tmux session, sends `f`, steps, and reads the page pane. The unattended suite stays.
**Target Platform**: Linux x86_64, the same binary `play` already ships
**Project Type**: the existing terminal face
**Performance Goals**: `f` redraws the page. It does not start a command.
**Constraints**: attributes stay `lang { key=value }`. The face still sends and waits through the stepper. The release manifest is not edited by hand. No new continuous-integration job.
**Scale/Scope**: one flag on the cursor, one rendering of the commands, one end-to-end test, and the reader page that describes the face.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| I. The Stepper Is the Product | Folding changes what the page draws. The same stepper runs, pushes, skips, interrupts and goes back. |
| II. The Source Is Plain CommonMark | The page source is unchanged. The folded view is a way of drawing it. |
| VII. A Cursor Over the Tutorial | The verbs are unchanged. Stepping works the same in both views. The cursor gains only whether the explanations are hidden. |
| IX. Settings Are Chosen at the Start | The values in force stay at the top in both views. Choosing them is a later issue. |
| XI. The Record Outranks the Code | This specification is committed before the behaviour. The page that describes folding lands in the same commit as the behaviour. |

No departure. Choosing settings and running an observation on demand stay later issues.

## Project Structure

### Documentation (this feature)

```text
specs/014-fold/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
src/render.rs        # the commands of one page, each named with its window
src/face.rs          # f toggles; the choice is saved with the position
src/lib.rs           # play --help names f
tests/face.rs        # detached tmux: fold, step, unfold, resume
docs/terminal.md     # f, the folded page, and the choice in the run folder
```

The fence parser, the attribute spelling, the stepper, and the release manifest stay as they are.

## What the folded page draws

The commands are the steps the face already has for the viewed page, in that order. Each one is the window name, then `next` when it is the command Enter will run, or `observation` when it is an observation and not that command, then the fence body. A command whose fence names no window uses the window the step already resolved.

`f` flips the flag and writes the position. The key match that runs, pushes, skips, interrupts, goes back, turns the page and quits does not look at the flag. The set-up script is drawn before that loop, so `f` there is ignored.

The position file gains one line, `folded = true` or `folded = false`, written whenever the position is written. Reading a position that has no such line draws the full page.
