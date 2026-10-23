# Tasks: Dot folder

**Input**: `specs/043-dot-folder/spec.md` (P1 story + acceptance), `plan.md`
**Prerequisites**: none

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Spec (record first)

- [ ] T001 [US1] Write `specs/043-dot-folder/{spec,plan,tasks}.md`; commit on `feat/43-dot-folder` (no code yet)

## Phase 2: Failing test first (TDD)

- [ ] T002 [US1] RED: add `examples/dot-folder/` (top `"."`, sub `"sub"`, `pwd` in each) and list it in `tests/examples.rs`; run the check test and watch it fail on `folder must be a relative path`

## Phase 3: Implementation + docs

- [ ] T003 [US1] GREEN: `is_window_folder` in `src/index.rs` (accept `"."`), folder call site uses it; `docs/format.md` says `.` is the run folder itself
- [ ] T004 [US1] Play coverage: `tests/play.rs` asserts the dot window runs in the run folder; unit tests for the predicate (accept/refuse sets)

## Phase 4: Green + verification

- [ ] T005 [US1] GREEN: `nix flake check` passes; `nix build` + `check`/`play --auto` pass on `examples/dot-folder`
- [ ] T006 [US1] Review the full diff, commit bisect-safe steps, journal `COMPLETE stepper-43 head=<sha>`, wait on channels

## Dependencies & Execution Order

- T001 before everything (record first); committed alone
- T002 before T003 (test first, RED watched)
- T003 before T004; T005 after T003–T004; T006 last
