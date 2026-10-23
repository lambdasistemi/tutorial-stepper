#!/usr/bin/env bash
# Run each release binary in a clean Ubuntu 24.04 container.
# Usage: ubuntu-binaries.sh ARTIFACTS EXAMPLES
#
# The x86_64 binary runs in ubuntu:24.04 for that architecture.
# The arm64 binary runs under Ubuntu 24.04's qemu-user-static, inside
# the same amd64 image. That emulator runs this fixed-address static
# binary. A position-independent build segfaults there.
set -euo pipefail

if [[ $# -ne 2 ]]; then
  printf 'usage: ubuntu-binaries.sh ARTIFACTS EXAMPLES\n' >&2
  exit 2
fi

artifacts=$(cd "$1" && pwd)
examples=$(cd "$2" && pwd)
tutorial="$examples/one-window"

if [[ ! -f "$tutorial/tutorial.toml" ]]; then
  printf 'no tutorial at %s\n' "$tutorial" >&2
  exit 1
fi

shopt -s nullglob
x86=( "$artifacts"/tutorial-stepper-*-x86_64-linux )
arm=( "$artifacts"/tutorial-stepper-*-aarch64-linux )
shopt -u nullglob

if [[ ${#x86[@]} -ne 1 || ${#arm[@]} -ne 1 ]]; then
  printf 'expected one x86_64 binary and one aarch64 binary in %s\n' "$artifacts" >&2
  ls -1 "$artifacts" >&2
  exit 1
fi

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
cp -L "${x86[0]}" "$stage/x86"
cp -L "${arm[0]}" "$stage/arm"
chmod +x "$stage/x86" "$stage/arm"

version_of() {
  local base rest
  base=$(basename "$1")
  rest=${base#tutorial-stepper-}
  if [[ $base == *-x86_64-linux ]]; then
    printf '%s' "${rest%-x86_64-linux}"
  else
    printf '%s' "${rest%-aarch64-linux}"
  fi
}

expect_version() {
  local bin=$1
  local got=$2
  local version
  version=$(version_of "$bin")
  printf '%s\n' "$got"
  if [[ "$got" != "tutorial-stepper ${version} (commit "*")" ]]; then
    printf 'version mismatch for %s\n' "$(basename "$bin")" >&2
    exit 1
  fi
}

run_direct() {
  local platform=$1
  local bin=$2
  local source=$3
  local got
  got=$(
    docker run --rm --platform "$platform" \
      -v "$bin:/usr/local/bin/tutorial-stepper:ro" \
      -v "$tutorial:/tutorial:ro" \
      ubuntu:24.04 \
      tutorial-stepper --version
  )
  expect_version "$source" "$got"
  docker run --rm --platform "$platform" \
    -v "$bin:/usr/local/bin/tutorial-stepper:ro" \
    -v "$tutorial:/tutorial:ro" \
    ubuntu:24.04 \
    tutorial-stepper check /tutorial
}

run_arm() {
  local bin=$1
  local source=$2
  local got first
  # One container installs the emulator once, then runs both commands.
  # apt's chatter stays on stderr. The first stdout line is --version.
  got=$(
    docker run --rm --platform linux/amd64 \
      -v "$bin:/usr/local/bin/tutorial-stepper:ro" \
      -v "$tutorial:/tutorial:ro" \
      ubuntu:24.04 \
      bash -c 'set -euo pipefail
        export DEBIAN_FRONTEND=noninteractive
        apt-get update -qq
        apt-get install -y -qq qemu-user-static >/dev/null
        qemu-aarch64-static /usr/local/bin/tutorial-stepper --version
        qemu-aarch64-static /usr/local/bin/tutorial-stepper check /tutorial'
  )
  first=${got%%$'\n'*}
  expect_version "$source" "$first"
  printf '%s\n' "${got#"$first"}"
}

run_direct linux/amd64 "$stage/x86" "${x86[0]}"
run_arm "$stage/arm" "${arm[0]}"
