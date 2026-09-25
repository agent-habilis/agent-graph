/// A `## Heading` section: the 1-based line of its heading and its lines,
/// each with its 1-based line number.
#[derive(Debug)]
pub(crate) struct Section<'a> {
    pub(crate) line: usize,
    pub(crate) lines: Vec<(usize, &'a str)>,
}

/// The section under `heading` (for example `## Edges`), up to the next
/// heading of the same or a higher level. A heading inside a code fence
/// is text, not a heading.
pub(crate) fn section<'a>(content: &'a str, heading: &str) -> Option<Section<'a>> {
    let level = heading_level(heading)?;
    let mut in_fence = false;
    let mut found: Option<Section<'a>> = None;
    for (index, line) in content.lines().enumerate() {
        let fence = is_fence(line);
        if !in_fence
            && !fence
            && let Some(line_level) = heading_level(line)
        {
            if found.is_some() && line_level <= level {
                break;
            }
            if found.is_none() && line.trim_end() == heading {
                found = Some(Section {
                    line: index + 1,
                    lines: Vec::new(),
                });
                continue;
            }
        }
        if fence {
            in_fence = !in_fence;
        }
        if let Some(section) = &mut found {
            section.lines.push((index + 1, line));
        }
    }
    found
}

pub(crate) fn is_fence(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

pub(crate) fn heading_level(line: &str) -> Option<usize> {
    let level = line.len() - line.trim_start_matches('#').len();
    (level > 0 && line[level..].starts_with(' ')).then_some(level)
}

/// Check the `## Boundaries` section, if there is one. It must hold one or
/// more `- ` items and nothing else. An indented line continues the item
/// above it. The error is a line number and a reason for the user.
pub(crate) fn check_boundaries(content: &str) -> Result<(), (usize, String)> {
    let Some(section) = section(content, "## Boundaries") else {
        return Ok(());
    };
    let mut items = 0;
    for (number, line) in section.lines {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with("- ") {
            items += 1;
        } else if items == 0 || !line.starts_with(char::is_whitespace) {
            return Err((
                number,
                "Boundaries must hold only `- ` list items".to_string(),
            ));
        }
    }
    if items == 0 {
        return Err((section.line, "Boundaries has no `- ` items".to_string()));
    }
    Ok(())
}
