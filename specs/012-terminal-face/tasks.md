# Tasks: Play a tutorial page by page in the terminal

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/012-terminal-face/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: The terminal face

Test first. A detached tmux session and the face are the subject. `--auto` stays green throughout.

- [ ] A failing test that the rendered page shows a heading, bold, inline code, a code block, a link and a list, wrapped to a given width, and marks the command whose fence is named.
- [ ] A failing test that `play DIR` in a detached tmux session shows that page beside the window of the next command, and that Enter runs the command there.
- [ ] A failing test that the page moves beside the other window of `examples/two-windows` when that window's command is next.
- [ ] A failing test that `s` skips, `c` interrupts a running command, `b` returns to the previous command and leaves its file, and `n` and `p` turn the page without moving the next command.
- [ ] A failing test that a status other than the declared one is shown, and that Enter retries, `s` skips and `q` quits.
- [ ] A failing test that `q` closes the page and keeps the windows, and that a second `play` of the same run marks the same command. The default folder is under the home directory and is named after the tutorial. `--run DIR` overrides it.
- [ ] The smallest change that makes those tests pass. Sending a command and waiting for its end are the functions `--auto` already uses. `--auto` keeps its transcript, its flags and its server lifetime.
- [ ] `docs/terminal.md` describes the keys, the default run folder and `--run`, in the same commit. `docs/play.md` and the README point at that page. `play --help` names the face first and stays within the line limit. The overview stays one screen.

## Phase 3: Verification

- [ ] `nix flake check`
- [ ] `nix build .#default`, then in a detached tmux session `tutorial-stepper play examples/two-windows`, driven with `tmux send-keys`. The capture shows the page beside the right window, a command run by Enter, and the position kept after `q` and a second `play`.
- [ ] The documentation presentation check on `README.md` and `docs/`.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phase 2 leaves the suite green. Phase 3 runs last. No new continuous-integration job. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.
