#!/usr/bin/env bash
# Build the Apple Silicon binary, install it from a temporary tap, and
# play the conformance example in bash and in zsh.
# Usage: macos-formula.sh
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

if ! command -v brew >/dev/null 2>&1; then
  printf 'brew is required\n' >&2
  exit 1
fi

export HOMEBREW_NO_AUTO_UPDATE=1
brew install bash
export PATH="/opt/homebrew/bin:${PATH}"

dest=$root/darwin-bundle
rm -rf "$dest"
bash "$root/scripts/build-darwin.sh" "$dest"

version=$(
  sed -n 's/^version = "\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)"/\1/p' \
    Cargo.toml | head -n 1
)
tarball=$dest/tutorial-stepper-${version}-aarch64-darwin.tar.gz
bash "$root/scripts/install-formula-locally.sh" "$tarball"
bash "$root/scripts/play-conformance.sh" "$root/examples"
