# Implementation Plan: macOS binary and Homebrew formula

**Branch**: `feat/18-homebrew` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/018-homebrew/spec.md`

## Summary

A release uploads `tutorial-stepper-<version>-aarch64-darwin.tar.gz` and a `SHA256SUMS` that lists that tarball and the two Linux binaries. The tarball holds `bin/tutorial-stepper`. The formula `Formula/tutorial-stepper.rb` in `lambdasistemi/homebrew-tap` is rendered from a template with that version and checksum, depends on tmux, and is pushed with a token minted from the organisation's CI app, scoped to `homebrew-tap`. Pull requests build the binary on a `macos-14` runner, install the formula from a temporary local tap pointed at the fresh tarball, and play `examples/one-window` unattended in bash and in zsh. They do not push the formula.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`. The Apple Silicon build uses that same toolchain.
**Primary Dependencies**: the existing crate. No new crate. Homebrew on the `macos-14` runner. The formula template is Ruby, the language Homebrew already uses.
**Storage**: N/A. The artifacts are files on the release, and the formula is a file in the tap.
**Testing**: a sandboxed check renders the formula and rejects a bad version or checksum. A sandboxed check assembles `SHA256SUMS` over the two Linux names and the Apple Silicon tarball. A sandboxed check plays `examples/one-window` through the same script the macOS job uses, in bash and in zsh. `ruby -c` parses the rendered formula. The install from a temporary tap runs on `macos-14`, because this machine is not a Mac.
**Target Platform**: macOS on Apple Silicon. Built on GitHub's `macos-14` runners.
**Project Type**: the existing binary, packaged for a release and for Homebrew.
**Performance Goals**: `--version`, and one conformance example of a single command, in each shell.
**Constraints**: the asset name and the tap path are fixed by the issue. The formula depends on tmux. The tap token is minted from `vars.CI_APP_ID` and `secrets.CI_APP_PRIVATE_KEY`, with `repositories: homebrew-tap`. No personal token. release-please keeps owning the version. The release manifest is not edited by hand. The new pull-request job is a required check, named in the completion line. Intel Macs, Developer ID signing, and notarisation are out of scope.
**Scale/Scope**: the formula template and its renderer, the release checksum script, the conformance script, the Darwin bundle, the pull-request job, the release job, and the install section of the readme.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| I. The Stepper Is the Product | One binary. The formula installs that binary. It does not grow a second stepper. |
| III. Nothing Runs Unseen | The conformance play types the example's command, which is on the page. The formula installs nothing beyond the binary and tmux, which the formula declares. |
| IV. A Tutorial Brings Its Own Environment | Playing still needs the learner's shell. The formula brings tmux, which the stepper needs at run time and which the tutorial does not bring. |
| V. Every Tutorial Is a Test | The macOS job plays a conformance example unattended. A failure exits non-zero. |
| VI. The Shell Says When a Command Ended | The same play command as on Linux. bash and zsh are both played. |
| XI. The Record Outranks the Code | This specification is committed before the behaviour. The install section lands in the same commit as the jobs it describes. |
| Platform and Distribution | The release ships the Apple Silicon build and the formula in `lambdasistemi/homebrew-tap`. At run time the stepper still needs tmux and bash or zsh, and no network. |
| Development Workflow | The pull-request job is a required check. release-please still cuts the release. The macOS build runs on `macos-14` because the NixOS runners cannot build for macOS, the same reason the Ubuntu job uses a GitHub-hosted runner. |

No departure.

## Project Structure

### Documentation (this feature)

```text
specs/018-homebrew/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
packaging/homebrew/tutorial-stepper.rb.in   # formula template
scripts/render-homebrew-formula.sh          # version and checksum in, formula out
scripts/release-checksums.sh                # SHA256SUMS over Linux binaries and the tarball
scripts/play-conformance.sh                 # one example, bash then zsh
scripts/bundle-darwin.sh                    # tarball; refuses a Nix store library path
scripts/install-formula-locally.sh          # temporary tap, file URL, no push
nix/checks.nix                              # formula, checksums, conformance play
flake.nix                                   # expose the pinned Rust toolchain for the Mac build
.github/workflows/ci.yml                    # Homebrew formula job; macOS formula job
.github/workflows/release.yml               # upload the tarball and push the formula
README.md                                   # how a Mac user installs with Homebrew
```

The crate, the release-please manifest, and the Linux release derivation stay as they are.

## What the release holds

The Linux derivation is unchanged: `tutorial-stepper-<version>-x86_64-linux`, `tutorial-stepper-<version>-aarch64-linux`, and a `SHA256SUMS` that lists those two. The Apple Silicon job builds `tutorial-stepper` with the toolchain the flake already pins, linked by the system linker so the binary does not need a Nix store on the learner's Mac. `scripts/bundle-darwin.sh` copies it to `bin/tutorial-stepper`, vendors any library that is not part of macOS into `libexec/lib`, rewrites those load paths, and refuses a path that still points into the Nix store. The tarball has one top-level directory, because Homebrew enters a single top-level directory before install. Inside it are `bin/tutorial-stepper` and, when a library was vendored, `libexec/lib`. The archive is `tutorial-stepper-<version>-aarch64-darwin.tar.gz`.

The release job on `macos-14` runs only after release-please has created a release and the Linux job has uploaded the two Linux binaries. It downloads those two binaries, adds the tarball, and writes a new `SHA256SUMS` with `scripts/release-checksums.sh`. It uploads the tarball and that checksum file. It renders the formula and pushes `Formula/tutorial-stepper.rb`. The push token is a second mint, scoped only to `homebrew-tap`.

A pull request never takes that branch. Its `macos-14` job runs the same bundle, renders the formula with a `file://` archive of the fresh tarball, taps a temporary local git repository, installs from that tap, and plays the example. It does not mint a tap token and it does not clone `lambdasistemi/homebrew-tap`.

The sandboxed formula check is the test for rendering. It fails while the script still prints the template unchanged. The checksum check is the test for the file a learner downloads. It fails while the Darwin tarball is optional. The conformance check is the test for the script the Mac job runs. It fails while that script plays only one shell. The install from Homebrew is the test this machine cannot run.

## Decisions

| Chosen | Rejected | Why |
| --- | --- | --- |
| A tarball `tutorial-stepper-<version>-aarch64-darwin.tar.gz` holding `bin/tutorial-stepper` | A bare binary as the Homebrew archive | Every formula in the tap downloads a tarball and installs `bin/<name>`. The issue allows a tarball that holds the binary. `SHA256SUMS` lists the tarball. |
| System linker, then vendor any non-macOS library | crane's Darwin package as the published binary | crane's linker records Nix store paths. A person who installed with Homebrew has no Nix store. |
| Temporary tap `local/tap` and a `file://` archive | Cloning `lambdasistemi/homebrew-tap` and rewriting one formula in the clone | A pull request must not touch the real tap. `file://` is how the existing Darwin installer points a formula at a fresh tarball. |
| `macos-14` job on every pull request | A path filter that skips the job | A required check that is skipped blocks the merge. The same choice was made for the Linux binaries. |
| One `SHA256SUMS` rewritten after both builds | A second checksum file for macOS | The release already has one `SHA256SUMS`. The learner checks one file. |
| Formula rendered in the repository, pushed by the release | Editing the tap by hand, or committing a formula with a placeholder checksum | The checksum does not exist until the tarball does. The template is what the tests can render here. |
