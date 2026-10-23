#!/usr/bin/env bash
# Upload the Apple Silicon tarball and a checksum file that also lists
# the two Linux binaries.
# Usage: publish-release-assets.sh TAG DARWIN_DIR
#
# GH_TOKEN must be able to upload to this repository's release. The Linux
# binaries are already on that release. TAG is the release tag, such as
# v0.2.0, and it must match the version in Cargo.toml.
set -euo pipefail

if [[ $# -ne 2 ]]; then
  printf 'usage: publish-release-assets.sh TAG DARWIN_DIR\n' >&2
  exit 2
fi

tag=$1
darwin_dir=$(cd "$2" && pwd)
root=$(cd "$(dirname "$0")/.." && pwd)

if [[ -z ${GH_TOKEN:-} ]]; then
  printf 'GH_TOKEN is required\n' >&2
  exit 1
fi

manifest=$(
  sed -n 's/^version = "\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)"/\1/p' \
    "$root/Cargo.toml" | head -n 1
)
if [[ $tag != "v${manifest}" ]]; then
  printf 'tag %s does not match Cargo.toml version %s\n' "$tag" "$manifest" >&2
  exit 1
fi

shopt -s nullglob
tarballs=( "$darwin_dir"/tutorial-stepper-*-aarch64-darwin.tar.gz )
shopt -u nullglob
if [[ ${#tarballs[@]} -ne 1 ]]; then
  printf 'expected one Apple Silicon tarball in %s\n' "$darwin_dir" >&2
  exit 1
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$root"
gh release download "$tag" --pattern 'tutorial-stepper-*-linux' --dir "$work"
cp "${tarballs[0]}" "$work/"
bash "$root/scripts/release-checksums.sh" "$work"

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$work" && sha256sum -c SHA256SUMS)
else
  (cd "$work" && shasum -a 256 -c SHA256SUMS)
fi

gh release upload "$tag" \
  "$work/$(basename "${tarballs[0]}")" \
  "$work/SHA256SUMS" \
  --clobber
