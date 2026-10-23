//! The tutorial page as text for a terminal pane.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// Render `markdown` wrapped to `width` columns.
///
/// `highlight` is the 1-based line of the opening fence of the next command.
/// The rendered text uses ANSI for bold, inline code and links. The next
/// command's block is preceded by the word `next`.
pub(crate) fn render(markdown: &str, width: usize, highlight: Option<usize>) -> String {
    let width = width.max(1);
    let mut blocks = Vec::new();
    let mut text = String::new();
    let mut strong = false;
    let mut heading = false;
    let mut list = 0_usize;
    let mut item = false;
    let mut link: Option<String> = None;
    let mut code: Option<Code> = None;

    for (event, range) in Parser::new_ext(markdown, Options::empty()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                flush_text(&mut blocks, &mut text, heading, list, item);
                heading = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                flush_text(&mut blocks, &mut text, heading, list, item);
                heading = false;
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush_text(&mut blocks, &mut text, heading, list, item);
                item = false;
            }
            Event::Start(Tag::List(_)) => list += 1,
            Event::End(TagEnd::List(_)) => list = list.saturating_sub(1),
            Event::Start(Tag::Item) => item = true,
            Event::End(TagEnd::Item) => {
                flush_text(&mut blocks, &mut text, heading, list, item);
                item = false;
            }
            Event::Start(Tag::Strong) | Event::Start(Tag::Emphasis) => strong = true,
            Event::End(TagEnd::Strong) | Event::End(TagEnd::Emphasis) => strong = false,
            Event::Start(Tag::Link { dest_url, .. }) => link = Some(dest_url.to_string()),
            Event::End(TagEnd::Link) => {
                if let Some(url) = link.take() {
                    text.push(' ');
                    text.push_str(&paint(&url, false, false, true));
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_text(&mut blocks, &mut text, heading, list, item);
                let info = match kind {
                    CodeBlockKind::Fenced(info) => info.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some(Code {
                    line: line_number(markdown, range.start),
                    info,
                    body: String::new(),
                });
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(code) = code.take() {
                    blocks.push(Block::Code(code));
                }
            }
            Event::Text(body) => {
                if let Some(code) = code.as_mut() {
                    code.body.push_str(&body);
                } else {
                    text.push_str(&paint(&body, strong || heading, false, false));
                }
            }
            Event::Code(body) => text.push_str(&paint(&body, strong || heading, true, false)),
            Event::SoftBreak => text.push(' '),
            Event::HardBreak => text.push('\n'),
            Event::Rule => {
                flush_text(&mut blocks, &mut text, heading, list, item);
                blocks.push(Block::Plain("——".to_string()));
            }
            _ => {}
        }
    }
    flush_text(&mut blocks, &mut text, heading, list, item);

    let mut out = String::new();
    for block in blocks {
        let piece = match block {
            Block::Plain(line) => wrap_styled(&line, width, "", ""),
            Block::Item(line) => wrap_styled(&line, width, "• ", "  "),
            Block::Code(code) => render_code(&code, width, highlight),
        };
        if piece.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&piece);
    }
    out
}

struct Code {
    line: usize,
    info: String,
    body: String,
}

enum Block {
    Plain(String),
    Item(String),
    Code(Code),
}

fn flush_text(blocks: &mut Vec<Block>, text: &mut String, heading: bool, list: usize, item: bool) {
    let text = std::mem::take(text);
    if text.trim().is_empty() {
        return;
    }
    if item || list > 0 && !heading {
        blocks.push(Block::Item(text));
    } else {
        blocks.push(Block::Plain(text));
    }
}

fn render_code(code: &Code, width: usize, highlight: Option<usize>) -> String {
    let mut out = String::new();
    if highlight == Some(code.line) {
        out.push_str(&paint("next", true, false, false));
    } else if code.info.contains("mode=show") {
        out.push_str(&paint("shown only", false, false, true));
    } else if code.info.contains("mode=observe") {
        out.push_str(&paint("observation", false, false, true));
    }
    let body = code.body.trim_end_matches('\n');
    if body.is_empty() && out.is_empty() {
        return String::new();
    }
    for line in body.split('\n') {
        if !out.is_empty() {
            out.push('\n');
        }
        let painted = paint(line, false, true, false);
        out.push_str(&wrap_styled(&painted, width, "│ ", "│ "));
    }
    out
}

fn line_number(markdown: &str, offset: usize) -> usize {
    markdown[..offset.min(markdown.len())].bytes().filter(|byte| *byte == b'\n').count() + 1
}

fn paint(text: &str, bold: bool, code: bool, dim: bool) -> String {
    if text.is_empty() || (!bold && !code && !dim) {
        return text.to_string();
    }
    let mut out = String::new();
    if bold {
        out.push_str("\u{1b}[1m");
    }
    if dim {
        out.push_str("\u{1b}[2m");
    }
    if code {
        out.push_str("\u{1b}[36m");
    }
    out.push_str(text);
    if code {
        out.push_str("\u{1b}[39m");
    }
    if bold || dim {
        out.push_str("\u{1b}[22m");
    }
    out
}

enum Tok {
    Ansi(String),
    Char(char),
}

fn tokenize(text: &str) -> Vec<Tok> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '\u{1b}' && chars.get(index + 1) == Some(&'[') {
            let start = index;
            index += 2;
            while index < chars.len() && chars[index] != 'm' {
                index += 1;
            }
            if index < chars.len() {
                index += 1;
            }
            tokens.push(Tok::Ansi(chars[start..index].iter().collect()));
            continue;
        }
        tokens.push(Tok::Char(chars[index]));
        index += 1;
    }
    tokens
}

