# The toolchain named in rust-toolchain.toml, so a Nix build and a
# developer shell compile with the same channel, components, and targets.
{ pkgs }:
pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml
