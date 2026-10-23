# Implementation Plan: Choose settings before the first page

**Branch**: `feat/16-choose-settings` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/016-choose-settings/spec.md`

## Summary

When a tutorial declares settings and the run has not chosen yet, the terminal face shows the choice in the page pane before any shell starts. The learner moves between settings and values with the keys written beside the list. Enter writes `.env` in the form the unattended run already writes, then the windows start and load that file. A combination that is not a tested path is named on the choice and is not repeated on the tutorial pages. A command that reads a setting shows the value in force beside it. A later visit can continue, or start a new run, which moves the earlier folder aside and does not delete it.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`
**Primary Dependencies**: the existing crate. No new dependency.
**Storage**: `.env` in the run folder, the same file the unattended run writes. The position file is unchanged. A new run renames the earlier folder to the next free `<name>.N` beside it.
**Testing**: cargo-nextest, as a flake check. The new tests start a detached tmux session, send keys, and read the page pane, `.env`, and the folder that was moved aside.
**Target Platform**: Linux x86_64, the same binary `play` already ships
**Project Type**: the existing terminal face
**Performance Goals**: changing the selection redraws the page. It does not start a shell.
**Constraints**: attributes stay `lang { key=value }`. The windows load `.env` while starting, through the same hook as the unattended run. The release manifest is not edited by hand. No new continuous-integration job.
**Scale/Scope**: the choice screen, the `.env` the face writes, the line beside a command that reads a setting, the new-run rename, the end-to-end tests, and the reader page that describes the face.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| I. The Stepper Is the Product | The face writes the values and then uses the same session the unattended run uses to start the shells and to type commands. It does not step on its own. |
| II. The Source Is Plain CommonMark | No new attribute. `if=NAME:value` stays the spelling already specified. |
| VII. A Cursor Over the Tutorial | The choice happens before the first page. Enter, `e`, `s`, `c`, `b`, `n`, `p`, `f`, `o` and `q` keep their meaning once the tutorial is on the page. |
| IX. Settings Are Chosen at the Start | The learner chooses once, before the first page. Any combination plays. An untested one is announced once. The page shows the values beside the commands that use them. Choosing again starts a new run. |
| XI. The Record Outranks the Code | This specification is committed before the behaviour. The page that describes the choice lands in the same commit as the behaviour. |

No departure.

## Project Structure

### Documentation (this feature)

```text
specs/016-choose-settings/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
src/tutorial.rs      # the face book carries the settings, the tested paths, and the values in force
src/play.rs          # a server whose first pane is the choice, then one shell per window loading .env
src/face.rs          # the choice, the announcement, the new run, the line beside a command
src/render.rs        # a note drawn beside a code block
src/lib.rs           # play --help names the choice
tests/face.rs        # detached tmux: the list, an untested combination, .env, a new run
docs/terminal.md     # the choice, the announcement, and the earlier folder kept
```

The fence parser, the attribute spelling, the unattended path, and the release manifest stay as they are.

## What the page does before the first page

The choice is the only pane until Enter. No tutorial shell has been started, so nothing has loaded `.env`. The selected value of each setting begins as the default. The combination is tested when some tested path sets every setting to the same value. When none does, and the index declares at least one tested path, the choice says `This combination is not a tested path.`

Enter writes `.env` and then opens the windows in the same server. The shell hook sources that file with every assignment exported, as it does for the unattended run, and only then installs the prompt hook. The page moves beside the first command's window. Commands are the blocks that belong to the chosen values. A command uses a setting when its text contains `$NAME` or `${NAME}`. The note beside it is `NAME=value` for each such setting, in index order.

`q` on the choice removes this run's files, writes no `.env`, and stops the server.

A later page, when `.env` is already there, shows the values and waits. Enter continues at the saved place. `n` moves the page pane to a window of its own, stops the tutorial windows, renames the run folder to the next free name beside it, and shows the choice again at the original path. The renamed folder is left as it was.
