#!/usr/bin/env bash
# Write SHA256SUMS for one release directory.
# Usage: release-checksums.sh DIRECTORY
#
# The directory must hold three files, all for the same version:
# tutorial-stepper-<version>-x86_64-linux,
# tutorial-stepper-<version>-aarch64-linux, and
# tutorial-stepper-<version>-aarch64-darwin.tar.gz.
# The old name tutorial-stepper-linux-x86_64 is refused.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  printf 'usage: release-checksums.sh DIRECTORY\n' >&2
  exit 2
fi

dir=$(cd "$1" && pwd)
cd "$dir"

if [[ -e tutorial-stepper-linux-x86_64 ]]; then
  printf 'old name still present: tutorial-stepper-linux-x86_64\n' >&2
  exit 1
fi

shopt -s nullglob
names=( tutorial-stepper-* )
shopt -u nullglob

x86=()
arm=()
darwin=()
other=()
for name in "${names[@]}"; do
  case "$name" in
    tutorial-stepper-*-x86_64-linux) x86+=("$name") ;;
    tutorial-stepper-*-aarch64-linux) arm+=("$name") ;;
    tutorial-stepper-*-aarch64-darwin.tar.gz) darwin+=("$name") ;;
    *) other+=("$name") ;;
  esac
done

if [[ ${#x86[@]} -ne 1 || ${#arm[@]} -ne 1 || ${#darwin[@]} -ne 1 || ${#other[@]} -ne 0 ]]; then
  printf 'expected one x86_64 binary, one aarch64 binary, and one aarch64-darwin tarball\n' >&2
  printf 'found:\n' >&2
  printf '%s\n' "${names[@]:-<nothing>}" >&2
  exit 1
fi

version_of() {
  local name=$1
  local rest=${name#tutorial-stepper-}
  case "$name" in
    *-x86_64-linux) printf '%s' "${rest%-x86_64-linux}" ;;
    *-aarch64-linux) printf '%s' "${rest%-aarch64-linux}" ;;
    *-aarch64-darwin.tar.gz) printf '%s' "${rest%-aarch64-darwin.tar.gz}" ;;
  esac
}

linux_x86=$(version_of "${x86[0]}")
linux_arm=$(version_of "${arm[0]}")
mac=$(version_of "${darwin[0]}")
if [[ $linux_x86 != "$linux_arm" || $linux_x86 != "$mac" ]]; then
  printf 'versions differ: %s %s %s\n' "$linux_x86" "$linux_arm" "$mac" >&2
  exit 1
fi

if [[ ! $linux_x86 =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'version %s is not three numbers\n' "$linux_x86" >&2
  exit 1
fi

hash_one() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1"
  else
    shasum -a 256 "$1"
  fi
}

{
  hash_one "${x86[0]}"
  hash_one "${arm[0]}"
  hash_one "${darwin[0]}"
} | sort -k 2 > SHA256SUMS
