pub fn fold(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.trim().chars() {
        let mapped = match ch {
            '\u{3000}' => ' ',
            '\u{FF01}'..='\u{FF5E}' => char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch),
            _ => ch,
        };
        for lower in mapped.to_lowercase() {
            out.push(lower);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_case_width_and_space() {
        assert_eq!(fold("  Hello  WORLD "), "hello world");
        assert_eq!(fold("Ｔａｇ１"), "tag1");
        assert_eq!(fold("标签　一"), "标签 一");
    }
}
