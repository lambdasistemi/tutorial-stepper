# Tasks: Several windows, the set-up script and the needs check

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/008-windows/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: Several windows

Test first. The binary is the subject.

- [x] A failing test that `examples/two-windows` exits 0, that each command's transcript entry names its window, and that the working directory ends with that window's folder, including a command that names no window after one that named `bob`.
- [x] A failing test that a directory change in one window leaves the other window in its own folder.
- [x] `examples/one-window` and the issue 7 refusals for zsh and for `if=` still pass. The refusal of a second window is removed.
- [x] The smallest change that makes those tests pass. Each declared window is one bash shell.
- [x] `docs/play.md` describes several windows, in the same commit as the behaviour. The README no longer says that unattended play is one window only.

## Phase 3: The run folder

- [x] A failing test that `--run DIR` on an empty directory keeps `DIR` and the window folder after the run, and that the transcript's working directory is inside `DIR`.
- [x] A failing test that a run without `--run` removes the working directory the transcript recorded.
- [x] A failing test that a non-empty `--run` directory is refused, unchanged, with no transcript and no server.
- [x] The smallest change that makes those tests pass. `play --help` describes `--run` and stays within the line limit.

## Phase 4: The set-up script

- [x] A failing test that without `--run-setup` the run stops, names the script, writes no transcript, and does not run the script.
- [x] A failing test that `--run-setup` runs the script in the run folder before any command, that a command can read a file the script wrote, and that the script's user id is the id of the user who started `play`.
- [x] A failing test that a script that exits non-zero stops the run, names the script and the status, and runs no page command.
- [x] `examples/setup` is that tutorial. `docs/play.md` describes `--run-setup` in the same commit.

## Phase 5: The needs check

- [ ] A failing test that `examples/needs-missing` stops before the first command, names `tutorial-stepper-missing-program`, and says it was not found.
- [ ] A failing test that `bash >= 99.0.0` stops with the dotted number `bash --version` printed, when that number is older.
- [ ] A failing test that a program that prints no dotted number, and declares a minimum, stops and says no version was found.
- [ ] A failing test that a program on `PATH` with no minimum does not need `--version` to succeed.
- [ ] `docs/play.md` and `docs/format.md` say how the dotted number is read and what happens when none is printed, in the same commit.

## Phase 6: Verification

- [ ] `nix flake check`
- [ ] `nix build .#default`, then play `examples/setup` with `--run` and `--run-setup` (two windows, the script first, the run folder kept), the same tutorial without `--run-setup`, `examples/needs-missing`, and a tutorial that needs a newer `bash` than the one installed.
- [ ] The documentation presentation check on `README.md` and `docs/`.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phases 2 to 5 each leave the suite green. Phase 6 runs last. No new continuous-integration job. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.
