#!/usr/bin/env bash
# Check that a formula can install the files in one Homebrew tarball.
# Usage: check-homebrew-archive.sh ARCHIVE FORMULA
#
# Homebrew enters a single top-level directory before it runs install.
# Each path quoted by bin.install must name a file after that step.
set -euo pipefail

if [[ $# -ne 2 ]]; then
  printf 'usage: check-homebrew-archive.sh ARCHIVE FORMULA\n' >&2
  exit 2
fi

archive=$1
formula=$2

if [[ ! -f $archive || ! -f $formula ]]; then
  printf 'archive and formula must both exist\n' >&2
  exit 1
fi

mapfile -t listing < <(tar -tzf "$archive")

declare -A top_is_dir=()
for raw in "${listing[@]}"; do
  entry=${raw#./}
  entry=${entry#/}
  [[ -z $entry || $entry == . ]] && continue
  top=${entry%%/*}
  [[ $top == .* ]] && continue
  if [[ $entry == */* || $entry == */ ]]; then
    top_is_dir[$top]=1
  elif [[ -z ${top_is_dir[$top]+x} ]]; then
    top_is_dir[$top]=0
  fi
done

prefix=
if [[ ${#top_is_dir[@]} -eq 1 ]]; then
  for key in "${!top_is_dir[@]}"; do
    if [[ ${top_is_dir[$key]} -eq 1 ]]; then
      prefix=$key
    fi
  done
fi

adjusted=()
for raw in "${listing[@]}"; do
  [[ $raw == */ ]] && continue
  entry=${raw#./}
  entry=${entry#/}
  entry=${entry%/}
  [[ -z $entry ]] && continue
  if [[ -n $prefix ]]; then
    [[ $entry == "$prefix" ]] && continue
    entry=${entry#"$prefix/"}
  fi
  [[ -z $entry || $entry == */ ]] && continue
  adjusted+=("$entry")
done

mapfile -t install_paths < <(
  grep -F 'bin.install' "$formula" | grep -o '"[^"]*"' | tr -d '"'
)
if [[ ${#install_paths[@]} -eq 0 ]]; then
  printf 'formula does not install a binary\n' >&2
  exit 1
fi

missing=0
for path in "${install_paths[@]}"; do
  found=0
  for entry in "${adjusted[@]}"; do
    if [[ $entry == "$path" ]]; then
      found=1
      break
    fi
  done
  if [[ $found -eq 0 ]]; then
    printf 'formula installs %s, which is not in the archive after Homebrew enters a single top-level directory\n' \
      "$path" >&2
    missing=1
  fi
done

if [[ $missing -ne 0 ]]; then
  printf 'archive after that step:\n' >&2
  printf '%s\n' "${adjusted[@]}" >&2
  exit 1
fi

has_library=0
for entry in "${adjusted[@]}"; do
  if [[ $entry == libexec/lib/* ]]; then
    has_library=1
    break
  fi
done
if [[ $has_library -eq 1 ]] && ! grep -F 'libexec/lib' "$formula" >/dev/null; then
  printf 'archive has libexec/lib but the formula does not install it\n' >&2
  exit 1
fi
