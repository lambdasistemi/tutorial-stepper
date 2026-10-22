# Implementation Plan: Observations in the unattended run

**Branch**: `feat/11-observe` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/011-observe/spec.md`

## Summary

`tutorial-stepper play --auto` runs each `mode=observe` block where it stands, then runs that list again after the path's other blocks. The transcript records each of those runs as `[[observation]]`, with `position` set to the command before it, to `start of path`, or to `end of path`. A status the block did not declare stops the path and names the observation. `check` and `script` stay as they are. `docs/play.md` and `docs/format.md` land with the behaviour.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`
**Primary Dependencies**: the existing crate. No new crate.
**Storage**: the transcript file the author names. Commands stay `[[command]]`. Observations are `[[observation]]` tables in the same file, in run order.
**Testing**: cargo-nextest, as a flake check, running real tmux, bash, and zsh. `examples/observe` and `examples/observe-gone` join the examples the existing test job already plays under both shells.
**Target Platform**: Linux x86_64, the same binary `check` already ships
**Project Type**: the existing `play` subcommand, extended
**Performance Goals**: an observation finishes as soon as its own shell reports it. The runs at the end use the shells the path already started.
**Constraints**: attributes stay `lang { key=value }`. The end of a command still comes from the prompt hook. The release manifest is not edited by hand. No new continuous-integration job.
**Scale/Scope**: the play loop, the transcript, two examples, and the play reference

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| II. The Source Is Plain CommonMark | `mode=observe` stays a fence attribute. Nothing about an observation is inferred from its text. |
| III. Nothing Runs Unseen | The text typed into the shell is the fence body, which is on the page. |
| V. Every Tutorial Is a Test | Each observation runs on the path and again after the path's effects. An unexpected status exits non-zero and names the observation. |
| VII. A Cursor Over the Tutorial | An observation does not move the position. The transcript records the command the position was on, or the end of the path. |
| VIII. Effects Can Be Observed at Any Time | The unattended half of this principle: where the block appears, and again at the end of the path. The list the learner runs from stays in issue 15. |
| XI. The Record Outranks the Code | This specification is committed before the command changes. The reference pages land in the same commit as the behaviour. |

No departure. The stepper still cannot check the author's promise that an observation changes nothing the tutorial depends on.

## Project Structure

### Documentation (this feature)

```text
specs/011-observe/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
src/tutorial.rs      # select observations, resolve the window, name a failed observation
src/play.rs          # run the steps, write [[observation]] in run order
src/lib.rs           # play --help names the two runs
tests/play.rs        # in place, at the end, the failing end, both shells
tests/examples.rs    # the new examples count as valid tutorials
tests/script.rs      # the new examples are still bash scripts
examples/observe/    # the observation still holds at the end
examples/observe-gone/ # the observation fails only at the end
docs/play.md         # when an observation runs, and the position field
docs/format.md       # play runs observations; the reference stays play.md
README.md            # the unattended run plays observations again at the end
```

The fence parser, `check`, `script`, the release manifest, and the continuous-integration jobs stay as they are.

## What runs

A runnable block is a command or an observation. `mode=show` and a fence with no braces stay out of the list. The window is the fence's `window` when it sets one, otherwise the window of the previous command that runs, and the first block with no window uses the first window the index declares. Only a command updates that window. An observation that names a window runs there and leaves the inherited window where it was.

The path's list is those blocks in index page order, then fence order, after `if=` has selected the values in force. The play loop types each one into its window and waits for the prompt hook, as it does for a command today. It then types each observation from that list once more. A status mismatch writes the transcript of what ran, including the mismatch, and stops the path. Later paths still start when the tutorial is playing every tested path.

`drive` returns the number of commands that ran. Observations are not in that count, so `N commands played` stays the sentence it is.

## The transcript

Each step is rendered as its own table and the tables are concatenated in run order. `toml_edit` groups every table of one key together, which would pull the run at the end away from the point it happened, so the file is built from one table at a time.

An observation table has the command keys, then `position`. A command table gains no key. `path`, when the run is a tested path, stays the first key of both.

## The message

`Failure::Status` records whether the step was a command, an observation, or an observation at the end. The three sentences are `command exited`, `observation exited`, and `observation exited …, at the end of the path`. The page, the line, the body, and the two statuses stay.

## Continuous integration

No package is added. No job is added or renamed. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.
