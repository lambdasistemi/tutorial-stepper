# The binary a person runs, and the statically linked Linux build a
# release attaches. Both are the same crate. The commit and the version
# are compiled in, so `--version` on a packaged build names the revision.
{ craneEnv }:
let
  inherit (craneEnv) craneLib commonArgs cargoArtifacts version commit;

  identity = {
    TUTORIAL_STEPPER_VERSION = version;
    TUTORIAL_STEPPER_COMMIT = commit;
  };

  tutorial-stepper = craneLib.buildPackage (commonArgs // identity // {
    inherit cargoArtifacts;
  });

  # Pure Rust, so the musl target shipped by rustup is enough. No C
  # toolchain is linked in.
  muslArgs = commonArgs // identity // {
    pname = "tutorial-stepper-static";
    CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
    CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
  };
  muslArtifacts = craneLib.buildDepsOnly muslArgs;
  tutorial-stepper-static = craneLib.buildPackage (muslArgs // {
    cargoArtifacts = muslArtifacts;
  });
in
{
  inherit tutorial-stepper tutorial-stepper-static;
}
