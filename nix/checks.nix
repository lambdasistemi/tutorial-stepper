# Each check is a sandboxed crane derivation. `nix flake check` runs
# clippy, rustfmt, the tests, cargo-deny, and rustdoc. It does not
# merely write a script.
{ pkgs
, craneEnv
, tutorial-stepper
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
}
