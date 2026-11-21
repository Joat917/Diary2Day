pub fn count_words(content: &str) -> i64 {
    let mut total: i64 = 0;
    let mut in_code = false;
    let mut in_word = false;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code = !in_code;
            in_word = false;
            continue;
        }
        if in_code {
            continue;
        }
        if trimmed.starts_with('|') || trimmed.starts_with("---") {
            continue;
        }
        for ch in line.chars() {
            if is_cjk(ch) {
                total += 1;
                in_word = false;
            } else if ch.is_alphanumeric() || ch == '\'' || ch == '-' {
                if !in_word {
                    total += 1;
                    in_word = true;
                }
            } else {
                in_word = false;
            }
        }
        in_word = false;
    }
    total
}

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32,
        0x2E80..=0x2EFF
        | 0x3040..=0x30FF
        | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xF900..=0xFAFF
        | 0x20000..=0x2FA1F)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_cjk_by_char_and_latin_by_word() {
        assert_eq!(count_words("今天很好"), 4);
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("今天 hello 世界"), 5);
    }

    #[test]
    fn skips_code_and_tables() {
        assert_eq!(count_words("```\nlet a = 1;\n```\n正文"), 2);
        assert_eq!(count_words("| a | b |\n|---|---|\n| 1 | 2 |"), 0);
    }
}
