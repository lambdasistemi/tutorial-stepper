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

  # cleanCargoSource drops Markdown. The example tutorials are the
  # tests, so their pages have to stay in the source.
  keepExamples = path: _type:
    let
      base = builtins.baseNameOf path;
    in
    base == "examples" || pkgs.lib.hasInfix "/examples/" path;

  commonArgs = {
    src = pkgs.lib.cleanSourceWith {
      src = src;
      filter = path: type:
        (craneLib.filterCargoSources path type) || (keepExamples path type);
    };
    strictDeps = true;
    pname = "tutorial-stepper";
    inherit version;
    # The play tests start a tmux server and a bash or zsh, and they
    # ask `ps` whether that server exited. Those programs are
    # build-time tools, not libraries the binary links.
    nativeBuildInputs = [
      pkgs.tmux
      pkgs.bashInteractive
      pkgs.zsh
      pkgs.procps
    ];
    buildInputs = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
      pkgs.libiconv
    ];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
{
  inherit craneLib commonArgs cargoArtifacts version commit;
}
