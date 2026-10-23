# Tasks: Push a command to edit it before it runs

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/013-push/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: Place the command, then record the line that ran

Test first. A detached tmux session and the face are the subject. `--auto` stays green, under bash and under zsh.

- [x] A failing test that `e` places a command of several lines at the prompt, in bash and in zsh, and that the command's output does not appear before Enter.
- [x] A failing test that editing the pushed command and pressing Enter leaves the edited line on the page, in bash and in zsh.
- [x] The smallest change that makes those tests pass. The prompt hook reports the accepted line. The transcript's `command`, the page, and the status error all use that line. A command nobody edited stays the fence body.
- [x] `docs/terminal.md` describes `e` and the line the page shows, in the same commit as that behaviour. `docs/play.md` says the transcript command is the line the hook reported. `play --help` names `e` and stays within the line limit.

## Phase 3: Verification

- [ ] `nix flake check`
- [ ] `nix build .#default`, then in a detached tmux session `tutorial-stepper play` on a tutorial with a command of several lines, driven with `tmux send-keys`. The capture shows the paste waiting, the edit, and the recorded line.
- [ ] The documentation presentation check on `README.md` and `docs/`.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phase 2 leaves the suite green. Phase 3 runs last. No new continuous-integration job. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.
