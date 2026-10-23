#!/usr/bin/env bash
# Print the Homebrew formula for one version and one checksum.
# Usage: render-homebrew-formula.sh VERSION SHA256 [URL]
#
# VERSION is three numbers, such as 0.2.0. SHA256 is 64 hex digits.
# When URL is omitted, the archive is the Apple Silicon tarball on the
# release for that version.
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  printf 'usage: render-homebrew-formula.sh VERSION SHA256 [URL]\n' >&2
  exit 2
fi

version=$1
sha=$2
url=${3:-}

if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'version must be three numbers, such as 0.2.0\n' >&2
  exit 1
fi

if [[ ! $sha =~ ^[0-9a-f]{64}$ ]]; then
  printf 'checksum must be 64 hex digits\n' >&2
  exit 1
fi

if [[ -z $url ]]; then
  url="https://github.com/lambdasistemi/tutorial-stepper/releases/download/v${version}/tutorial-stepper-${version}-aarch64-darwin.tar.gz"
fi

root=$(dirname "$0")/..
template=$root/packaging/homebrew/tutorial-stepper.rb.in
if [[ ! -f $template ]]; then
  printf 'formula template is missing: %s\n' "$template" >&2
  exit 1
fi

sed \
  -e "s|@VERSION@|${version}|g" \
  -e "s|@SHA256@|${sha}|g" \
  -e "s|@URL@|${url}|g" \
  "$template"
