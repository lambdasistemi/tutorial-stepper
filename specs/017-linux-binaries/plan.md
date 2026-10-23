# Implementation Plan: Static Linux binaries for x86_64 and arm64

**Branch**: `feat/17-linux-binaries` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/017-linux-binaries/spec.md`

## Summary

A release uploads two statically linked binaries, `tutorial-stepper-<version>-x86_64-linux` and `tutorial-stepper-<version>-aarch64-linux`, and one `SHA256SUMS` that lists both. The version is the crate version. The arm64 binary is built on the x86_64 runner by cross-compiling the pure-Rust crate to `aarch64-unknown-linux-musl`, the same way the x86_64 binary already targets `x86_64-unknown-linux-musl`. A sandboxed check reads the artifact directory and rejects a wrong name, a dynamic link, or a bad checksum. A separate job runs each binary in a clean Ubuntu 24.04 container. Pull requests build both binaries. The release workflow uploads them only after release-please creates a release.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`. That file gains the `aarch64-unknown-linux-musl` target.
**Primary Dependencies**: the existing crate. No new crate. crane and the toolchain's musl targets. No C compiler.
**Storage**: N/A. The artifacts are files in the release derivation.
**Testing**: a `runCommand` flake check over `.#release-artifacts` (`file`, `sha256sum`). The Ubuntu run is a continuous-integration job, because it needs a container daemon, which the sandbox does not have.
**Target Platform**: Linux x86_64 and Linux arm64, statically linked. Built on the x86_64 NixOS runner.
**Project Type**: the existing binary, packaged for a release.
**Performance Goals**: `--version` and `check` on one small example. The arm64 run may be emulated.
**Constraints**: asset names are fixed by the issue. One `SHA256SUMS`. The old name `tutorial-stepper-linux-x86_64` is replaced. release-please keeps owning the version. The release manifest is not edited by hand. Every new job is a required check, named in the completion line. The Ubuntu job may use GitHub's `ubuntu-24.04` runners.
**Scale/Scope**: the two static packages, the release derivation, the sandboxed check, the Ubuntu job, and the install section of the readme.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| I. The Stepper Is the Product | One binary per machine. The release is that binary, not a second implementation. |
| IV. A Tutorial Brings Its Own Environment | The container runs `--version` and `check` only. It does not play a tutorial. Playing stays in the learner's shell. The binary installs nothing. |
| XI. The Record Outranks the Code | This specification is committed before the behaviour. The install section lands in the same commit as the binaries it describes. |
| Platform and Distribution | The release ships one static binary for Linux x86_64 and one for Linux arm64. At run time, playing still needs tmux and bash or zsh, and no network. |
| Development Workflow | The new jobs run in continuous integration and are required checks. release-please still cuts the release. Formatting, lints, and the existing tests stay as they are. |

No departure.

## Project Structure

### Documentation (this feature)

```text
specs/017-linux-binaries/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
rust-toolchain.toml                 # add the arm64 musl target
nix/packages.nix                    # the two static packages
nix/release.nix                     # versioned names and one checksum file
nix/checks.nix                      # names, architectures, static link, checksums
flake.nix                           # pass the version into the release derivation
.github/workflows/ci.yml            # build both on pull requests; Ubuntu job
scripts/ubuntu-binaries.sh          # --version and check in Ubuntu 24.04
README.md                           # how a Linux user installs from the release
.github/workflows/release.yml       # unchanged trigger; the upload already takes every file in the derivation
```

The crate, the release-please manifest, and the command-line behaviour stay as they are.

## What the release holds

The x86_64 static package is the one the release already builds, retargeted only in its file name. The arm64 static package is the same crate with `CARGO_BUILD_TARGET=aarch64-unknown-linux-musl` and `target-feature=+crt-static`. The host `cc` links the x86_64 objects and rejects the arm64 ones, so the arm64 link uses the toolchain's `rust-lld`. crane keeps a separate dependency build for that target, so the host build's cache stays valid. The commit is passed only to the final package build.

`nix/release.nix` copies the two binaries to the names in the specification and writes `SHA256SUMS` with `sha256sum`. The release job already uploads every file in that directory, and only when release-please set `release_created`. A pull request never takes that branch. The build gate already builds `.#release-artifacts` on every pull request, so both binaries are built there once the derivation contains both.

The sandboxed check is the test for the directory. It fails while the directory still has the old name. The Ubuntu script is the test for running. It mounts one example, runs `--version`, and runs `check`. The x86_64 binary runs in `ubuntu:24.04` for that architecture. The arm64 binary runs under `qemu-user-static` from that same Ubuntu release, inside the amd64 image.

## Decisions

| Chosen | Rejected | Why |
| --- | --- | --- |
| musl targets from the pinned toolchain | a Nix cross stdenv and a C toolchain | The crate is pure Rust. The x86_64 release already links this way. |
| Ubuntu job on GitHub's `ubuntu-24.04` runners | running the container inside the Nix sandbox | The sandbox has no container daemon. The organisation's NixOS runners build. This one job only runs the built files. |
| Ubuntu 24.04's `qemu-user-static` inside the amd64 image | an arm64 container via binfmt, or a position-independent arm64 link | That emulator runs the fixed-address static binary and passes its arguments. A position-independent link segfaults under it. |
| Build both binaries on every pull request | a path filter that skips the job | A required check that is skipped blocks the merge. Building both is the same cost as today's single static build plus one cross build, and the gate already builds the release derivation. |
