#!/usr/bin/env python3
"""Refuse a built documentation site that is missing a required page.

The site directory is the output of ``mkdocs build --strict``. The
docs directory is the Markdown the site is built from. This script
reads both. It does not build the site.
"""

import re
import sys
from pathlib import Path

FENCE = re.compile(r"^(`{3,})(.*)$")
KEYS = {"window", "status", "mode", "if"}


def info_problem(info):
    """Return a problem with a fence info string, or None when it is allowed.

    A single language word is an ordinary code block. Attributes are
    ``lang { key=value }``. Anything else is a problem.
    """
    info = info.strip()
    if info == "":
        return None
    if "{" not in info and len(info.split()) == 1:
        return None
    if "{" not in info:
        return "attributes are written as { key=value } after the language, not as bare words"
    if info.count("{") != 1 or info.count("}") != 1 or not info.endswith("}"):
        return "attributes are written as { key=value } after the language"
    language, rest = info.split("{", 1)
    language = language.strip()
    if language == "" or len(language.split()) != 1:
        return "the language is one word before the attributes"
    inside = rest[:-1].strip()
    if inside.startswith("."):
        return "attributes are written as { key=value } after the language, not as a Pandoc class"
    if '"' in inside:
        return "attributes are written as { key=value } after the language, not as a JSON object"
    if inside == "":
        return None
    for token in inside.split():
        if "=" not in token:
            return "each attribute is key=value"
        key, value = token.split("=", 1)
        if key not in KEYS or value == "" or "=" in value:
            return f"attribute {token} is not a format key"
        if key == "mode" and value not in {"show", "observe"}:
            return "mode is show or observe"
        if key == "status" and (not value.isdigit() or int(value) > 255):
            return "status must be a non-negative integer that fits in one byte"
        if key == "if":
            name, sep, setting_value = value.partition(":")
            if sep == "" or name == "" or setting_value == "" or ":" in setting_value:
                return "if is NAME:value"
    return None


def prove_the_parser_rejects_a_bad_spelling():
    """The fence rule has to fail on a spelling the format refuses."""
    samples = {
        "bash { window=bob status=19 }": None,
        "bash { window=bob mode=observe }": None,
        "bash { window=alice if=SIGNING:external }": None,
        "text { window=bob mode=show }": None,
        "bash": None,
        "mermaid": None,
        "bash window=bob": "bare words",
        "bash {.bash}": "Pandoc",
        'bash {"window":"bob"}': "JSON",
        "bash { window=bob mode=maybe }": "mode",
    }
    failures = []
    for info, expected in samples.items():
        problem = info_problem(info)
        if expected is None and problem is not None:
            failures.append(f"rejected a valid fence {info!r}: {problem}")
        if expected is not None and (problem is None or expected not in problem):
            failures.append(f"accepted a bad fence {info!r}")
    return failures


def page(site, name):
    nested = site / name / "index.html"
    if nested.is_file():
        return nested
    flat = site / f"{name}.html"
    return flat


def require_text(text, needle, failures, message):
    if needle not in text:
        failures.append(message)


def fences_in(docs):
    failures = []
    author_attributed = 0
    for path in sorted(docs.rglob("*.md")):
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            match = FENCE.match(line)
            if match is None:
                continue
            info = match.group(2)
            problem = info_problem(info)
            if problem is not None:
                failures.append(f"{path}:{number}: {problem}")
            if path.name == "author.md" and "{ window=" in info:
                author_attributed += 1
    if author_attributed == 0:
        failures.append("the author's guide shows no command fence")
    return failures


def main(argv):
    if len(argv) != 3:
        print("usage: check_site.py SITE DOCS WORKFLOW", file=sys.stderr)
        return 2
    site = Path(argv[0])
    docs = Path(argv[1])
    workflow = Path(argv[2])
    failures = prove_the_parser_rejects_a_bad_spelling()
    index = site / "index.html"
    if not index.is_file():
        failures.append(f"missing {index}")
    else:
        text = index.read_text(encoding="utf-8", errors="replace")
        require_text(text, 'id="tutorial-palette-toggle"', failures, "front page has no palette switch")
        require_text(text, "tutorial-dark-palette", failures, "front page does not load the dark palette")
        require_text(text, "A learner", failures, "front page does not tell the learner's story")
        require_text(text, "A tutorial author", failures, "front page does not tell the author's story")
        if "mermaid" not in text or "Shells in tmux" not in text:
            failures.append("front page has no context diagram")
    for name, needle, message in (
        ("format", "tutorial.toml", "the format specification is missing"),
        ("commands", "tutorial-stepper check", "the command reference does not describe check"),
        ("author", "play --auto", "the author's guide does not play the tutorial unattended"),
    ):
        built = page(site, name)
        if not built.is_file():
            failures.append(f"missing {built}")
            continue
        body = built.read_text(encoding="utf-8", errors="replace")
        require_text(body, needle, failures, message)
    author = page(site, "author")
    if author.is_file():
        body = author.read_text(encoding="utf-8", errors="replace")
        require_text(body, "{ window=bob }", failures, "the worked example is not a command fence")
        require_text(body, "echo hi", failures, "the worked example has no command")
        if "<code" not in body:
            failures.append("the worked example did not render as a code block")
    if docs.is_dir():
        failures.extend(fences_in(docs))
    else:
        failures.append(f"missing {docs}")
    library = page(site, "library")
    if not library.is_file():
        failures.append(f"missing {library}")
    else:
        body = library.read_text(encoding="utf-8", errors="replace")
        require_text(body, "<iframe", failures, "the library page does not embed the reference")
        require_text(
            body,
            "api/tutorial_stepper/index.html",
            failures,
            "the library page does not point at the generated reference",
        )
    rustdoc = site / "api" / "tutorial_stepper" / "index.html"
    if not rustdoc.is_file():
        failures.append(f"missing {rustdoc}")
    else:
        reference = rustdoc.read_text(encoding="utf-8", errors="replace")
        require_text(reference, "fn.run.html", failures, "the generated reference does not describe run")
        require_text(reference, "fence/index.html", failures, "the generated reference does not describe fence attributes")
    if not workflow.is_file():
        failures.append(f"missing {workflow}")
    else:
        text = workflow.read_text(encoding="utf-8")
        require_text(text, "actions/upload-pages-artifact@", failures, "the workflow does not upload the site")
        require_text(text, "actions/deploy-pages@", failures, "the workflow does not deploy the site")
        require_text(text, "refs/heads/main", failures, "the workflow does not deploy from main")
    if failures:
        for failure in failures:
            print(failure, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
