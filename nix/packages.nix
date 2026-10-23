# The binary a person runs, and the statically linked Linux builds a
# release attaches. All three are the same crate. The commit and the
# version are compiled in, so `--version` on a packaged build names
# the revision.
{ craneEnv }:
let
  inherit (craneEnv) craneLib commonArgs cargoArtifacts version commit rustLld;

  identity = {
    TUTORIAL_STEPPER_VERSION = version;
    TUTORIAL_STEPPER_COMMIT = commit;
  };

  tutorial-stepper = craneLib.buildPackage (commonArgs // identity // {
    inherit cargoArtifacts;
  });

  # Pure Rust, so the musl target shipped by the toolchain is enough.
  # No C toolchain is linked in. Each target has its own dependency
  # build. The tests run in the host check; a cross build cannot run
  # them, and the x86_64 static build would only repeat that suite.
  #
  # The arm64 build cannot use the host `cc`: that driver calls the
  # host linker, which rejects arm64 objects. The toolchain's own lld
  # links every architecture, and the musl target carries its objects.
  # The result is a fixed-address static executable. Ubuntu 24.04's
  # qemu-user-static runs that binary; a position-independent link
  # segfaults under that emulator.
  staticPackage = { target, pname, extraEnv ? { } }:
    let
      # The commit stays out of the dependency build. It changes on
      # every revision and would throw away the cached crates. build.rs
      # falls back while those crates build, and the package build
      # below passes the real commit.
      args = commonArgs // extraEnv // {
        inherit pname;
        CARGO_BUILD_TARGET = target;
        CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
        doCheck = false;
      };
      deps = craneLib.buildDepsOnly args;
    in
    craneLib.buildPackage (args // identity // {
      cargoArtifacts = deps;
    });

  tutorial-stepper-static = staticPackage {
    target = "x86_64-unknown-linux-musl";
    pname = "tutorial-stepper-static";
  };

  tutorial-stepper-static-aarch64 = staticPackage {
    target = "aarch64-unknown-linux-musl";
    pname = "tutorial-stepper-static-aarch64";
    extraEnv = {
      CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER = rustLld;
      CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS =
        "-C target-feature=+crt-static -C linker-flavor=ld.lld -C link-self-contained=yes";
    };
  };
in
{
  inherit
    tutorial-stepper
    tutorial-stepper-static
    tutorial-stepper-static-aarch64
    ;
}
