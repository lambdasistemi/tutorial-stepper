#!/usr/bin/env bash
# Build the Apple Silicon binary and pack the Homebrew tarball.
# Usage: build-darwin.sh DESTDIR
#
# The compiler is the Rust toolchain the flake pins. The linker is the
# system linker, so the binary does not need a Nix store. The tarball
# lands in DESTDIR.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  printf 'usage: build-darwin.sh DESTDIR\n' >&2
  exit 2
fi

dest=$1
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

if ! command -v nix >/dev/null 2>&1; then
  printf 'nix is required\n' >&2
  exit 1
fi

toolchain=$(nix build --quiet --no-link --print-out-paths .#rust-toolchain)
version=$(
  sed -n 's/^version = "\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)"/\1/p' \
    Cargo.toml | head -n 1
)
if [[ -z $version ]]; then
  printf 'cannot read the version from Cargo.toml\n' >&2
  exit 1
fi
commit=$(git rev-parse --verify HEAD)

export PATH="${toolchain}/bin:/usr/bin:/bin:/opt/homebrew/bin:${PATH:-}"
unset NIX_CFLAGS_COMPILE NIX_LDFLAGS NIX_CC NIX_BINTOOLS CARGO_TARGET_DIR
case ${CC:-} in /nix/store/*) unset CC ;; esac
case ${CXX:-} in /nix/store/*) unset CXX ;; esac
case ${RUSTFLAGS:-} in */nix/store/*) unset RUSTFLAGS ;; esac
export TUTORIAL_STEPPER_VERSION=$version
export TUTORIAL_STEPPER_COMMIT=$commit

cargo build --release --locked
bash "$root/scripts/bundle-darwin.sh" \
  "$root/target/release/tutorial-stepper" \
  "$version" \
  "$dest"
