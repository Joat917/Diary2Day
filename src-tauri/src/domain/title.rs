const TITLE_LIMIT: usize = 100;
const PREVIEW_LIMIT: usize = 300;

pub fn derive_title(content: &str) -> String {
    for raw in content.lines() {
        let line = strip_markers(raw);
        if !line.is_empty() {
            return truncate_chars(&line, TITLE_LIMIT);
        }
    }
    String::new()
}

pub fn preview(content: &str) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    for ch in content.chars() {
        if ch.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(ch);
        if out.chars().count() >= PREVIEW_LIMIT * 2 {
            break;
        }
    }
    truncate_chars(&out, PREVIEW_LIMIT)
}

pub fn preview_around(content: &str, needle: &str) -> String {
    if needle.is_empty() {
        return preview(content);
    }
    let lower = content.to_lowercase();
    let lower_needle = needle.to_lowercase();
    let Some(byte_pos) = lower.find(&lower_needle) else {
        return preview(content);
    };
    let char_pos = content[..byte_pos].chars().count();
    let chs: Vec<char> = content.chars().collect();
    let half = PREVIEW_LIMIT / 2;
    let start = char_pos.saturating_sub(half);
    let end = (start + PREVIEW_LIMIT).min(chs.len());
    let mut snippet: String = chs[start..end].iter().collect();
    snippet = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
    if start > 0 {
        snippet.insert(0, '…');
    }
    if end < chs.len() {
        snippet.push('…');
    }
    snippet
}

fn strip_markers(line: &str) -> String {
    let mut text = line.trim().to_string();
    loop {
        let before = text.len();
        text = text
            .trim_start_matches(|c: char| {
                c == '#' || c == '>' || c == '-' || c == '*' || c == '+' || c.is_whitespace()
            })
            .to_string();
        if let Some(rest) = strip_ordered_marker(&text) {
            text = rest;
        }
        if text.len() == before {
            break;
        }
    }
    text.trim().to_string()
}

fn strip_ordered_marker(text: &str) -> Option<String> {
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = &text[digits.len()..];
    rest.strip_prefix(". ")
        .map(|tail| tail.trim_start().to_string())
}

fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    text.chars().take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_first_content_line() {
        assert_eq!(derive_title("# 标题\n\n正文"), "标题");
        assert_eq!(derive_title("\n\n- 第一条\n第二条"), "第一条");
        assert_eq!(derive_title("   "), "");
    }

    #[test]
    fn truncates_long_title() {
        let long = "字".repeat(120);
        assert_eq!(derive_title(&long).chars().count(), 100);
    }

    #[test]
    fn preview_folds_whitespace() {
        assert_eq!(preview("a\n\nb   c"), "a b c");
    }
}
