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

  # The formula a release pushes. Fails while the renderer still prints
  # the template's placeholders.
  homebrew-formula = pkgs.runCommand "tutorial-stepper-homebrew-formula"
    {
      nativeBuildInputs = [
        pkgs.ruby
        pkgs.bash
        pkgs.coreutils
        pkgs.gnugrep
        pkgs.glibcLocales
      ];
      LANG = "C.UTF-8";
      LC_ALL = "C.UTF-8";
    }
    ''
      set -euo pipefail
      mkdir -p scripts packaging/homebrew
      cp ${../scripts/render-homebrew-formula.sh} scripts/render-homebrew-formula.sh
      cp ${../packaging/homebrew/tutorial-stepper.rb.in} \
        packaging/homebrew/tutorial-stepper.rb.in
      chmod +x scripts/render-homebrew-formula.sh
      sha=abababababababababababababababababababababababababababababababab
      formula=$(bash scripts/render-homebrew-formula.sh 1.2.3 "$sha")
      printf '%s\n' "$formula" > formula.rb
      if ! printf '%s\n' "$formula" | grep -F "sha256 \"$sha\""; then
        printf 'rendered formula does not carry the checksum\n' >&2
        printf '%s\n' "$formula" >&2
        exit 1
      fi
      printf '%s\n' "$formula" | grep -F 'depends_on "tmux"' >/dev/null
      printf '%s\n' "$formula" | grep -F \
        'tutorial-stepper-1.2.3-aarch64-darwin.tar.gz' >/dev/null
      printf '%s\n' "$formula" | grep -F 'version "1.2.3"' >/dev/null
      printf '%s\n' "$formula" | grep -F 'class TutorialStepper' >/dev/null
      printf '%s\n' "$formula" | grep -F 'bin.install "bin/tutorial-stepper"' \
        >/dev/null
      if printf '%s\n' "$formula" | grep -F '@'; then
        printf 'placeholder left in the formula\n' >&2
        exit 1
      fi
      ruby -c formula.rb
      local_url='file:///tmp/fresh/tutorial-stepper-1.2.3-aarch64-darwin.tar.gz'
      local_formula=$(bash scripts/render-homebrew-formula.sh 1.2.3 "$sha" "$local_url")
      printf '%s\n' "$local_formula" | grep -F "url \"$local_url\"" >/dev/null
      if bash scripts/render-homebrew-formula.sh 1.2 "$sha"; then
        printf 'a short version was accepted\n' >&2
        exit 1
      fi
      if bash scripts/render-homebrew-formula.sh 1.2.3 xyz; then
        printf 'a short checksum was accepted\n' >&2
        exit 1
      fi
      touch "$out"
    '';

  # The checksum file a learner downloads. Fails while the Apple Silicon
  # tarball is optional.
  release-checksums = pkgs.runCommand "tutorial-stepper-release-checksums"
    {
      nativeBuildInputs = [
        pkgs.bash
        pkgs.coreutils
        pkgs.gnugrep
        pkgs.glibcLocales
      ];
      LANG = "C.UTF-8";
      LC_ALL = "C.UTF-8";
    }
    ''
      set -euo pipefail
      script=${../scripts/release-checksums.sh}
      work=$(mktemp -d)
      cd "$work"
      printf 'x' > tutorial-stepper-1.2.3-x86_64-linux
      printf 'y' > tutorial-stepper-1.2.3-aarch64-linux
      printf 'z' > tutorial-stepper-1.2.3-aarch64-darwin.tar.gz
      bash "$script" "$work"
      if ! grep -F tutorial-stepper-1.2.3-aarch64-darwin.tar.gz SHA256SUMS; then
        printf 'SHA256SUMS does not list the Apple Silicon tarball\n' >&2
        cat SHA256SUMS >&2
        exit 1
      fi
      sha256sum -c SHA256SUMS
      test "$(wc -l < SHA256SUMS)" -eq 3
      rm tutorial-stepper-1.2.3-aarch64-darwin.tar.gz SHA256SUMS
      if bash "$script" "$work"; then
        printf 'a directory without the tarball was accepted\n' >&2
        exit 1
      fi
      printf 'z' > tutorial-stepper-1.2.3-aarch64-darwin.tar.gz
      printf 'old' > tutorial-stepper-linux-x86_64
      if bash "$script" "$work"; then
        printf 'the old Linux name was accepted\n' >&2
        exit 1
      fi
      rm tutorial-stepper-linux-x86_64
      mv tutorial-stepper-1.2.3-aarch64-darwin.tar.gz \
        tutorial-stepper-9.9.9-aarch64-darwin.tar.gz
      if bash "$script" "$work"; then
        printf 'mixed versions were accepted\n' >&2
        exit 1
      fi
      touch "$out"
    '';

  # The script the macOS job runs after install. Fails while zsh is not played.
  play-conformance = pkgs.runCommand "tutorial-stepper-play-conformance"
    {
      nativeBuildInputs = [
        tutorial-stepper
        pkgs.bashInteractive
        pkgs.zsh
        pkgs.tmux
        pkgs.coreutils
        pkgs.glibcLocales
      ];
      LANG = "C.UTF-8";
      LC_ALL = "C.UTF-8";
    }
    ''
      set -euo pipefail
      bash ${../scripts/play-conformance.sh} ${../examples}
      touch "$out"
    '';
}
