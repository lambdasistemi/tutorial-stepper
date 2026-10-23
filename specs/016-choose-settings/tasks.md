# Tasks: Choose settings before the first page

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/016-choose-settings/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: The choice, the file, and the new run

Test first. A detached tmux session and the face are the subject. `--auto` stays green.

- [x] A failing test that the page lists each setting's allowed values with the default selected, and that the keys are written beside the list, before the first page.
- [x] A failing test that a combination that is not a tested path is announced on the choice, and that confirming plays the tutorial without repeating the announcement.
- [x] A failing test that Enter writes `.env` in the unattended form, that the window has those values, and that a command which reads a setting shows them beside it.
- [x] A failing test that quitting the choice writes no `.env` and starts no shell.
- [x] A failing test that a later visit continues the same run, and that asking to choose again moves the earlier folder aside without deleting it.
- [x] The smallest change that makes those tests pass.
- [x] `docs/terminal.md` describes the choice, the announcement, the values beside the commands, and the kept folder, in the same commit as that behaviour. `play --help` names the choice and stays within the line limit.

## Phase 3: Verification

- [x] `nix flake check`
- [x] `nix build .#default`, then in a detached tmux session `tutorial-stepper play` on a tutorial with settings, driven with `tmux send-keys`. The capture shows the choice, the announcement, the values beside the command, and the earlier folder kept.
- [x] The documentation presentation check on `README.md` and `docs/`.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phase 2 leaves the suite green. Phase 3 runs last. No new continuous-integration job. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.
