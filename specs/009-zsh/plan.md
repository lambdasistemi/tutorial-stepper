# Implementation Plan: zsh shells

**Branch**: `feat/9-zsh` | **Date**: 2026-10-22 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/009-zsh/spec.md`

## Summary

`tutorial-stepper play --auto` starts the shell the index declares. An absent `shell`, and `shell = "bash"`, stay one bash per window with the hook already in place. `shell = "zsh"` starts one zsh per window. Its `precmd` appends the exit status of the command that just ended, which is the report `PROMPT_COMMAND` already appends for bash. Commands are still one bracketed paste. The transcript keys stay as they are. The refusal of zsh is removed. `docs/play.md` and the README land with that behaviour. The test build and the development shell gain zsh, and the existing test job plays every valid example under both shells. No new continuous-integration job.

## Technical Context

**Language/Version**: Rust, stable, pinned in `rust-toolchain.toml`
**Primary Dependencies**: the existing crate. No new crate. `tmux` and `bash` stay runtime programs. `zsh` joins them for a tutorial that declares it, and for the test environment.
**Storage**: unchanged. The transcript is the file the author names. Hook files live in the control directory, which is always removed.
**Testing**: cargo-nextest, as a flake check, running real tmux, bash, and zsh. The 120 second command stays in that run, under both shells.
**Target Platform**: Linux x86_64, the same binary `check` already ships
**Project Type**: the existing `play` subcommand, extended
**Performance Goals**: a short command finishes as soon as its own shell reports it. A long command is waited out under either shell.
**Constraints**: attributes stay `lang { key=value }`. The end of a command comes from that shell's prompt hook. The server is not the user's tmux. The release manifest is not edited by hand. No new continuous-integration job. The set-up script stays bash.
**Scale/Scope**: the play command, the play reference page, zsh on the test build and the development shell

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| I. The Stepper Is the Product | zsh is the same play loop with a second hook. No second stepper is introduced. |
| II. The Source Is Plain CommonMark | The shell comes from the index. Nothing about the shell is inferred from a fence. |
| III. Nothing Runs Unseen | The text typed into the shell is the fence body. A status other than the declared one still stops the run. |
| V. Every Tutorial Is a Test | The run still exits non-zero at the first unexpected status and writes a transcript. The examples run under both shells. |
| VI. The Shell Says When a Command Ended | bash reports from `PROMPT_COMMAND`. zsh reports from `precmd`. tmux hosts both. There is no deadline. |
| X. No Code for Any One Tutorial | No example is rewritten for one product. The existing examples are played again with the other shell. |
| XI. The Record Outranks the Code | This specification is committed before the command changes. `docs/play.md` lands in the same commit as the behaviour it describes. |

No departure. Settings and observations stay out of this command by the refusals issue 8 already has.

## Project Structure

### Documentation (this feature)

```text
specs/009-zsh/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
src/tutorial.rs      # the declared shell reaches play; zsh is not a refusal
src/play.rs          # one hook per shell, the launcher, the transcript output
src/lib.rs           # play help names the shell the index declares
nix/crane.nix        # zsh on the test build
flake.nix            # zsh in the development shell
tests/play.rs        # a zsh transcript, a missing zsh, every valid example under both shells
docs/play.md         # which shell starts, and what the zsh hook reports
README.md            # unattended play follows the index shell
```

The index parser, the fence parser, the printed script, and the release manifest stay as they are.

## The shells

The server stays `tmux -f /dev/null -L tutorial-stepper-<id>`. Each window is still `new-session` or `new-window` with that window's launcher.

Bash is unchanged: the launcher runs `bash --noprofile --rcfile <hook>`. The hook sets `PROMPT_COMMAND` to append `$?`, sets the prompt to the per-run marker, turns off echo, and turns on bracketed paste.

zsh is a second hook file, `.zshrc`, and the launcher sets `ZDOTDIR` to the directory that holds it and runs `zsh --no-globalrcs`. No `-i`: the pane is a terminal, so the shell is interactive and reads `.zshrc`. `--no-globalrcs` skips the system startup files. The hook is:

```zsh
precmd() {
  local exit_status=$?
  print -r -- "$exit_status" >> "$TUTORIAL_STEPPER_STATUS"
}
PS1=$'\n'"$TUTORIAL_STEPPER_PROMPT"$'\n'
PS2=
unset RPS1
unsetopt prompt_cr prompt_sp
stty -echo
unset HISTFILE
setopt no_beep
zle_highlight=(none)
```

`precmd` runs before the prompt, including the first prompt, so the ready line is the same protocol as bash. The name `status` is not used: in zsh that parameter is read-only, and assigning it aborts the hook. `exit_status` is captured on the first line of `precmd`, before any other command can replace `$?`.

A command is still one bracketed paste, `\e[200~` then the fence body without its trailing newline, then `\e[201~`, then Enter. zsh's line editor accepts that paste as one command, including a body of several lines. `zle_highlight=(none)` keeps the paste from being redrawn character by character. The shell still writes the command text, then `\e[?2004l`, then the command's output, then `\e[0m\e[27m\e[24m\e[J`, then the prompt marker. The transcript output is the text after the first `\e[?2004l` and the newline that follows it, with a trailing `\e[?2004h` removed and a trailing `\e[0m\e[27m\e[24m\e[J` removed, then `\r\n` folded to `\n`. Bash already begins that text with `\e[?2004l`, so the same rule leaves a bash transcript as it is.

`tmux` is looked up first, then the declared shell, both before the run directory is created. A tutorial that is refused for a set-up script the author did not ask to run, or for `if=`, does not reach that lookup. The set-up script is still `bash <absolute path>` in the run folder. A zsh tutorial that passes `--run-setup` therefore needs bash as well as zsh. A zsh tutorial with no script does not need bash.

A shell that exits first, or that reports a line that is not a status, fails the run and names that shell, bash or zsh.

## Both shells in continuous integration

`pkgs.zsh` is added to the crane `nativeBuildInputs` that the nextest check uses, and to the development shell packages. The development-shell job runs `cargo test` inside that shell, so it needs the same `zsh`. The job names stay Build Gate, Format, Clippy, Tests, Supply chain, API reference, and Development shell.

The test copies each valid example, writes the other shell into that copy's index, and plays both. `examples/if` already declares zsh; its other shell is bash. A copy is a temporary directory, so the examples in the repository stay as they are. `examples/setup` is played with `--run-setup` in both copies. `examples/slow` is waited out in both copies.
