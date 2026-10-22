# Tasks: Print a tutorial's commands as one script

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/006-script/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: The script command

Test first. The binary is the subject: a test runs `tutorial-stepper script` and fails because the command is not there yet.

- [ ] A failing test that `examples/one-window` prints the shebang, then `# welcome.md, window bob`, then `echo hi`, and exits 0.
- [ ] A failing test that `examples/mode` includes the observation and the command, in that order, and leaves out the `mode=show` block.
- [ ] A failing test that `examples/status` includes the command, and that `examples/if` includes both setting branches in page order.
- [ ] A failing test that a tutorial with two pages prints in index order, that an ordinary code block is left out, and that a fence with no window is commented `window not named`.
- [ ] A failing test that every tutorial under `examples/invalid/` prints empty standard output, the same standard error as `check`, and exits non-zero.
- [ ] A failing test that `bash -n` accepts the script of every sound example, and that `script --help` describes the command on one screen.
- [ ] The smallest command that makes those tests pass. The fence body is copied from the page. `check`'s confirmation and its problem lines stay as they are.
- [ ] `docs/script.md`, and the README sentence that `paolino/runmd` is superseded by `tutorial-stepper script`, in the same commit as the command.

## Phase 3: Verification

- [ ] `nix flake check`
- [ ] `nix build .#default`, then `tutorial-stepper script` on each sound example, piped to `bash -n`.
- [ ] The documentation presentation check on `README.md` and `docs/`.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phase 2 does not reopen the fence spelling or the index. Phase 3 runs last, on the commit that contains the command and `docs/script.md`.
