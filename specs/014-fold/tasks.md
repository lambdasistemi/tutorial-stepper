# Tasks: Fold the explanations away

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/014-fold/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: Fold, step, and keep the choice

Test first. A detached tmux session and the face are the subject. `--auto` stays green.

- [ ] A failing test that `f` leaves each command with its window and drops the explanation, the shown block, and the ordinary code block, and that `f` again brings the explanation back.
- [ ] A failing test that Enter, `e`, `s` and `b` do the same thing while the page is folded, and the page stays folded.
- [ ] A failing test that the choice holds on the next page and after quitting and playing again.
- [ ] The smallest change that makes those tests pass. `f` toggles a flag saved with the position. The stepping keys are untouched.
- [ ] `docs/terminal.md` describes `f`, the folded page, and the choice in the run folder, in the same commit as that behaviour. The README names `f` where it already tells the learner the explanations fold away. `play --help` names `f` and stays within the line limit.

## Phase 3: Verification

- [ ] `nix flake check`
- [ ] `nix build .#default`, then in a detached tmux session `tutorial-stepper play` on a page with an explanation and commands in two windows, driven with `tmux send-keys`. The capture shows the fold, a step, and the explanations back.
- [ ] The documentation presentation check on `README.md` and `docs/`.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phase 2 leaves the suite green. Phase 3 runs last. No new continuous-integration job. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.
