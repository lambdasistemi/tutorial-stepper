# Tasks: Observations in the unattended run

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/011-observe/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: Observations in the unattended run

Test first. The binary, a real shell, and the transcript are the subject.

- [ ] A failing test that `examples/mode` plays `echo observed` where it appears, with `position` set to `start of path`, and again with `position` set to `end of path`, and that the shown block stays out of the transcript.
- [ ] A failing test that `examples/observe` plays the observation between its two commands, records `look.md:1`, plays it again at `end of path`, and prints `seen` both times.
- [ ] The same tutorials under zsh, through the both-shells sweep.
- [ ] A failing test that `examples/observe-gone` exits 1, names the observation, says the failure was at the end of the path, and still records the successful run where the observation appears.
- [ ] A failing test that an observation which fails where it appears does not run the later command and does not run again at the end.
- [ ] A failing test that an observation with no window uses the inherited window and does not change it, and that an observation which names another window runs there.
- [ ] A failing test that an observation marked `if=` runs, in place and at the end, only on the path it belongs to.
- [ ] The smallest change that makes those tests pass. `N commands played` still counts commands. `check` and `script` stay as they are.
- [ ] `docs/play.md`, `docs/format.md`, and the README describe the two runs in the same commit. `play --help` names them and stays within the line limit.

## Phase 3: Verification

- [ ] `nix flake check`
- [ ] `nix build .#default`, then `play --auto examples/observe`, and `play --auto examples/observe-gone`.
- [ ] The documentation presentation check on `README.md` and `docs/`.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phase 2 leaves the suite green. Phase 3 runs last. No new continuous-integration job. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.
