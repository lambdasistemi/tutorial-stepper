# Tasks: Documentation site

**Input**: [spec.md](spec.md), [plan.md](plan.md)
**Prerequisites**: the specification is committed before any of the site tasks
**Tests**: required. Each behaviour starts as a check that fails because the behaviour is absent.

## Phase 1: Specification

- [x] Write `specs/019-docs-site/spec.md`, `plan.md`, and `tasks.md`, and commit them before any site file.

## Phase 2: A strict build

Test first. The subject is `mkdocs build --strict` and the Terminal palette switch.

- [x] A failing check that the built front page exists, names Terminal's palette switch, and was built strictly. The check fails while `mkdocs.yml` is absent.
- [x] The smallest change that makes that check pass: the theme, the palette switch, the Nix site derivation, and a documentation workflow that builds the check on a pull request and does not deploy it.

## Phase 3: The front page

Test first. The subject is the built front page.

- [ ] A failing check that the front page has a story heading, the learner's story, the author's story, and one context diagram.
- [ ] The smallest change that makes that check pass: `docs/index.md`.

## Phase 4: The format, the commands, and the guide

Test first. The subject is the built pages and the fences on them.

- [ ] A failing check that the format specification, the command reference, and the author's guide are in the built site, that the guide contains a command fence in the format, and that a fence with a bad spelling is rejected. Every fence in `docs/` is read by that check.
- [ ] The smallest change that makes that check pass: `docs/commands.md`, `docs/author.md`, and the pages already in `docs/` linked from the navigation.

## Phase 5: The presentation check

Test first. The subject is `tools/check_presentation.py` on `docs/`.

- [ ] A failing check that the presentation check reports no violations for the site's pages. Speech companions are not required.
- [ ] Copy the presentation check into `tools/` and run it from the Nix check and from the documentation workflow.

## Phase 6: The library reference and the deploy

Test first. The subject is the built site's `api/` directory and the workflow file.

- [ ] A failing check that `api/tutorial_stepper/index.html` is in the built site and that the library page embeds it, and that the workflow deploys from `main` with `actions/upload-pages-artifact` and `actions/deploy-pages`.
- [ ] The smallest change that makes that check pass: rustdoc copied into the site, `docs/library.md`, and the deploy job.

## Phase 7: Verification

- [ ] `nix flake check`
- [ ] The site served on `127.0.0.1`, the front page and one page of each section fetched.
- [ ] `tools/check_presentation.py` on the site's pages.
- [ ] `nix shell nixpkgs#actionlint -c actionlint`
- [ ] Every diagram rendered and looked at, in light and dark, at a desktop width and a phone width.

## Dependencies

Phase 2 starts only after phase 1 is committed. Phases 2 through 6 each leave the suite green. Phase 7 runs last. The pull-request job to require is Documentation site, together with the jobs the repository already requires: Build Gate, Format, Clippy, Tests, Supply chain, API reference, Development shell, Release binaries, Ubuntu binaries, Homebrew formula, and macOS formula. Deploy documentation runs only on `main`.
