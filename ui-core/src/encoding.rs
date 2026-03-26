/// Convert a UTF-16 code unit offset into a Unicode codepoint index within `s`.
///
/// If `utf16_offset` falls in the middle of a surrogate pair — i.e. it points
/// at the low surrogate of a supplementary character (codepoint > U+FFFF) —
/// the result is rounded up to the codepoint immediately after that character.
/// This strips the invalid first half rather than producing a split codepoint.
///
/// # Examples
/// ```
/// use ui_utils::encoding::utf16_offset_to_codepoint;
/// let s = "A🦀B"; // 🦀 is U+1F980, a surrogate pair in UTF-16
/// assert_eq!(utf16_offset_to_codepoint(s, 0), 0); // before 'A'
/// assert_eq!(utf16_offset_to_codepoint(s, 1), 1); // before 🦀
/// assert_eq!(utf16_offset_to_codepoint(s, 2), 2); // mid-surrogate → rounds up past 🦀
/// assert_eq!(utf16_offset_to_codepoint(s, 3), 2); // after 🦀, before 'B'
/// assert_eq!(utf16_offset_to_codepoint(s, 4), 3); // after 'B'
/// ```
pub fn utf16_offset_to_codepoint(s: &str, utf16_offset: usize) -> usize {
    let mut utf16_count = 0;
    let mut codepoint_index = 0;
    for ch in s.chars() {
        if utf16_count >= utf16_offset {
            break;
        }
        utf16_count += ch.len_utf16();
        codepoint_index += 1;
    }
    codepoint_index
}

/// Convert a Unicode codepoint index into a UTF-16 code unit offset within `s`.
///
/// # Examples
/// ```
/// use ui_utils::encoding::codepoint_to_utf16_offset;
/// let s = "A🦀B";
/// assert_eq!(codepoint_to_utf16_offset(s, 0), 0); // before 'A'
/// assert_eq!(codepoint_to_utf16_offset(s, 1), 1); // before 🦀
/// assert_eq!(codepoint_to_utf16_offset(s, 2), 3); // after 🦀 (2 UTF-16 units)
/// assert_eq!(codepoint_to_utf16_offset(s, 3), 4); // after 'B'
/// ```
pub fn codepoint_to_utf16_offset(s: &str, codepoint_index: usize) -> usize {
    s.chars()
        .take(codepoint_index)
        .map(|ch| ch.len_utf16())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_roundtrip() {
        let s = "hello";
        for i in 0..=5 {
            assert_eq!(utf16_offset_to_codepoint(s, i), i);
            assert_eq!(codepoint_to_utf16_offset(s, i), i);
        }
    }

    #[test]
    fn surrogate_pair_mid_offset_rounds_up() {
        let s = "A🦀B"; // 🦀 occupies UTF-16 units 1 and 2
        // Pointing at the low surrogate (offset 2) must yield codepoint 2,
        // i.e. past the emoji, not between its two halves.
        assert_eq!(utf16_offset_to_codepoint(s, 2), 2);
    }

    #[test]
    fn surrogate_pair_roundtrip() {
        let s = "A🦀B";
        // Codepoint 1 = 🦀 → UTF-16 offset 1
        assert_eq!(codepoint_to_utf16_offset(s, 1), 1);
        // UTF-16 offset 1 → codepoint 1
        assert_eq!(utf16_offset_to_codepoint(s, 1), 1);
        // Codepoint 2 = 'B' → UTF-16 offset 3
        assert_eq!(codepoint_to_utf16_offset(s, 2), 3);
        // UTF-16 offset 3 → codepoint 2
        assert_eq!(utf16_offset_to_codepoint(s, 3), 2);
    }

    #[test]
    fn multibyte_utf8_no_surrogate() {
        // '€' is U+20AC: 3 UTF-8 bytes, 1 UTF-16 unit, 1 codepoint
        let s = "€100";
        assert_eq!(codepoint_to_utf16_offset(s, 0), 0);
        assert_eq!(codepoint_to_utf16_offset(s, 1), 1);
        assert_eq!(utf16_offset_to_codepoint(s, 1), 1);
    }
}
