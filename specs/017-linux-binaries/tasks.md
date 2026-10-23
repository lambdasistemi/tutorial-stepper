# Tasks: Static Linux binaries for x86_64 and arm64

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/017-linux-binaries/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: The two binaries and the checksum

Test first. The subject is the release artifact directory.

- [x] A failing check that the directory holds `tutorial-stepper-<version>-x86_64-linux` and `tutorial-stepper-<version>-aarch64-linux`, that `SHA256SUMS` matches them, that `file` reports each statically linked for its architecture, and that `tutorial-stepper-linux-x86_64` is absent.
- [x] The smallest change that makes that check pass: the arm64 musl target, the two static packages, and the versioned names.
- [x] `README.md` tells a Linux user without Nix how to download the binary, check it, and run `--version`, in the same commit as that behaviour.

## Phase 3: Clean Ubuntu

Test first. The subject is each binary. This needs a container daemon, so it is a continuous-integration job, not a sandboxed check.

- [x] A failing run of each binary in a clean Ubuntu 24.04 container: `--version` prints the release, and `check` on an example exits 0. The arm64 binary runs under emulation.
- [x] The job that runs that test on pull requests, without uploading anything. The build of both binaries stays on the NixOS build gate.

## Phase 4: Verification

- [ ] `nix flake check`
- [ ] `nix build .#release-artifacts`, then `sha256sum -c`, and `file` on both binaries.
- [ ] Each binary prints `--version` and checks an example in a clean Ubuntu 24.04 container.
- [ ] `nix shell nixpkgs#actionlint -c actionlint`

## Dependencies

Phase 2 starts only after phase 1 is committed. Phase 2 leaves the suite green. Phase 3 adds the Ubuntu job. Phase 4 runs last. The job names are Build Gate, Format, Clippy, Tests, Supply chain, API reference, Development shell, Release binaries, and Ubuntu binaries.
