<!--
Sync Impact Report
- Version change: template → 1.0.0 (first ratification)
- Principles added: The Stepper Is the Product; The Source Is Plain CommonMark; Nothing Runs Unseen;
  A Tutorial Brings Its Own Environment; Every Tutorial Is a Test; The Shell Says When a Command Ended;
  A Cursor Over the Tutorial; No Code for Any One Tutorial; The Record Outranks the Code
- Sections added: Platform and Distribution; Development Workflow; Governance
- Templates: .specify/templates/plan-template.md ✅ no change needed (its Constitution Check is filled
  per plan from this file); spec-template.md ✅ no change; tasks-template.md ✅ no change
- Deferred: none
-->

# tutorial-stepper Constitution

tutorial-stepper takes a tutorial written in Markdown and steps its commands into real shells, one at
a time, beside the page that explains them. It knows when each command ended and with what status, so
the same tutorial a learner follows by hand also runs unattended as a test.

## Core Principles

### I. The Stepper Is the Product

The stepper sends a tutorial's commands into shells one at a time, knows when each one ended and with
what status, and reports it. Everything a person sees is a face on the stepper.

- A face MUST NOT step commands by itself. Stepping lives in one place.
- The faces in scope are the terminal face (the page beside the window where the command runs), its
  unattended mode, and the Killercoda renderer, which only renders: Killercoda steps by itself.
- Further faces (a browser page, an editor) are added only when a real tutorial needs them. Until then
  the stepper is a library inside the one binary, with no protocol for other languages.

Rationale: before this project, the Singular demo built the stepper three times over (a bash script for
the terminal, a Python server for the browser, a bash and JavaScript pair for its test), each
re-deciding how to find the commands, choose the window, send them and notice the end. The rendering
was the cheap part.

### II. The Source Is Plain CommonMark

A tutorial is one index file plus one CommonMark file per page. No syntax is invented.

- What a block needs beyond its language goes in the code fence's info string, after the language, as
  `key=value` pairs in braces: ` ```bash { window=bob status=19 } `. Values are single words.
- Nothing about a block is inferred from its text: which window it runs in, the exit status it is
  expected to end with, and whether it runs at all are written on the block.
- Pages MUST render as ordinary code blocks on GitHub, in Pandoc and in MkDocs with `superfences`, and
  MUST stay runnable block by block in Runme.
- The format, the index file and the meaning of every attribute are specified in this repository's
  documentation, versioned, and checked by a conformance suite of small example tutorials with their
  expected outcomes.
- Killercoda's scenario format is an output, written by the Killercoda renderer, never an input.

Rationale: the info string is CommonMark's own extension point. Of the four spellings compared, only
`lang { key=value }` kept the block intact in GitHub, Pandoc, MkDocs with `superfences` and Runme.
Markers after the fence (Killercoda's `{{exec}}`) and attributes guessed from command text (the Singular
demo's generator) are what this principle exists to end.

### III. Nothing Runs Unseen

- Every command the stepper sends is on the page before it is sent.
- A tutorial's set-up script is shown to the learner and runs only when they agree. It MUST NOT need
  root.
- The stepper installs nothing.
- A block that ends with a status other than the one it declares (zero when it declares none) is
  reported to the learner, never passed over.

Rationale: the stepper types into the learner's own shell on their own machine, not into a throwaway
machine as Killercoda does.

### IV. A Tutorial Brings Its Own Environment

- A tutorial ships as a package that brings everything its commands call. Its index file declares what
  its windows need: programs, and versions where they matter.
- Before the first page the stepper checks those needs and names whatever is missing. A tutorial MUST
  NOT fail halfway because a program is absent.
- The stepper's own reading of which programs the commands call is advice. The proof is an unattended
  run on a clean machine.
- Tutorials play in the learner's own shell. Running them in containers is out of scope.

### V. Every Tutorial Is a Test

- Every tutorial MUST play unattended from a clean start. The unattended mode exits non-zero at the
  first unexpected status or failed check, and writes a transcript: each command with its output, its
  status and its time.
- A tutorial that cannot play unattended is a defect in the tutorial or in the stepper.

### VI. The Shell Says When a Command Ended

- The end of a command and its exit status come from the shell's prompt hook, never from timeouts and
  never from reading the output. bash and zsh are supported.
- tmux hosts the shells. The stepper drives tmux and never reimplements a terminal.

### VII. A Cursor Over the Tutorial

The stepper keeps a position in the tutorial and nothing else.

- Its verbs are: run (type the command and press Enter), push (place the command at the prompt for the
  learner to edit and run; several lines go in as one bracketed paste), skip, interrupt (Ctrl-C) and
  back (move the position to an earlier command).
- The stepper does not model, save or restore the shell's state. Back moves the position and leaves
  every effect in place; what to do about effects that cannot be repeated is the learner's call.
- In the terminal face Enter runs and `e` pushes.
- The learner can fold the explanations away, leaving only the commands to step through, and unfold
  them again. Stepping works the same in both views.
- The record holds what actually ran, including a command the learner edited, never what the page
  said would run.

### VIII. No Code for Any One Tutorial

- The stepper knows nothing of any one tutorial. Window names, the set-up script and the needs come from
  the tutorial's index file.
- The Singular demo (`lambdasistemi/singular-demo`) is a consumer and the first worked example.
- `paolino/runmd`, which printed selected Markdown blocks as a bash script, is superseded: the stepper
  prints a tutorial's commands as one script.

### IX. The Record Outranks the Code

Documentation, specifications, vision and acceptance outrank implementation, always. Code can be
regenerated from a good record; the record cannot be regenerated from code.

- Every pull request ships its documentation in the same diff.
- Acceptance is written in terms a learner or a tutorial author can see, before the code.
- When scope is cut, the cut lands on the implementation, never on the record.

## Platform and Distribution

- Written in Rust. Each release ships one static binary per platform: Linux x86_64, Linux arm64 and
  macOS on Apple Silicon.
- At run time the stepper needs tmux and the learner's bash or zsh, and nothing else. It uses no network.
- Built with a Nix flake and cached on Cachix. Released as binaries on the release page and as a formula
  in `lambdasistemi/homebrew-tap`.

## Development Workflow

- Every change reaches `main` through a pull request, merged with a merge commit and only on a green CI.
- CI runs on the organisation's NixOS runners, starting with a build gate. Every check job is a required
  check on `main`, and a pull request that adds a job adds it to the required checks.
- Formatting (rustfmt) and lints (clippy) are checks. The conformance suite and an end-to-end test that
  plays the example tutorials unattended in tmux are checks.
- Commit messages follow Conventional Commits; release-please cuts the releases.
- A feature starts as a specification (Spec Kit) whose acceptance is written before the code.

## Governance

- This constitution overrides any other practice in this repository.
- It is amended by a pull request that changes this file, bumps its version and updates the Sync Impact
  Report at the top. Versions follow semantic versioning: MAJOR for removing or redefining a principle,
  MINOR for adding a principle or materially expanding one, PATCH for wording.
- Every plan's Constitution Check names the principles it touches. A plan that departs from a principle
  records the departure, the need for it and the simpler alternative it rejected, in the plan's
  complexity table.

**Version**: 1.0.0 | **Ratified**: 2026-10-22 | **Last Amended**: 2026-10-22
