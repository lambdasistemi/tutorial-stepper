{
  description = "Check a Markdown tutorial's format, and step its commands into real shells";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , crane
    , rust-overlay
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustToolchain = import ./nix/toolchain.nix { inherit pkgs; };

        craneEnv = import ./nix/crane.nix {
          inherit pkgs crane rustToolchain self;
          src = ./.;
        };

        packages = import ./nix/packages.nix { inherit craneEnv; };

        checks = import ./nix/checks.nix {
          inherit pkgs craneEnv;
          tutorial-stepper = packages.tutorial-stepper;
        };
      in
      {
        packages = {
          default = packages.tutorial-stepper;
          inherit (packages) tutorial-stepper tutorial-stepper-static;
          release-artifacts = import ./nix/release.nix {
            inherit pkgs packages;
          };
        };

        inherit checks;

        devShells.default = craneEnv.craneLib.devShell {
          packages = [
            pkgs.just
            pkgs.cargo-deny
            pkgs.tmux
            pkgs.bashInteractive
            pkgs.zsh
            pkgs.procps
          ];
          env = {
            TUTORIAL_STEPPER_VERSION = craneEnv.version;
            TUTORIAL_STEPPER_COMMIT = craneEnv.commit;
          };
        };
      }
    );
}
