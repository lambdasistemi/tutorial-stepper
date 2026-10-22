//! Attributes written in a code fence's info string.

/// How a block runs, when the fence says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The block is shown and not sent to a shell.
    Show,
    /// The block is an observation.
    Observe,
}

/// One fence that carries attributes.
#[derive(Debug, PartialEq, Eq)]
pub struct AttributedFence {
    /// 1-based line of the opening fence.
    pub line: usize,
    /// The language word. Empty when the fence did not name one.
    pub language: String,
    /// `window`, when the fence sets it.
    pub window: Option<String>,
    /// `status`, when the fence sets it.
    pub status: Option<u8>,
    /// `mode`, when the fence sets it.
    pub mode: Option<Mode>,
    /// `if`, the setting name and the value, when the fence sets it.
    pub setting: Option<(String, String)>,
    /// The fence body, without the opening or closing fence.
    pub body: String,
}

/// A fence whose info string is not `lang { key=value }`.
#[derive(Debug, PartialEq, Eq)]
pub struct FenceError {
    /// 1-based line of the opening fence.
    pub line: usize,
    /// What is wrong, without the file name.
    pub message: String,
}

/// The attributed fences in a CommonMark page, in order, and every fence that
/// is not an ordinary code block and not `lang { key=value }`.
///
/// A fence with only a language is left out. A later fence is still read when
/// an earlier one is wrong.
pub fn fence_attributes(markdown: &str) -> (Vec<AttributedFence>, Vec<FenceError>) {
    let parser = pulldown_cmark::Parser::new(markdown);
    let mut fences = Vec::new();
    let mut errors = Vec::new();
    let mut open: Option<AttributedFence> = None;
    for (event, range) in parser.into_offset_iter() {
        match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(kind)) => {
                let pulldown_cmark::CodeBlockKind::Fenced(info) = kind else {
                    continue;
                };
                let line = line_number(markdown, range.start);
                open = match parse_info(&info, line) {
                    Ok(fence) => fence,
                    Err(error) => {
                        errors.push(error);
                        None
                    }
                };
            }
            pulldown_cmark::Event::Text(text) => {
                if let Some(fence) = open.as_mut() {
                    fence.body.push_str(&text);
                }
            }
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
                if let Some(fence) = open.take() {
                    fences.push(fence);
                }
            }
            _ => {}
        }
    }
    (fences, errors)
}

fn line_number(markdown: &str, offset: usize) -> usize {
    markdown
        .get(..offset)
        .map_or(1, |prefix| prefix.bytes().filter(|byte| *byte == b'\n').count() + 1)
}

fn parse_info(info: &str, line: usize) -> Result<Option<AttributedFence>, FenceError> {
    let info = info.trim();
    if info.is_empty() {
        return Ok(None);
    }
    let Some(brace) = info.find('{') else {
        if info.split_whitespace().nth(1).is_some() {
            return Err(malformed(line));
        }
        return Ok(None);
    };
    let language = info[..brace].trim();
    if language.split_whitespace().nth(1).is_some() {
        return Err(malformed(line));
    }
    let rest = info[brace..].trim();
    let Some(inside) = rest.strip_prefix('{').and_then(|body| body.strip_suffix('}')) else {
        return Err(malformed(line));
    };
    if rest[1..].contains('{') {
        return Err(malformed(line));
    }
    let inside = inside.trim();
    if inside.starts_with('.') {
        return Err(FenceError {
            line,
            message:
                "attributes are written as { key=value } after the language, not as a Pandoc class"
                    .to_string(),
        });
    }
    if inside.contains('"') {
        return Err(FenceError {
            line,
            message:
                "attributes are written as { key=value } after the language, not as a JSON object"
                    .to_string(),
        });
    }
    let mut fence = AttributedFence {
        line,
        language: language.to_string(),
        window: None,
        status: None,
        mode: None,
        setting: None,
        body: String::new(),
    };
    if inside.is_empty() {
        return Ok(Some(fence));
    }
    for token in inside.split_whitespace() {
        let Some((key, value)) = token.split_once('=') else {
            return Err(malformed(line));
        };
        if value.is_empty() || value.contains('=') {
            return Err(malformed(line));
        }
        apply(&mut fence, key, value)?;
    }
    Ok(Some(fence))
}

fn apply(fence: &mut AttributedFence, key: &str, value: &str) -> Result<(), FenceError> {
    match key {
        "window" => fence.window = Some(word(fence.line, "window", value)?.to_string()),
        "status" => {
            fence.status = Some(value.parse::<u8>().map_err(|_| FenceError {
                line: fence.line,
                message: "status must be a non-negative integer".to_string(),
            })?);
        }
        "mode" => {
            fence.mode = Some(match value {
                "show" => Mode::Show,
                "observe" => Mode::Observe,
                _ => {
                    return Err(FenceError {
                        line: fence.line,
                        message: "mode must be show or observe".to_string(),
                    });
                }
            });
        }
        "if" => {
            let Some((name, setting_value)) = value.split_once(':') else {
                return Err(FenceError {
                    line: fence.line,
                    message: "if must be written as NAME:value".to_string(),
                });
            };
            if name.is_empty()
                || setting_value.is_empty()
                || setting_value.contains(':')
                || !is_word(name)
                || !is_word(setting_value)
            {
                return Err(FenceError {
                    line: fence.line,
                    message: "if must be written as NAME:value".to_string(),
                });
            }
            fence.setting = Some((name.to_string(), setting_value.to_string()));
        }
        other => {
            return Err(FenceError {
                line: fence.line,
                message: format!("unknown attribute \"{other}\""),
            });
        }
    }
    Ok(())
}

fn word<'a>(line: usize, key: &str, value: &'a str) -> Result<&'a str, FenceError> {
    if is_word(value) {
        Ok(value)
    } else {
        Err(FenceError { line, message: format!("{key} must be a single word") })
    }
}

pub(crate) fn is_word(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn malformed(line: usize) -> FenceError {
    FenceError {
        line,
        message: "attributes are written as { key=value } after the language".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{fence_attributes, FenceError, Mode};

    fn only_error(markdown: &str) -> FenceError {
        let (fences, errors) = fence_attributes(markdown);
        assert!(fences.is_empty(), "{fences:?}");
        assert_eq!(errors.len(), 1, "{errors:?}");
        errors.into_iter().next().expect("the assertion above holds")
    }

    #[test]
    fn reads_attributes_and_skips_an_ordinary_block() {
        let markdown = "\
```bash
echo plain
```

```bash { window=bob status=19 mode=observe if=SIGNING:external }
echo hi
```
";
        let (fences, errors) = fence_attributes(markdown);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(fences.len(), 1, "fences: {fences:?}");
        let fence = &fences[0];
        assert_eq!(fence.line, 5);
        assert_eq!(fence.language, "bash");
        assert_eq!(fence.window.as_deref(), Some("bob"));
        assert_eq!(fence.status, Some(19));
        assert_eq!(fence.mode, Some(Mode::Observe));
        assert_eq!(
            fence.setting.as_ref().map(|(name, value)| (name.as_str(), value.as_str())),
            Some(("SIGNING", "external"))
        );
    }

    #[test]
    fn reads_show_mode() {
        let markdown = "```text { mode=show }\nshown\n```\n";
        let (fences, errors) = fence_attributes(markdown);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(fences.len(), 1);
        assert_eq!(fences[0].mode, Some(Mode::Show));
        assert_eq!(fences[0].language, "text");
        assert_eq!(fences[0].line, 1);
    }

    #[test]
    fn rejects_a_json_object_at_its_line() {
        let markdown = "```bash {\"window\":\"bob\"}\ncmd\n```\n";
        let err = only_error(markdown);
        assert_eq!(err.line, 1);
        assert!(err.message.contains("JSON"), "{err:?}");
    }

    #[test]
    fn rejects_bare_words_at_their_line() {
        let markdown = "```bash window=bob\ncmd\n```\n";
        let err = only_error(markdown);
        assert_eq!(err.line, 1);
        assert!(err.message.contains("key=value"), "{err:?}");
    }

    #[test]
    fn rejects_pandoc_braces_at_their_line() {
        let markdown = "```{.bash window=bob}\ncmd\n```\n";
        let err = only_error(markdown);
        assert_eq!(err.line, 1);
        assert!(err.message.contains("Pandoc"), "{err:?}");
    }

    #[test]
    fn rejects_an_unknown_attribute() {
        let markdown = "```bash { window=bob colour=red }\ncmd\n```\n";
        let err = only_error(markdown);
        assert_eq!(err.line, 1);
        assert!(err.message.contains("colour"), "{err:?}");
    }

    #[test]
    fn rejects_a_status_that_is_not_an_integer() {
        let markdown = "```bash { status=no }\ncmd\n```\n";
        let err = only_error(markdown);
        assert_eq!(err.line, 1);
        assert!(err.message.contains("status"), "{err:?}");
    }

    #[test]
    fn rejects_a_mode_outside_the_format() {
        let markdown = "```bash { mode=run }\ncmd\n```\n";
        let err = only_error(markdown);
        assert_eq!(err.line, 1);
        assert!(err.message.contains("mode"), "{err:?}");
    }
}
