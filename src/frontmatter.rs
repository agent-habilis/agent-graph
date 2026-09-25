#[derive(Debug)]
pub(crate) struct Frontmatter<'a> {
    fields: Vec<(&'a str, &'a str)>,
    pub(crate) body: &'a str,
}

impl Frontmatter<'_> {
    /// The value of a top-level `key: value` line, without surrounding quotes.
    pub(crate) fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(field, _)| *field == key)
            .map(|(_, value)| unquote(value))
    }
}

/// Parse per the OKF rule: line 1 is exactly `---`, a closing `---` follows,
/// and the block holds a `type:` key. Anything else (a leading horizontal
/// rule, say) is not frontmatter and yields `None`.
pub(crate) fn parse(content: &str) -> Option<Frontmatter<'_>> {
    let mut offset = 0;
    let mut fields = Vec::new();
    for (index, raw) in content.split_inclusive('\n').enumerate() {
        offset += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        if index == 0 {
            if line != "---" {
                return None;
            }
            continue;
        }
        if line == "---" {
            let has_type = fields.iter().any(|(key, _)| *key == "type");
            return has_type.then(|| Frontmatter {
                fields,
                body: &content[offset..],
            });
        }
        if line.starts_with(char::is_whitespace) {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            fields.push((key.trim(), value.trim()));
        }
    }
    None
}

/// The text after the frontmatter, or all of `content` if it has none.
pub(crate) fn body(content: &str) -> &str {
    parse(content).map_or(content, |frontmatter| frontmatter.body)
}

/// Items of an inline YAML list (`[a, "b"]`), or `None` for any other form.
pub(crate) fn inline_list(value: &str) -> Option<Vec<String>> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        inner
            .split(',')
            .map(|item| unquote(item.trim()).to_string())
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}
