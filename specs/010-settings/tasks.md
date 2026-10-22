# Tasks: Settings and tested paths in the unattended run

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/010-settings/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: One path in the printed script

Test first. The binary is the subject.

- [x] A failing test that `script examples/if --path external` prints `echo external` and does not print `echo internal`, and that `--path internal` does the opposite.
- [x] A failing test that a shared block is printed with the matching branch and that the other branch is left out, including an observation, and that `mode=show` stays left out.
- [x] A failing test that `--path` of a name that is not a tested path exits 1, names the path, and prints nothing.
- [x] A failing test that a tutorial `check` rejects still prints those problems when `--path` is passed.
- [x] The smallest change that makes those tests pass. Without `--path`, every branch is still printed.
- [x] `docs/script.md` describes `--path` in the same commit. `script --help` names it and stays within the line limit.

## Phase 3: One path in the unattended run

Test first. The binary, a real shell, and the run folder are the subject.

- [x] A failing test that `play --auto --path careful --run DIR` on a two-setting tutorial writes `.env` as two `NAME=value` lines, plays only that path's blocks, and records `external slow` as the shared command's output.
- [x] The same test under zsh.
- [x] A failing test that the other path's block is absent from the transcript, and that the transcript's commands carry `path`.
- [x] A failing test that an unknown `--path` writes no transcript and creates no run folder.
- [x] A failing test that a tutorial with no settings still plays, writes no `.env`, and writes no `path` key.
- [x] The smallest change that makes those tests pass. `.env` is written before the windows open and sourced from the bash hook and the zsh hook before the prompt hook. Without `--path`, a block marked `if=` is still refused, until phase 4.
- [x] `docs/play.md` and `docs/format.md` describe `.env` and `--path` in the same commit. `play --help` names `--path` and stays within the line limit.

## Phase 4: Every tested path

Test first.

- [x] A failing test that a tutorial with no tested paths and one setting plays the default, writes that default into `.env`, and runs the matching block.
- [x] A failing test that two tested paths play without `--path`, that `--run DIR` holds one sub-folder per path, and that a failing path does not stop the later path.
- [x] The report names each path, and the exit code is non-zero when either path fails.
- [x] `examples/settings`, with the outcomes in the specification. `check`, `script`, and the both-shells play sweep include it.
- [x] The smallest change that makes those tests pass. The refusal `play --auto does not choose a setting` is removed.
- [x] `docs/play.md` and the README describe the every-path run in the same commit.

## Phase 5: Verification

- [ ] `nix flake check`
- [ ] `nix build .#default`, then `play --auto examples/settings --path careful` and the same command without `--path`.
- [ ] `script examples/settings --path careful` prints only that path. `check` on an `if` that names an undeclared setting or value exits non-zero.
- [ ] The documentation presentation check on `README.md` and `docs/`.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phases 2, 3, and 4 each leave the suite green. Phase 5 runs last. No new continuous-integration job. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.
