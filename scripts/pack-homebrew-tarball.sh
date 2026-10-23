#!/usr/bin/env bash
# Pack a staged Apple Silicon build into the Homebrew tarball.
# Usage: pack-homebrew-tarball.sh STAGE VERSION DESTDIR
#
# STAGE holds bin/tutorial-stepper and, when libraries were copied,
# libexec/lib/. The archive is
# DESTDIR/tutorial-stepper-<version>-aarch64-darwin.tar.gz.
set -euo pipefail

if [[ $# -ne 3 ]]; then
  printf 'usage: pack-homebrew-tarball.sh STAGE VERSION DESTDIR\n' >&2
  exit 2
fi

stage=$1
version=$2
dest=$3

if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'version must be three numbers, such as 0.2.0\n' >&2
  exit 1
fi

if [[ ! -f $stage/bin/tutorial-stepper ]]; then
  printf 'binary is missing: %s\n' "$stage/bin/tutorial-stepper" >&2
  exit 1
fi

# One top-level directory. Homebrew enters it before install, and the
# formula then finds bin/tutorial-stepper. A tarball whose only
# directory is bin/ makes that path disappear.
wrap=$(mktemp -d)
trap 'rm -rf "$wrap"' EXIT
mkdir -p "$wrap/tutorial-stepper"
cp -R "$stage/bin" "$wrap/tutorial-stepper/bin"
if [[ -d $stage/libexec ]]; then
  cp -R "$stage/libexec" "$wrap/tutorial-stepper/libexec"
fi

mkdir -p "$dest"
archive=$dest/tutorial-stepper-${version}-aarch64-darwin.tar.gz
COPYFILE_DISABLE=1 tar -czf "$archive" -C "$wrap" tutorial-stepper
printf '%s\n' "$archive"
