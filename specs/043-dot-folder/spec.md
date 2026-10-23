# Feature Specification: A window's folder may be the run folder itself

**Feature Branch**: `feat/43-dot-folder`
**Created**: 2026-10-23
**Status**: Specified
**Input**: Issue 43, "A window's folder may be the run folder itself" (tutorial-stepper milestone M1; needed by singular-demo#23, whose chain window runs at the top of the run)

## User Scenarios & Testing

### User Story 1 — An author declares a window in the run folder (Priority: P1)

As a tutorial author, I declare a window with `folder = "."` in `tutorial.toml`, and observe `check` accept it and `play` run that window's commands in the run folder itself.

**Why this priority**: This is the whole issue. The Singular demo's chain window runs at the top of the run (where `devnet.out` and `registry.address` live) while Alice and Bob run in folders of their own.

**Independent Test**: A new valid example with a `folder = "."` window passes `check` and plays unattended with the transcript proving the dot window ran in the run folder and the other window in its subfolder.

**Acceptance Scenarios**:

1. **Given** `folder = "."`, **When** the author runs `check`, **Then** it passes and counts the commands.
2. **Given** an empty folder, an absolute path, or any path containing `..`, **When** the author runs `check`, **Then** it fails naming the relative-path problem.
3. **Given** a `folder = "."` window, **When** the author plays unattended (and in the terminal face), **Then** that window starts in the run folder itself.

## Requirements

### Functional Requirements

- **FR-001**: `check` accepts `folder = "."` as the run folder itself.
- **FR-002**: `check` still refuses an empty folder, an absolute path, and any path containing `..` (unchanged error).
- **FR-003**: `play` (unattended `drive` and the terminal face) runs a `folder = "."` window in the run folder itself.
- **FR-004**: `docs/format.md` says what `.` means.
- **FR-005**: A valid example and tests cover it (new `examples/dot-folder/`, `check` + `play` assertions, unit coverage for the folder predicate).

### Key Entities

- **Dot folder**: `folder = "."`, the run folder itself; every other folder is a relative path of `Normal` components only.

## Success Criteria

- **SC-001**: `nix flake check` passes.
- **SC-002**: `./result/bin/tutorial-stepper check` and `play --auto` pass on the new dot-folder example.

## Assumptions

- Folders outside the run folder stay out of scope (non-goal).
- `setup` and `pages` keep the strict relative-path rule; only window `folder` gains `"."`.
