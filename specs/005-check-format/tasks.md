# Tasks: Check a tutorial's format

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the code tasks
**Tests**: required. Each behaviour starts as a test that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/005-check-format/spec.md`, `plan.md`, and `tasks.md`, including the open index-file decision, and commit them before any code.

## Phase 2: Crate, flake, and continuous integration

These do not read the index file.

- [x] Add the Rust crate at the repository root: `Cargo.toml`, committed `Cargo.lock`, `rust-toolchain.toml`, `rustfmt.toml`, `deny.toml`. The binary's `--version` and `--help` meet the command-line bar, covered by a test.
- [x] Add the Nix flake (`flake.nix`, `nix/toolchain.nix`, `nix/crane.nix`, `nix/packages.nix`, `nix/checks.nix`, `nix/release.nix`, `justfile`) so `nix flake check` runs formatting, clippy, the tests, cargo-deny, and rustdoc.
- [ ] Replace `.github/workflows/ci.yml`. Keep a job named `Build Gate`. Add Format, Clippy, Tests, Supply chain, API reference, and Development shell, each depending on the build gate, all on `runs-on: nixos`.
- [ ] Add release-please (`release-please-config.json`, `.release-please-manifest.json`, `.github/workflows/release.yml`) for the Rust crate. The release pull request is opened with a token minted from the organisation app. The release job uploads the Linux x86_64 binary and `SHA256SUMS`.

## Phase 3: Fence attributes

Independent of the index file. Test first.

- [x] A failing test for a fence `bash { window=bob status=19 mode=observe if=SIGNING:external }` yielding those four attributes and the fence's line.
- [x] A failing test for each rejected spelling — a JSON object, bare words, Pandoc's `{.lang …}` — asserting the error names the line.
- [x] A failing test for an unknown attribute, and for a `status` or `mode` value outside the format.
- [x] The smallest parser that makes those tests pass. A language with no braces stays an ordinary code block.

## Phase 4: The index and `check`

The operator chose `tutorial.toml`. The specification records the contents.

- [x] A failing test per index rule: a window that is not declared, a page the index does not list, an index entry whose file is missing, an `if` naming an undeclared setting.
- [x] The index parser for the decided file, and `tutorial-stepper check DIR` printing the confirmation or `FILE:LINE: message`.
- [x] One valid tutorial under `examples/` for each attribute, and one tutorial under `examples/invalid/` for each of the six error kinds. `cargo test` runs the binary on them.
- [x] `docs/format.md` (version 0, one example per attribute) and the README status, in the same commit as the behaviour they describe.

## Phase 5: Verification

- [ ] `nix flake check`
- [ ] `nix build .#default`, then `tutorial-stepper check` on each example, exit 0 for the valid ones and non-zero with `FILE:LINE: message` for the invalid ones.
- [ ] `actionlint` clean on the workflow files.
- [ ] The documentation presentation check clean on `README.md` and `docs/`.

## Dependencies

Phase 2 and phase 3 can proceed while phase 4 waits. Phase 4 starts from the operator's answer and does not reopen the fence spelling. Phase 5 runs last, on the commit that contains the examples and `docs/format.md`.
