# Each check is a sandboxed crane derivation. `nix flake check` runs
# clippy, rustfmt, the tests, cargo-deny, and rustdoc. It does not
# merely write a script.
{ pkgs
, craneEnv
, tutorial-stepper
, release-artifacts
}:
let
  inherit (craneEnv) craneLib commonArgs cargoArtifacts version;
in
{
  clippy = craneLib.cargoClippy (commonArgs // {
    inherit cargoArtifacts;
    cargoClippyExtraArgs = "--all-targets --all-features -- --deny warnings";
  });

  fmt = craneLib.cargoFmt { inherit (commonArgs) src; };

  nextest = craneLib.cargoNextest (commonArgs // {
    inherit cargoArtifacts;
  });

  deny = craneLib.cargoDeny { inherit (commonArgs) src; };

  doc = craneLib.cargoDoc (commonArgs // {
    inherit cargoArtifacts;
    env.RUSTDOCFLAGS = "--deny warnings";
  });

  # The packaged binary, not a developer build: the commit must be a
  # real revision. `unknown` is only for a build outside Nix.
  version = pkgs.runCommand "tutorial-stepper-version"
    {
      nativeBuildInputs = [ pkgs.glibcLocales ];
      LANG = "C.UTF-8";
      LC_ALL = "C.UTF-8";
    }
    ''
      set -euo pipefail
      got=$(${tutorial-stepper}/bin/tutorial-stepper --version)
      printf '%s\n' "$got" | grep -E \
        '^tutorial-stepper ${version} \(commit [0-9a-f]+(-dirty)?\)$'
      touch "$out"
    '';

  # The directory a release uploads. Fails while that directory still
  # carries the old single name, or a binary that is not static.
  release-binaries = pkgs.runCommand "tutorial-stepper-release-binaries"
    {
      nativeBuildInputs = [
        pkgs.file
        pkgs.coreutils
        pkgs.findutils
        pkgs.glibc
        pkgs.glibcLocales
      ];
      LANG = "C.UTF-8";
      LC_ALL = "C.UTF-8";
    }
    ''
      set -euo pipefail
      work=$(mktemp -d)
      cd ${release-artifacts}
      x86="tutorial-stepper-${version}-x86_64-linux"
      arm="tutorial-stepper-${version}-aarch64-linux"
      missing=0
      for name in "$x86" "$arm" SHA256SUMS; do
        if [[ ! -f $name ]]; then
          printf 'missing %s\n' "$name" >&2
          missing=1
        fi
      done
      if [[ -e tutorial-stepper-linux-x86_64 ]]; then
        printf 'old name still present: tutorial-stepper-linux-x86_64\n' >&2
        missing=1
      fi
      if [[ $missing -ne 0 ]]; then
        printf 'directory holds:\n' >&2
        ls -1 >&2
        exit 1
      fi
      sha256sum -c SHA256SUMS
      for pair in "$x86 x86-64" "$arm aarch64"; do
        name=''${pair%% *}
        arch=''${pair#* }
        desc=$(file "$name")
        printf '%s\n' "$desc"
        # file 5.46 and later says "static-pie linked" for a static
        # position-independent executable. ldd still says "statically linked".
        printf '%s\n' "$desc" | grep -E -q 'statically linked|static-pie linked'
        printf '%s\n' "$desc" | grep -q "$arch"
        ldd_out=$(ldd "$name" 2>&1 || true)
        printf '%s\n' "$ldd_out"
        printf '%s\n' "$ldd_out" | grep -E -q 'statically linked|not a dynamic executable'
      done
      find . -type f -printf '%P\n' | sort > "$work/found"
      printf '%s\n' SHA256SUMS "$arm" "$x86" | sort > "$work/expected"
      cmp "$work/found" "$work/expected"
      touch "$out"
    '';
}
