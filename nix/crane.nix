# crane bound to the pinned toolchain, plus the argument record shared
# by the package build and the checks. The commit is not part of the
# dependency build: it changes on every revision and would throw away
# the cached crates.
{ pkgs
, crane
, rustToolchain
, src
, self
}:
let
  craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  version = cargoToml.package.version;
  commit = self.rev or (self.dirtyRev or "unknown");

  commonArgs = {
    src = craneLib.cleanCargoSource src;
    strictDeps = true;
    pname = "tutorial-stepper";
    inherit version;
    buildInputs = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
      pkgs.libiconv
    ];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
{
  inherit craneLib commonArgs cargoArtifacts version commit;
}
