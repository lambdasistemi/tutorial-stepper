# Tasks: Readline check and signal cleanup

**Input**: `spec.md` (US1 readline + US2 signals, P1), `plan.md`
**Prerequisites**: none

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Spec (record first)

- [ ] T001 [US1] Write `specs/047-readline-and-signals/{spec,plan,tasks}.md`; commit on `fix/47-readline-and-signals` (no code yet)

## Phase 2: Failing tests first (TDD)

- [ ] T002 [US1] RED: fake-`bash` refusal test in `tests/play.rs` (script failing on `bind`); watch it fail without the readline message
- [ ] T003 [US2] RED: `SIGINT` + `SIGTERM` tests in `tests/play.rs` (`timeout -s INT/TERM 3` on `examples/slow`, asserting no server); watch them fail with a left server

## Phase 3: Implementation + docs

- [ ] T004 [US1] GREEN: `shell_hook_check` + `require_shell` in `src/play.rs`, wired into `drive`, `start_kept`, `open_shells`; `T002` passes
- [ ] T005 [US2] GREEN: signal guard in `src/play.rs` (`signal-hook-iterator`, armed in `prepare`/`start_page`); `T003` passes
- [ ] T006 [US1] `TUTORIAL_STEPPER_WINDOW` paragraph in `docs/play.md` + window-var proof test; in the same commit as the behaviour it describes

## Phase 4: Green + verification

- [ ] T007 Gate: `nix flake check` passes; `nix build` + stdenv-bash refusal + `timeout INT/TERM` on `slow` leave no server; presentation check passes
- [ ] T008 Review the full diff, commit bisect-safe steps, journal `COMPLETE done head=<sha>`, wait on channels

## Dependencies & Execution Order

- T001 before everything (record first); committed alone
- T002 before T004 (RED watched); T003 before T005 (RED watched: left server)
- T004 before T005 (check first, then guard); T006 with T004–T005 (docs in behaviour commits)
- T007 after T004–T006; T008 last
