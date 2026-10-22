# Implementation Plan: Settings and tested paths in the unattended run

**Branch**: `feat/10-settings` | **Date**: 2026-10-22 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/010-settings/spec.md`

## Summary

`tutorial-stepper script` and `tutorial-stepper play --auto` gain `--path NAME`. The name selects one tested path from the index. Play writes that path's values to `.env` in the run folder, and each window sources the file during shell startup, before the prompt hook and before the first command. Blocks marked `if=NAME:value` run only when the value matches. Blocks with no `if` always run. Without `--path`, play runs every tested path in index order, each in its own run folder, and exits non-zero if any path fails. `check` already rejects an `if` that names an undeclared setting or value. `docs/play.md`, `docs/script.md`, and `docs/format.md` land with the behaviour they describe.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`
**Primary Dependencies**: the existing crate. No new crate.
**Storage**: `.env` in the run folder. The transcript stays the file the author names. One extra key, `path`, is written only when the run is a tested path.
**Testing**: cargo-nextest, as a flake check, running real tmux, bash, and zsh. `examples/settings` joins the examples the existing test job already plays under both shells.
**Target Platform**: Linux x86_64, the same binary `check` already ships
**Project Type**: the existing `script` and `play` subcommands, extended
**Performance Goals**: a short command finishes as soon as its own shell reports it. A second path starts only after the first path's server is gone.
**Constraints**: attributes stay `lang { key=value }`. Values stay single words, so `.env` does not quote them. The end of a command still comes from the prompt hook. The release manifest is not edited by hand. No new continuous-integration job.
**Scale/Scope**: the index keeps the paths it already parses, script selection, the play loop, `.env`, and the three reference pages

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| II. The Source Is Plain CommonMark | `if` stays a fence attribute. Nothing about a branch is inferred from the command text. |
| III. Nothing Runs Unseen | The text typed into the shell is still the fence body. Loading `.env` is the window's startup, specified in `docs/play.md`, and it is not a transcript entry. |
| V. Every Tutorial Is a Test | Every tested path plays from a clean run folder. The command exits non-zero if any path fails, after reporting each path. |
| IX. Settings Are Chosen at the Start | The values are fixed before the first window opens. `.env` is how every window receives them. A block marked `if` belongs to that value. |
| X. No Code for Any One Tutorial | `examples/settings` is a generic two-setting tutorial. No product name is special-cased. |
| XI. The Record Outranks the Code | This specification is committed before the command changes. The reference pages land in the same commits as the behaviour they describe. |

No departure. Interactive choice stays out. A page still has no `if` of its own, which is the format version 0 already published. Observations stay out of play by the rule issue 9 left in place.

## Project Structure

### Documentation (this feature)

```text
specs/010-settings/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
src/index.rs         # keep each tested path, and each setting's default
src/tutorial.rs      # select blocks, report every path, write nothing new for check
src/play.rs          # write .env and source it from both shell hooks
src/lib.rs           # --path on script and on play
tests/script.rs      # one path's script, and an unknown path
tests/play.rs        # .env, one path, every path, both shells
tests/examples.rs    # examples/settings counts as a valid tutorial
examples/settings/   # two settings, two tested paths, one of them fails
docs/format.md       # the environment file, pointed at from the index section
docs/play.md         # --path, every path, and the exact .env syntax
docs/script.md       # --path leaves out the other branches
README.md            # unattended play runs the tested paths
```

The fence parser, the release manifest, and the continuous-integration jobs stay as they are.

## Selection

The index parser already checks each path: a unique name, and one allowed value for every setting. It drops the paths after that check. This change keeps them, and keeps each setting's default, which it already checks and then drops.

A block belongs to the values in force when it has no `if`, or when `if=NAME:value` names the value those settings give `NAME`. `script` and `play` both use that rule. `check` does not: it still counts every command fence.

The window a command runs in is still the fence's `window`, or the window of the previous command that actually runs. A block left out by `if`, by `mode=show`, or by `mode=observe` does not move that window.

`script` without `--path` does not select. It prints every branch, so the existing script of `examples/if` stays.

## One path

`--path NAME` looks the name up in the tested paths. A miss is `tutorial.toml:1: path "NAME" is not a tested path`, exit 1, before a shell and before the run folder.

The values are that path's values. Play creates the run folder, writes `.env`, creates the window folders, and only then starts tmux. The bash hook and the zsh hook both source the file when `TUTORIAL_STEPPER_ENV` is set:

```sh
if [ -n "${TUTORIAL_STEPPER_ENV:-}" ]; then
  set -a
  . "$TUTORIAL_STEPPER_ENV"
  set +a
fi
```

That block is the first thing in the hook, before `PROMPT_COMMAND` or `precmd` is installed, so the assignments are not commands and the ready line is still the first status. `set -a` exports each assignment. The launcher exports `TUTORIAL_STEPPER_ENV` as the absolute path of `<run>/.env` only when the file was written. A tutorial with no settings does not write the file and does not set the variable.

A tutorial with no tested paths ignores `--path` by rejecting it. Without `--path` it plays once. If it has settings, the values are the defaults and `.env` is still written. Blocks are selected with those defaults. The transcript has no `path` key.

A single `--path` run reports the way play reports today. Its transcript adds `path` as the first key of each command.

The set-up script and the needs check stay where they are: needs once, before the run folder; the set-up script inside the run folder, after `.env` and before the windows, and only with `--run-setup`. A declared script without `--run-setup` still stops the run first, which is why `examples/if` does not start a shell.

## Every tested path

When the tutorial declares at least one tested path and the author passes no `--path`, the tutorial-level refusals run once: format, a set-up script the author did not ask to run, that script as root, a missing or too-old program, and a `--run` directory that exists and is not an empty directory. None of those messages gain a path name.

Then each path plays, in index order, through the same one-path run. `--run DIR` means the path's folder is `DIR/<path name>`. The parent is created if it is missing and is kept. Without `--run`, each path gets a temporary folder and loses it when that path's server is killed. A path that fails to match a status, or whose set-up script exits non-zero, is reported and the next path still starts.

Standard output gains one line per path that played through: `careful: 3 commands played`. Standard error gains the existing failure text for each path that did not, with the path name and `:`  before the first line only. The exit code is 1 if any path failed.

The transcript file is written once, after the paths that started a shell. It is the concatenation of those paths' command tables. Each table starts with `path`. A path that never starts a shell contributes nothing. If none do, the file is not created.

`examples/settings` is the tutorial the verification plays. `careful` exits 0. `quick` fails on a command that declares status 19. The both-shells sweep plays it under bash and under zsh and expects that failure.

## Continuous integration

No package is added. No job is added or renamed. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.
