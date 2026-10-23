# Implementation Plan: Documentation site

**Branch**: `feat/19-docs-site` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/019-docs-site/spec.md`

## Summary

An MkDocs site, theme Terminal, is built strictly on pull requests and deployed to GitHub Pages from `main`. The front page carries the learner's story, the author's story, and a context diagram. The site publishes the format specification, the command reference, the author's guide with one worked example, and the pages already in `docs/`. The library reference is rustdoc from this revision, copied into the built site. The presentation check is copied into `tools/` and runs in the same derivation as the strict build.

## Technical Context

**Language/Version**: The site is MkDocs on the Python that Nixpkgs provides. The theme is `mkdocs-terminal` 4.8.0, pinned by the wheel hash. The library reference is rustdoc from the toolchain the flake already pins.
**Primary Dependencies**: MkDocs, `pymdownx.superfences`, `mkdocs-terminal`. No new Rust crate. Mermaid 11.17.2 is loaded by the page to draw the diagrams.
**Storage**: N/A. The site is a directory of static files.
**Testing**: A sandboxed check builds the site with `mkdocs build --strict`, runs the presentation check on `docs/`, and reads the built pages. It fails while the theme, the front page, the guide, the command reference, or the library reference is missing. `actionlint` reads the workflow.
**Target Platform**: GitHub Pages, project site `https://lambdasistemi.github.io/tutorial-stepper/`. Built on the organisation's NixOS runners.
**Project Type**: the existing repository, plus a documentation site.
**Performance Goals**: the front page and one page of each section open from a local static server.
**Constraints**: Block attributes stay `lang { key=value }`. The index file stays `tutorial.toml`. Narration is out of scope. The release manifest is not edited. Pages workflow mode is turned on by the parent. The new job name is listed when the work is done.
**Scale/Scope**: the MkDocs configuration, the theme switch, the new pages, the Nix site derivation, the presentation check, and the documentation workflow.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan meets it |
| --- | --- |
| I. The Stepper Is the Product | The site explains the stepper. It does not step commands. |
| II. The Source Is Plain CommonMark | The published pages are the pages already in `docs/`, plus the front page, the command reference, and the author's guide. Tutorial blocks stay `lang { key=value }` and render as ordinary code blocks through `pymdownx.superfences`. |
| X. No Code for Any One Tutorial | The worked example is ordinary tutorial files. The stepper is not taught a special case for it. |
| XI. The Record Outranks the Code | This specification is committed before the site. Each page lands in the same commit as the check that requires it. |
| Platform and Distribution | The site is GitHub Pages. The library reference is the rustdoc of this revision. Install instructions on the site repeat the limit on signing and notarisation. |
| Development Workflow | The pull-request job is a required check on the NixOS runners. release-please is unchanged. |

No departure.

## Project Structure

### Documentation (this feature)

```text
specs/019-docs-site/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
mkdocs.yml
docs/index.md                          # front page
docs/commands.md                       # command reference
docs/author.md                         # author's guide
docs/library.md                        # rustdoc, embedded
docs/overrides/                        # Terminal palette switch
docs/javascripts/palette.js
docs/javascripts/mermaid.js
docs/stylesheets/terminal.css
docs/format.md                         # already the format specification
docs/play.md                           # already the unattended run
docs/script.md                         # already the printed script
docs/terminal.md                       # already the terminal face
tools/check_presentation.py            # copied presentation check
tools/check_site.py                    # the built site
nix/docs.nix                           # strict build and the check
.github/workflows/docs.yml             # strict build on pull requests, deploy from main
```

The crate, the release workflow, and the release manifest stay as they are.

## How the site is built

`nix/docs.nix` builds two derivations. The site derivation runs `mkdocs build --strict` and copies `share/doc` from the rustdoc derivation into `api/` under the site. The check derivation runs that site, runs `tools/check_presentation.py --no-speech` on `docs/`, and runs `tools/check_site.py` on the built files. `nix flake check` builds the check, so a missing page fails the same gate as the Rust checks.

The cargo source filter does not gain the Markdown pages. A docs edit rebuilds the site and does not rebuild the crate.

The documentation workflow is separate from the existing continuous-integration workflow. That workflow cancels a stale run. A Pages deploy must not be cancelled once it has started, so the documentation workflow sets `cancel-in-progress: false`. On a pull request it builds the check and stops. On a push to `main` it builds the site, uploads the directory with `actions/upload-pages-artifact`, and deploys it with `actions/deploy-pages`.

## Decisions

| Chosen | Rejected | Why |
| --- | --- | --- |
| Terminal, with a palette script that remembers the reader's choice | Material; Terminal's static `palette: default` alone | The issue names Terminal and a light and dark switch. The static palette follows the system once and cannot be switched. |
| rustdoc copied under `api/`, embedded from a page | Converting rustdoc to Markdown | rustdoc has no stable Markdown export. The generated pages keep their own links and their source view. |
| Presentation check copied into `tools/`, speech turned off | Calling the check from a home directory; generating narration | Continuous integration has no home-directory skill tree. The issue lists narration as a non-goal. |
| A documentation workflow beside the existing one | Adding the deploy to the existing workflow | The existing workflow cancels stale runs. Cancelling a Pages deploy leaves a partial site. |
| The author's guide shows `examples/one-window` written out | A second, larger tutorial invented for the page | The conformance example is the tutorial the repository already plays. The guide's command is the command the repository runs. |
