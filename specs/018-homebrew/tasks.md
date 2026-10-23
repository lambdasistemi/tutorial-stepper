# Tasks: macOS binary and Homebrew formula

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/018-homebrew/spec.md`, `plan.md`, and `tasks.md`, and commit them before any code.

## Phase 2: The formula

Test first. The subject is the rendered formula.

- [x] A failing check that a version and a checksum produce a formula which depends on tmux, names `tutorial-stepper-<version>-aarch64-darwin.tar.gz`, and carries that checksum. A version or a checksum of the wrong shape is refused. `ruby -c` accepts the file.
- [x] The smallest change that makes that check pass: the template and the renderer.

## Phase 3: The release checksum

Test first. The subject is `SHA256SUMS` for a directory that holds the two Linux binaries and the Apple Silicon tarball.

- [x] A failing check that the directory's `SHA256SUMS` lists all three names, that `sha256sum -c` succeeds, and that a directory missing the tarball is refused.
- [x] The smallest change that makes that check pass.

## Phase 4: Both shells

Test first. The subject is the script the macOS job will run after install.

- [ ] A failing check that `examples/one-window` plays unattended in bash and, with `shell = "zsh"`, in zsh.
- [ ] The smallest change that makes that check pass.

## Phase 5: Continuous integration and the release

- [ ] A `macos-14` job builds the binary, installs it from a temporary local tap pointed at that build, and plays the conformance example in bash and in zsh. The job does not mint a tap token and does not push.
- [ ] The release job uploads `tutorial-stepper-<version>-aarch64-darwin.tar.gz`, rewrites `SHA256SUMS` so it lists that tarball and the two Linux binaries, and pushes `Formula/tutorial-stepper.rb` with a token minted for `homebrew-tap`.
- [ ] `README.md` tells a Mac user how to install with Homebrew, in the same commit as that behaviour.

## Phase 6: Verification

- [ ] `nix flake check`
- [ ] `nix shell nixpkgs#actionlint -c actionlint`
- [ ] The formula check, the checksum check, and the conformance check, each watched failing before the behaviour existed.
- [ ] `brew style` on the rendered formula, when `brew` is installed on this machine.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phases 2, 3, and 4 each leave the suite green. Phase 5 adds the jobs and the readme. Phase 6 runs last. The job names are Build Gate, Format, Clippy, Tests, Supply chain, API reference, Development shell, Release binaries, Ubuntu binaries, Homebrew formula, macOS formula, and macOS release. macOS release runs only after a release is created, so the pull-request check to require is macOS formula, together with Homebrew formula.
