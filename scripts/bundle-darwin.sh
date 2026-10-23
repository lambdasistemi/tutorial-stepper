#!/usr/bin/env bash
# Pack the Apple Silicon binary into a tarball Homebrew can install.
# Usage: bundle-darwin.sh BINARY VERSION DESTDIR
#
# The tarball is DESTDIR/tutorial-stepper-<version>-aarch64-darwin.tar.gz
# and holds bin/tutorial-stepper. A library that is not part of macOS is
# copied to libexec/lib and the load path is rewritten. A path that still
# points into the Nix store is a failed build.
set -euo pipefail

if [[ $# -ne 3 ]]; then
  printf 'usage: bundle-darwin.sh BINARY VERSION DESTDIR\n' >&2
  exit 2
fi

binary=$1
version=$2
dest=$3

if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'version must be three numbers, such as 0.2.0\n' >&2
  exit 1
fi

if [[ ! -f $binary ]]; then
  printf 'binary is missing: %s\n' "$binary" >&2
  exit 1
fi

if ! command -v otool >/dev/null 2>&1 || ! command -v install_name_tool >/dev/null 2>&1; then
  printf 'otool and install_name_tool are required\n' >&2
  exit 1
fi

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/bin" "$stage/libexec/lib"
cp -L "$binary" "$stage/bin/tutorial-stepper"
chmod 755 "$stage/bin/tutorial-stepper"

rpaths_of() {
  otool -l "$1" | awk '
    $1 == "cmd" && $2 == "LC_RPATH" { show = 1; next }
    show && $1 == "path" { print $2; show = 0 }
  '
}

resolve_dep() {
  local file=$1
  local dep=$2
  local base rpath candidate
  case "$dep" in
    /usr/lib/*|/System/*|@executable_path/*)
      printf '%s' ""
      return 0
      ;;
    @rpath/*)
      base=${dep#@rpath/}
      while IFS= read -r rpath; do
        candidate=${rpath%/}/$base
        if [[ -f $candidate ]]; then
          printf '%s' "$candidate"
          return 0
        fi
      done < <(rpaths_of "$file")
      printf 'cannot resolve %s from %s\n' "$dep" "$file" >&2
      return 1
      ;;
    @loader_path/*)
      base=${dep#@loader_path/}
      candidate=$(cd "$(dirname "$file")" && pwd)/$base
      if [[ -f $candidate ]]; then
        printf '%s' "$candidate"
        return 0
      fi
      printf 'cannot resolve %s from %s\n' "$dep" "$file" >&2
      return 1
      ;;
    *)
      printf '%s' "$dep"
      ;;
  esac
}

pending="$stage/bin/tutorial-stepper"
while [[ -n $pending ]]; do
  next=""
  for file in $pending; do
    deps=$(otool -L "$file" | awk 'NR > 1 { print $1 }')
    for dep in $deps; do
      case "$dep" in
        /usr/lib/*|/System/*|@executable_path/*) continue ;;
      esac
      source_path=$(resolve_dep "$file" "$dep")
      base=$(basename "$source_path")
      target=$stage/libexec/lib/$base
      if [[ ! -f $target ]]; then
        if [[ ! -f $source_path ]]; then
          printf 'library is missing: %s\n' "$source_path" >&2
          exit 1
        fi
        cp -L "$source_path" "$target"
        chmod 755 "$target"
        install_name_tool -id "@executable_path/../libexec/lib/$base" "$target"
        next="$next $target"
      fi
      install_name_tool -change "$dep" "@executable_path/../libexec/lib/$base" "$file"
    done
  done
  pending=$next
done

leftover=$(
  {
    otool -L "$stage/bin/tutorial-stepper"
    if compgen -G "$stage/libexec/lib/*" >/dev/null; then
      otool -L "$stage/libexec/lib"/*
    fi
  } | grep -F /nix/store || true
)
if [[ -n $leftover ]]; then
  printf 'a library path still points into the Nix store\n' >&2
  printf '%s\n' "$leftover" >&2
  exit 1
fi

codesign --force --sign - "$stage/bin/tutorial-stepper"
if compgen -G "$stage/libexec/lib/*" >/dev/null; then
  codesign --force --sign - "$stage/libexec/lib"/*
else
  rm -rf "$stage/libexec"
fi

mkdir -p "$dest"
archive=$dest/tutorial-stepper-${version}-aarch64-darwin.tar.gz
if [[ -d $stage/libexec ]]; then
  COPYFILE_DISABLE=1 tar -czf "$archive" -C "$stage" bin libexec
else
  COPYFILE_DISABLE=1 tar -czf "$archive" -C "$stage" bin
fi
printf '%s\n' "$archive"
