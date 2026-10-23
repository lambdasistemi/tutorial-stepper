#!/usr/bin/env bash
# Play examples/one-window unattended, in bash and in zsh.
# Usage: play-conformance.sh EXAMPLES
#
# tutorial-stepper, tmux, zsh, and bash 4 or newer must be on PATH.
# The bash tutorial is the example as published. The zsh tutorial is a
# copy with shell = "zsh" added. Each transcript must record echo hi
# with status 0.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  printf 'usage: play-conformance.sh EXAMPLES\n' >&2
  exit 2
fi

examples=$(cd "$1" && pwd)
source=$examples/one-window

if [[ ! -f $source/tutorial.toml || ! -f $source/welcome.md ]]; then
  printf 'no tutorial at %s\n' "$source" >&2
  exit 1
fi

if ! command -v tutorial-stepper >/dev/null 2>&1; then
  printf 'tutorial-stepper is not on PATH\n' >&2
  exit 1
fi

if ! command -v tmux >/dev/null 2>&1; then
  printf 'tmux is not on PATH\n' >&2
  exit 1
fi

if ! command -v zsh >/dev/null 2>&1; then
  printf 'zsh is not on PATH\n' >&2
  exit 1
fi

bash_major=$(bash -c 'printf %s "${BASH_VERSINFO[0]}"')
if [[ $bash_major -lt 4 ]]; then
  printf 'bash 4 or newer is required; found bash %s\n' "$bash_major" >&2
  exit 1
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

play_one() {
  local tutorial=$1
  local shell_name=$2
  local transcript=$work/$shell_name.toml
  tutorial-stepper play --auto "$tutorial" --transcript "$transcript"
  if ! grep -q 'echo hi' "$transcript"; then
    printf '%s transcript has no echo hi\n' "$shell_name" >&2
    cat "$transcript" >&2
    exit 1
  fi
  if ! grep -q 'status = 0' "$transcript"; then
    printf '%s transcript has no zero status\n' "$shell_name" >&2
    cat "$transcript" >&2
    exit 1
  fi
  printf 'played %s\n' "$shell_name"
}

play_one "$source" bash

zsh_tutorial=$work/zsh-tutorial
mkdir -p "$zsh_tutorial"
cp "$source/welcome.md" "$zsh_tutorial/welcome.md"
{
  printf 'shell = "zsh"\n'
  cat "$source/tutorial.toml"
} > "$zsh_tutorial/tutorial.toml"

play_one "$zsh_tutorial" zsh
