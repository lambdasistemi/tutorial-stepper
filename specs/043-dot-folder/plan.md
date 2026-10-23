# Implementation Plan: Dot folder

**Branch**: `feat/43-dot-folder` | **Date**: 2026-10-23 | **Spec**: `specs/043-dot-folder/spec.md`

## Summary

Accept `folder = "."` as a window folder meaning the run folder itself, in `check`, unattended `play` and the terminal face, with docs and a new example plus tests. Nothing else changes.

## Technical Context

**Language/Version**: Rust (stable via rust-overlay), Nix flake + crane
**Primary Dependencies**: none added
**Storage**: N/A (example dir + specs)
**Testing**: `nix flake check` (rustfmt, clippy `-D warnings`, nextest, cargo-deny, rustdoc, conformance + end-to-end tmux plays); new `examples/dot-folder/` + `tests/examples.rs` + `tests/play.rs` + unit tests for the predicate
**Target Platform**: Linux x86_64/arm64, macOS Apple Silicon (unchanged)
**Project Type**: CLI + library (unchanged)
**Performance Goals**: N/A (parse-time predicate)
**Constraints**: `.` only for window `folder`; `setup`/`pages` keep the strict rule; empty/absolute/`..` still refused with the same message; Conventional Commits, no attribution
**Scale/Scope**: 1 predicate + 1 call site, docs paragraph, 1 example (2 files), test additions

## Constitution Check

- Principle II (plain CommonMark): no syntax invented; `"."` is a TOML value, not a fence change. No departure.
- Principle IV (environment): folders still declare where windows run; `.` names the run folder explicitly. No departure.
- Principle XI (record outranks code): spec/plan/tasks first, docs in the same commits. No departure.

No complexity-table entry: no principle is departed from.

## Project Structure

```text
specs/043-dot-folder/
├── spec.md
├── plan.md
└── tasks.md

src/index.rs            # is_window_folder ("." or strict relative); folder call site uses it
docs/format.md          # "." means the run folder itself
examples/dot-folder/    # tutorial.toml (top="." + sub="sub"), welcome.md (pwd in each)
tests/examples.rs       # dot-folder in the valid table
tests/play.rs           # dot window runs in the run folder, sub window in its subfolder
```

**Structure Decision**: predicate scoped to window folders; `is_relative_path` untouched for `setup`/`pages`.

## Design

- `is_window_folder(value) = value == "." || is_relative_path(value)`. Folder call site uses it; error message unchanged (`folder must be a relative path`).
- `play` needs no logic change: `run_dir.join(".")` is the run folder; `create_dir_all` on it is a no-op. The terminal face shares the same `Loaded` windows, so it follows.
- Docs: folder sentence gains "`.` is the run folder itself".
- Example `dot-folder`: windows `top` (`"."`) and `sub` (`"sub"`); page runs `pwd` in each. `check` prints `1 page, 2 commands`.
- Tests: valid-table entry; play test asserts `top` output equals the run folder and `sub` output ends with `/sub`; unit tests for the predicate (accept `.`, `alice`, `a/b`; refuse `""`, `/bob`, `..`, `a/../b`, `a/..`).

## Verification

From `/code/tutorial-stepper-issue-43`:

1. `nix flake check`
2. `nix build .#tutorial-stepper --out-link result` then `./result/bin/tutorial-stepper check examples/dot-folder` and `play --auto` on it with `--run` + `--transcript`
