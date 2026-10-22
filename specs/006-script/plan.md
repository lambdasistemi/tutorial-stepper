# Implementation Plan: Print a tutorial's commands as one script

**Branch**: `feat/6-script` | **Date**: 2026-10-22 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/006-script/spec.md`

## Summary

Add `tutorial-stepper script DIR`. It reads a tutorial with the format rules `check` already enforces. A sound tutorial prints one bash script: a shebang, then each runnable block in page order, with a comment naming its page and its window. `mode=show` and ordinary code blocks are left out. Observations are included where they appear. A tutorial `check` rejects prints the same problems and no script. The command does not run anything, does not filter by setting, and does not include the set-up script. `docs/script.md` and a README sentence land with the command. `paolino/runmd` is named there as superseded.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`
**Primary Dependencies**: the existing crate (clap, pulldown-cmark, toml_edit). No new dependency.
**Storage**: files only
**Testing**: cargo-nextest, as a flake check. The script's text is asserted through the binary, and `bash -n` is run on every sound example.
**Target Platform**: Linux x86_64, the same binary `check` already ships
**Project Type**: one command added to the existing binary
**Performance Goals**: printing a small tutorial finishes immediately. No throughput target.
**Constraints**: attributes stay `lang { key=value }`. The command runs nothing. No second attribute spelling. The release manifest is not edited by hand.
**Scale/Scope**: one subcommand, the four sound examples, the invalid examples as refusals, one reference page

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| I. The Stepper Is the Product | `script` prints. It does not step, and it is not a face that sends commands to a shell. |
| II. The Source Is Plain CommonMark | The command reads the fences `check` already accepts. A fence without braces stays an ordinary code block. Nothing is inferred from the command text, including the window. |
| III. Nothing Runs Unseen | The command runs nothing: not the commands, not the set-up script. |
| X. No Code for Any One Tutorial | The examples stay generic. The README states that `paolino/runmd` is superseded by `tutorial-stepper script`. |
| XI. The Record Outranks the Code | This specification is committed before the command. `docs/script.md` and the README sentence land in the same commit as the behaviour they describe. |
| Development Workflow | Spec Kit acceptance before code. No new continuous-integration job. Conventional Commits. |

No departure. Selecting a tested path would be a departure into issue 10, so the script prints every runnable block, including every `if=` branch.

## Project Structure

### Documentation (this feature)

```text
specs/006-script/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
src/lib.rs           # the script subcommand, its help, and the exit status
src/fence.rs         # the fence body, copied into the script
src/tutorial.rs      # shared reading: problems, or the runnable blocks
tests/cli.rs         # overview and script help
tests/examples.rs    # the sound examples' scripts, and the invalid refusals
docs/script.md       # the command's reference
README.md            # paolino/runmd is superseded
```

The index parser, the flake, and the workflows stay as they are. `script` calls the same read `check` uses, then prints.
