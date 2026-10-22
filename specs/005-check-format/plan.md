# Implementation Plan: Check a tutorial's format

**Branch**: `feat/5-check` | **Date**: 2026-10-22 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/005-check-format/spec.md`

## Summary

Ship the repository's first binary. `tutorial-stepper check DIR` reads a tutorial and reports format problems as `FILE:LINE: message`, exiting 0 only when the tutorial matches version 0. The binary is a Rust crate built by a Nix flake. Continuous integration on the NixOS runners replaces the placeholder workflow: a build gate, then rustfmt, clippy with warnings as errors, and the tests. release-please opens release pull requests with a token minted from the organisation app, and a release attaches the Linux x86_64 binary and its checksum. `docs/format.md` is version 0 of the format and lands in the same change as the check that enforces it.

The index file's name and syntax are proposed in the specification and are not implemented until the operator chooses.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml` and consumed by rust-overlay
**Primary Dependencies**: clap for the command line; pulldown-cmark to read CommonMark fences without inventing a Markdown dialect; thiserror for the check's errors. No network crate.
**Storage**: files only
**Testing**: cargo-nextest, as a flake check. The tutorials under `examples/` and `examples/invalid/` are tests too, run through the binary by `cargo test`.
**Target Platform**: Linux x86_64 for the release binary. The flake builds on the host Nix supports.
**Project Type**: one command-line binary, library and binary in a single crate
**Performance Goals**: a check of a small tutorial finishes immediately. No throughput target.
**Constraints**: attributes come only from `lang { key=value }` in the fence info string. The command runs nothing. The release pull request is opened with the organisation app token, not the default workflow token.
**Scale/Scope**: one command (`check`), six invalid examples, a handful of valid ones, version 0 of the format document.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| II. The Source Is Plain CommonMark | Attributes live in the fence info string as `key=value`. The other spellings are errors. Pages stay ordinary CommonMark. The format, the index, and every attribute are written in `docs/format.md` and checked by the example tutorials. |
| X. No Code for Any One Tutorial | The examples are generic. Window names and settings come from the tutorial's index, not from the binary. |
| XI. The Record Outranks the Code | This specification is committed before the parser. `docs/format.md` and the README status land with the behaviour they describe. |
| Platform and Distribution | Rust, Nix flake, release-please. This issue attaches the Linux x86_64 binary only; the other platforms are a later issue, as the issue's non-goals state. |
| Development Workflow | Spec Kit acceptance before code. Continuous integration on NixOS runners, build gate first. Conventional Commits. Formatting and clippy are checks. |

No departure. The index file is not a departure either: the constitution leaves its name and syntax open, and the specification asks.

## Project Structure

### Documentation (this feature)

```text
specs/005-check-format/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml
Cargo.lock
rust-toolchain.toml
rustfmt.toml
deny.toml
justfile
flake.nix
flake.lock
nix/
├── toolchain.nix
├── crane.nix
├── packages.nix
├── checks.nix
└── release.nix
src/
├── main.rs          # command line: version, help, check
├── lib.rs
└── fence.rs         # info-string attributes and their line
examples/
└── <one directory per valid tutorial>
examples/invalid/
└── <one directory per error kind>
docs/format.md
.github/workflows/ci.yml
.github/workflows/release.yml
release-please-config.json
.release-please-manifest.json
```

**Structure Decision**: One crate at the repository root, so release-please's Rust strategy can bump its version. A virtual workspace of one crate would hide that version. The Nix flake follows the crane layout: thin `flake.nix`, toolchain from `rust-toolchain.toml`, checks that are real sandboxed derivations (`cargoClippy`, `cargoFmt`, `cargoNextest`, `cargoDeny`, `cargoDoc`).

## Design notes

**Fence attributes, independent of the index.** A fence is located with pulldown-cmark so the line numbers are CommonMark's, not a private scanner's. The info string is accepted only as an optional language word plus an optional `{ key=value … }` group. JSON, bare `key=value` outside braces, and a leading `.lang` inside the braces are errors at that line. Unknown keys are errors. This parser and its tests do not read the index, so they can land before the operator answers.

**The index parser waits.** Pages, declared windows, and declared settings are checked only after the operator picks the file name and the syntax. Until then, `check` is not claimed to meet the acceptance scenarios that mention the index.

**What `check` prints.** Success prints one line with the page count and the command count. Failure prints every problem as `FILE:LINE: message` and exits 1. A usage mistake exits 2 and names `check --help`. The file path is the directory argument joined with the page's path inside it.

**Identity.** The packaged binary prints `tutorial-stepper <version> (commit <commit>)`. The version is the crate version. The commit is the flake's source revision, or `unknown` on a bare developer build.

**Continuous integration.** Job names, stable, in this order of dependence:

| Job | What it runs |
| --- | --- |
| Build Gate | `nix build` of the binary, every flake check, the release-artifacts derivation, and the development shell |
| Format | the rustfmt flake check |
| Clippy | the clippy flake check, warnings denied |
| Tests | the nextest flake check |
| Supply chain | cargo-deny |
| API reference | rustdoc with warnings denied |
| Development shell | `nix develop -c cargo test --locked`, because a packaged check never enters the shell |

Every job after Build Gate depends on it. Runners are `nixos`. Cachix is `paolino` and is read-only when its token is absent.

**Release.** `release-please-action` with `release-type: rust`, manifest at the repository root. The workflow mints a token from `vars.CI_APP_ID` and `secrets.CI_APP_PRIVATE_KEY` and passes it as the release-please token. When a release is created, a NixOS job builds `.#release-artifacts` and uploads the Linux x86_64 binary and `SHA256SUMS`.

## Complexity Tracking

No constitution violation.
