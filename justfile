set unstable := true

default: ci

ci:
    nix flake check

build:
    nix build .#default

test:
    nix build .#checks.x86_64-linux.nextest

clippy:
    nix build .#checks.x86_64-linux.clippy

fmt:
    nix develop -c cargo fmt --all

fmt-check:
    nix build .#checks.x86_64-linux.fmt

deny:
    nix build .#checks.x86_64-linux.deny

doc:
    nix build .#checks.x86_64-linux.doc

# Fast inner loop. The gate is `just ci`.
cargo-test:
    nix develop -c cargo test
