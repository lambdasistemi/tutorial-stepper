#!/usr/bin/env bash
# Push Formula/tutorial-stepper.rb to lambdasistemi/homebrew-tap.
# Usage: push-homebrew-formula.sh TAG DARWIN_DIR
#
# TAP_TOKEN is a token scoped to homebrew-tap. TAG is the release tag,
# such as v0.2.0. The checksum is of the tarball in DARWIN_DIR.
set -euo pipefail

if [[ $# -ne 2 ]]; then
  printf 'usage: push-homebrew-formula.sh TAG DARWIN_DIR\n' >&2
  exit 2
fi

tag=$1
darwin_dir=$(cd "$2" && pwd)
root=$(cd "$(dirname "$0")/.." && pwd)

if [[ -z ${TAP_TOKEN:-} ]]; then
  printf 'TAP_TOKEN is required\n' >&2
  exit 1
fi

shopt -s nullglob
tarballs=( "$darwin_dir"/tutorial-stepper-*-aarch64-darwin.tar.gz )
shopt -u nullglob
if [[ ${#tarballs[@]} -ne 1 ]]; then
  printf 'expected one Apple Silicon tarball in %s\n' "$darwin_dir" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  sha=$(sha256sum "${tarballs[0]}" | awk '{ print $1 }')
else
  sha=$(shasum -a 256 "${tarballs[0]}" | awk '{ print $1 }')
fi

version=${tag#v}
formula=$(mktemp)
tap=$(mktemp -d)
trap 'rm -rf "$formula" "$tap"' EXIT
bash "$root/scripts/render-homebrew-formula.sh" "$version" "$sha" > "$formula"

export GIT_TERMINAL_PROMPT=0
git clone --depth 1 https://github.com/lambdasistemi/homebrew-tap.git "$tap"
git -C "$tap" config user.name "github-actions"
git -C "$tap" config user.email "actions@github.com"
git -C "$tap" config http.extraheader "AUTHORIZATION: bearer ${TAP_TOKEN}"
mkdir -p "$tap/Formula"
cp "$formula" "$tap/Formula/tutorial-stepper.rb"
git -C "$tap" add Formula/tutorial-stepper.rb
if git -C "$tap" diff --cached --quiet; then
  printf 'formula is unchanged\n'
else
  git -C "$tap" commit -m "update tutorial-stepper to ${tag}"
  git -C "$tap" push origin HEAD
fi
