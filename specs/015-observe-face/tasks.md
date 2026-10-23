# Tasks: Run an observation at any time

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/015-observe-face/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: The list, the run, and the record

Test first. A detached tmux session and the face are the subject. `--auto` stays green.

- [x] A failing test that `o` lists only the observations that stand before the next command, that going back takes one off the list, and that turning the page does not add one.
- [x] A failing test that a number and Enter run that observation in its window, leave the next command where it was, and append a record with that position.
- [x] A failing test that a window whose shell has not reported the end of the command it was given is named on the page, and that the observation is not typed and not recorded.
- [x] A failing test that the same observation, run at two positions, is two records whose positions differ.
- [x] The smallest change that makes those tests pass. The shell records a command when it is given. The face asks the stepper. The list and the record are that answer.
- [x] `docs/terminal.md` describes `o`, the list, the busy window, and the record, in the same commit as that behaviour. The README names `o` where it already tells the learner the observations stay on a list. `play --help` names `o` and stays within the line limit.

## Phase 3: Verification

- [x] `nix flake check`
- [x] `nix build .#default`, then in a detached tmux session `tutorial-stepper play` on a page with an observation, driven with `tmux send-keys`. The capture shows the list, a run at one position, and the same observation at a later position.
- [x] The documentation presentation check on `README.md` and `docs/`.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phase 2 leaves the suite green. Phase 3 runs last. No new continuous-integration job. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.
