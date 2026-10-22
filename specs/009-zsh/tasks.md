# Tasks: zsh shells

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/009-zsh/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: The declared shell

Test first. The binary is the subject.

- [x] A failing test that a tutorial with `shell = "zsh"` and `echo hi` exits 0, and that the transcript records the command, the output `hi`, and status 0.
- [x] A failing test that the same page with several lines is one transcript entry under zsh.
- [x] A failing test that `examples/status` with `shell = "zsh"` exits non-zero and names the command.
- [x] A failing test that a tutorial with no `shell` key, and one with `shell = "bash"`, still play in bash.
- [x] A failing test that a missing zsh is named, no transcript is written, and no server stays up.
- [x] `examples/if` is no longer refused for declaring zsh. Without `--run-setup` it is still refused, and the message names `setup.sh`. A command marked `if=` is still refused.
- [x] The smallest change that makes those tests pass. zsh loads the `precmd` hook from the plan. Commands stay one bracketed paste. The transcript output drops the shell's paste and prompt redraw and keeps the command's text.
- [x] `docs/play.md` describes the declared shell and the zsh hook, in the same commit. The README no longer says that unattended play is bash only. `play --help` names the shell the index declares and stays within the line limit.

## Phase 3: Both shells

- [x] zsh is on the test build and in the development shell.
- [x] A failing test, then the code, so that every valid example plays under bash and under zsh with the outcome in the specification. `examples/setup` uses `--run-setup`. `examples/slow` is waited out under both.
- [x] A unit test that a zsh pane log becomes the command's output: `hi`, an empty command, a command of several lines, and a command that prints its own colour codes.

## Phase 4: Verification

- [x] `nix flake check`
- [x] `nix build .#default`, then `tutorial-stepper play --auto` on a zsh example, and on an example that must fail under zsh.
- [x] The documentation presentation check on `README.md` and `docs/`.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phases 2 and 3 each leave the suite green. Phase 4 runs last. No new continuous-integration job. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.
