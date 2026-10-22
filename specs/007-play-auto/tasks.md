# Tasks: Play a one-window tutorial unattended

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/007-play-auto/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: Unattended play

Test first. The binary is the subject: a test runs `tutorial-stepper play --auto` and fails because the command is not there yet.

- [ ] A failing test that `play --help` describes `--auto`, `--transcript`, and an example, and that the overview names `play`.
- [ ] A failing test that `examples/one-window` exits 0, prints `1 command played`, and writes a transcript entry for `echo hi` with output `hi`, status 0, and a start time no later than the end time.
- [ ] A failing test that `examples/status` exits non-zero, names `echo not yet` and the statuses 0 and 19, writes that entry, and leaves no tmux server of the stepper's.
- [ ] A failing test that a block declaring `status=19` passes when the command exits 19.
- [ ] A failing test that a command continued over two lines is one transcript entry whose output is the continued command's output.
- [ ] A failing test that `examples/mode` plays `echo run` only.
- [ ] A failing test that a command `sleep 120` passes, and that the transcript's times cover at least those 120 seconds.
- [ ] A failing test that a tutorial `check` rejects prints the same problems and creates no transcript.
- [ ] A failing test that zsh, a second window, a set-up script, and a command marked `if=` are each refused with a message that names the reason, and that no server is started.
- [ ] A failing test that the shell's working directory is the window's folder.
- [ ] A failing test that a missing `tmux` or a missing `bash` is named and exits non-zero.
- [ ] The smallest command that makes those tests pass. `check` and `script` stay as they are.
- [ ] `docs/play.md`, and the README sentence that a one-window tutorial plays unattended, in the same commit as the command.
- [ ] The conformance examples `examples/continued`, `examples/declared-status`, and `examples/slow`, played in the test run.

## Phase 3: Verification

- [ ] `nix flake check`
- [ ] `nix build .#default`, then `tutorial-stepper play --auto` on a passing example and on `examples/status`, and `ps` shows no stepper tmux server afterwards.
- [ ] `actionlint`, if a workflow changed.
- [ ] The documentation presentation check on `README.md` and `docs/`.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phase 2 does not reopen the fence spelling or the index. Phase 3 runs last, on the commit that contains the command and `docs/play.md`. If the sandboxed `nix flake check` cannot host a tmux server, the play tests run in a job outside the sandbox and that job's name is part of the completion record.
