# Implementation Plan: Readline check and signal cleanup

**Branch**: `fix/47-readline-and-signals` | **Date**: 2026-10-23 | **Spec**: `spec.md`

## Summary

Refuse a bash without readline before the first command (run `bind -v` through that shell; `stdenv`'s bash fails it), and kill the private tmux server on `SIGINT`/`SIGTERM` via one guard thread per process. Document `TUTORIAL_STEPPER_WINDOW` in `docs/play.md`. No new dependency beyond `signal-hook-iterator`; no format, index, or transcript change.

## Technical Context

**Language/Version**: Rust (stable via rust-overlay), Nix flake + crane
**Primary Dependencies**: `signal-hook-iterator 0.1` added (MIT/Apache-2.0; `cargo-deny` must stay green)
**Storage**: N/A (no new files at run time)
**Testing**: `nix flake check` (rustfmt, clippy `-D warnings`, nextest, cargo-deny, rustdoc, conformance + tmux plays); new `tests/play.rs` cases for readline (fake bash), `SIGINT`, `SIGTERM` (via `timeout` on `examples/slow`)
**Target Platform**: Linux x86_64/arm64, macOS Apple Silicon (unchanged; signals are Unix, `unsafe_code` stays forbidden — the crate wraps `sigwait`, no `unsafe` in our code)
**Project Type**: CLI + library (unchanged)
**Performance Goals**: N/A (one `bind -v` spawn per run; guard thread blocks forever)
**Constraints**: message is `bash at <path> lacks readline: the stepper's prompt hook needs bind`; exits 1 before any server/dir/transcript; signals exit 130/143 after killing the server; Conventional Commits, no attribution
**Scale/Scope**: 1 check function + 3 call sites, 1 guard (~30 lines) + 2 arm sites, 1 docs paragraph, 3–4 tests

## Constitution Check

- Principle II (plain CommonMark): no fence or index change. No departure.
- Principle VI (shell says when a command ended): the check keeps the hook honest — a shell that cannot install the hook is refused instead of waited on forever. No departure.
- Principle XI (record outranks code): spec/plan/tasks first, docs in the same commits as the behaviour. No departure.

No complexity-table entry: no principle is departed from.

## Project Structure

```text
specs/047-readline-and-signals/
├── spec.md
├── plan.md
└── tasks.md

src/play.rs             # shell_hook_check + require_shell; signal guard + arm in prepare/start_page
src/tutorial.rs         # unchanged (calls drive, which checks)
src/face.rs             # unchanged (start_kept/open_shells check via play::require_shell)
docs/play.md            # TUTORIAL_STEPPER_WINDOW paragraph
Cargo.toml              # + signal-hook-iterator
tests/play.rs           # readline refusal, SIGINT cleanup, SIGTERM cleanup, window-var proof
examples/slow/          # reused for the signal tests (sleep 120, killed after 3s)
```

**Structure Decision**: the check lives in `play.rs` beside `require_program` so unattended and face share it; the guard lives in `play.rs` beside `Session` so both faces arm it.

## Design

- `shell_hook_check(shell_path, shell) -> Option<String>`: `Bash` runs `shell_path --noprofile --norc -c 'bind -v'`; success passes, any failure returns the message. `Zsh` passes. `require_shell(shell) -> Result<PathBuf, Failure>` wraps `require_program` + the check.
- Call sites: `drive` (after `require_program(shell_program)`, before `ensure_transcript_directory`), `Session::start_kept` (after `require_program`, before `prepare`), `Session::open_shells` (moved before `.env`/dirs).
- Guard: `static GUARD_INFO: OnceLock<Mutex<Option<GuardInfo>>>` + `static GUARD_ONCE: Once`; `ensure_signal_guard()` spawns one thread with `Signals::new([SIGINT, SIGTERM])`; on signal it kills `tmux -L socket kill-server`, removes `control_dir` (and `run_dir` when `!keep_run`), then `exit(128 + sig)`. `arm_signal_guard(info)` stores the current server. Armed in `Session::prepare` and `Session::start_page`.
- Docs: after the `.env` paragraph, one paragraph: every window's shell exports `TUTORIAL_STEPPER_WINDOW` set to that window's name; the prompt shows it; tutorials (the Singular demo's setup) may read it.
- Tests: fake-`bash` refusal (script failing on `bind`, exits at once elsewhere); `timeout -s INT/TERM 3` on `examples/slow` asserting no server; `echo $TUTORIAL_STEPPER_WINDOW` asserting the window name.

## Verification

From `/code/tutorial-stepper-issue-47`:

1. `nix flake check`
2. `nix build .#default`, then with `stdenv`'s bash first on `PATH`: `./result/bin/tutorial-stepper play --auto examples/one-window --transcript /tmp/<id>.toml` — stops at once with the message, no server left.
3. `timeout -s INT 3 ./result/bin/tutorial-stepper play --auto examples/slow --transcript /tmp/<id>.toml`, and the same with `-s TERM` — no `tmux -L tutorial-stepper-…` server left afterwards.
4. `nix shell nixpkgs#python3 -c python3 tools/check_presentation.py --no-speech --front README.md README.md docs`
