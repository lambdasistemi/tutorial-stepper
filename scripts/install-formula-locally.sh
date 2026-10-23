#!/usr/bin/env bash
# Install the formula from a temporary tap pointed at one fresh tarball.
# Usage: install-formula-locally.sh TARBALL
#
# The tap is a local git repository named local/tap. The archive URL is
# file:// of the tarball. Nothing is cloned from lambdasistemi/homebrew-tap
# and nothing is pushed.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  printf 'usage: install-formula-locally.sh TARBALL\n' >&2
  exit 2
fi

if ! command -v brew >/dev/null 2>&1; then
  printf 'brew is required\n' >&2
  exit 1
fi

tarball=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
base=$(basename "$tarball")
case "$base" in
  tutorial-stepper-*-aarch64-darwin.tar.gz) ;;
  *)
    printf 'tarball name must be tutorial-stepper-<version>-aarch64-darwin.tar.gz\n' >&2
    exit 1
    ;;
esac
rest=${base#tutorial-stepper-}
version=${rest%-aarch64-darwin.tar.gz}

if command -v sha256sum >/dev/null 2>&1; then
  sha=$(sha256sum "$tarball" | awk '{ print $1 }')
else
  sha=$(shasum -a 256 "$tarball" | awk '{ print $1 }')
fi

root=$(cd "$(dirname "$0")/.." && pwd)
formula=$(mktemp)
tap=$(mktemp -d)
trap 'rm -rf "$formula" "$tap"' EXIT
bash "$root/scripts/render-homebrew-formula.sh" \
  "$version" "$sha" "file://${tarball}" > "$formula"

mkdir -p "$tap/Formula"
cp "$formula" "$tap/Formula/tutorial-stepper.rb"
git -C "$tap" init
git -C "$tap" add Formula/tutorial-stepper.rb
git -C "$tap" -c user.email=formula@localhost -c user.name=formula \
  commit -m "formula"

export HOMEBREW_NO_AUTO_UPDATE=1
export HOMEBREW_NO_REQUIRE_TAP_TRUST=1
export HOMEBREW_NO_INSTALL_FROM_API=1

if brew tap | grep -qx 'local/tap'; then
  brew untap local/tap
fi
if brew list --formula tutorial-stepper >/dev/null 2>&1; then
  brew uninstall --force tutorial-stepper
fi
brew tap local/tap "$tap"
brew trust local/tap >/dev/null 2>&1 || true
brew install local/tap/tutorial-stepper
brew test local/tap/tutorial-stepper