fn visible_width(text: &str) -> usize {
    tokenize(text).iter().filter(|token| matches!(token, Tok::Char(_))).count()
}

struct Word {
    text: String,
    cols: usize,
}

/// Wrap styled text on spaces. ANSI sequences take no columns.
fn wrap_styled(text: &str, width: usize, prefix: &str, cont: &str) -> String {
    let width = width.max(1);
    let mut words = Vec::new();
    let mut word = Word { text: String::new(), cols: 0 };
    let mut pending_break = false;
    let flush_word = |words: &mut Vec<(Word, bool)>, word: &mut Word, hard: bool| {
        if word.cols == 0 && word.text.is_empty() {
            return;
        }
        words.push((std::mem::replace(word, Word { text: String::new(), cols: 0 }), hard));
    };
    for token in tokenize(text) {
        match token {
            Tok::Ansi(ansi) => word.text.push_str(&ansi),
            Tok::Char('\n') => {
                flush_word(&mut words, &mut word, true);
                pending_break = true;
            }
            Tok::Char(' ') => {
                flush_word(&mut words, &mut word, false);
                pending_break = false;
            }
            Tok::Char(ch) => {
                if pending_break {
                    flush_word(&mut words, &mut word, true);
                    pending_break = false;
                }
                word.text.push(ch);
                word.cols += 1;
            }
        }
    }
    flush_word(&mut words, &mut word, false);

    let mut lines = Vec::new();
    let mut line = prefix.to_string();
    let mut cols = visible_width(prefix);
    let prefix_cols = cols;
    let cont_cols = visible_width(cont);
    for (word, hard) in words {
        let gap = usize::from(cols > prefix_cols);
        if cols > prefix_cols && cols + gap + word.cols > width {
            lines.push(std::mem::take(&mut line));
            line = cont.to_string();
            cols = cont_cols;
        }
        if cols > cont_cols && cols > prefix_cols {
            line.push(' ');
            cols += 1;
        }
        line.push_str(&word.text);
        cols += word.cols;
        if hard {
            lines.push(std::mem::take(&mut line));
            line = cont.to_string();
            cols = cont_cols;
        }
    }
    if cols > prefix_cols || lines.is_empty() {
        lines.push(line);
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{render, visible_width};

    fn plain(text: &str) -> String {
        let mut out = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut index = 0;
        while index < chars.len() {
            if chars[index] == '\u{1b}' && chars.get(index + 1) == Some(&'[') {
                index += 2;
                while index < chars.len() && chars[index] != 'm' {
                    index += 1;
                }
                index += 1;
                continue;
            }
            out.push(chars[index]);
            index += 1;
        }
        out
    }

    #[test]
    fn the_page_shows_markdown_wrapped_and_marks_the_next_command() {
        let markdown = "\
# Title

A **bold** word, `code`, and a [name](https://example.com).

- alpha
- beta

```bash { window=bob }
echo hi
```
";
        let fence = markdown.lines().position(|line| line.starts_with("```")).unwrap() + 1;
        let text = render(markdown, 40, Some(fence));
        let shown = plain(&text);
        assert!(shown.contains("Title"), "{shown}");
        assert!(text.contains("\u{1b}[1mTitle") || text.contains("\u{1b}[1m"), "{text:?}");
        assert!(shown.contains("bold"), "{shown}");
        assert!(!shown.contains("**bold**"), "{shown}");
        assert!(text.contains("\u{1b}[36mcode\u{1b}[39m"), "{text:?}");
        assert!(shown.contains("name"), "{shown}");
        assert!(shown.contains("https://example.com"), "{text:?}");
        assert!(text.contains("\u{1b}[2mhttps://example.com"), "{text:?}");
        assert!(shown.contains("• alpha"), "{shown}");
        assert!(shown.contains("• beta"), "{shown}");
        assert!(shown.contains("next"), "{shown}");
        assert!(shown.contains("echo hi"), "{shown}");
        assert!(text.contains("\u{1b}[1mnext"), "{text:?}");
        for line in shown.lines() {
            assert!(line.chars().count() <= 40, "line wider than the pane: {line:?}");
        }
    }

    #[test]
    fn wrapping_breaks_on_words_and_ignores_ansi_width() {
        let markdown = "one two three four five six seven eight nine ten\n";
        let text = render(markdown, 12, None);
        let shown = plain(&text);
        assert!(shown.contains('\n'), "the paragraph did not wrap:\n{shown}");
        for line in shown.lines() {
            assert!(line.chars().count() <= 12, "line wider than 12: {line:?}");
        }
        assert!(shown
            .replace('\n', " ")
            .split_whitespace()
            .eq(["one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"]));
        let bold = render("**abcdefghij** klmnopqrstuv\n", 12, None);
        for line in plain(&bold).lines() {
            assert!(visible_width(line) <= 12, "styled line wider than 12: {line:?} in {bold:?}");
        }
    }

    #[test]
    fn a_shown_block_and_an_observation_are_labelled_and_not_marked() {
        let markdown = "\
```text { mode=show }
shown
```

```bash { mode=observe }
echo observed
```
";
        let text = plain(&render(markdown, 40, None));
        assert!(text.contains("shown only"), "{text}");
        assert!(text.contains("observation"), "{text}");
        assert!(!text.contains("next"), "{text}");
    }
}
