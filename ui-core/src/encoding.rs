/// Convert a UTF-16 code unit offset into a Unicode codepoint index within `s`.
///
/// If `utf16_offset` falls in the middle of a surrogate pair — i.e. it points
/// at the low surrogate of a supplementary character (codepoint > U+FFFF) —
/// the result is rounded up to the codepoint immediately after that character.
/// This strips the invalid first half rather than producing a split codepoint.
///
/// # Examples
/// ```
/// use ui_core::encoding::utf16_offset_to_codepoint;
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
/// use ui_core::encoding::codepoint_to_utf16_offset;
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

/// Convert a UTF-16 code unit offset into a byte offset within `s`, rounding a
/// mid-surrogate offset up past the character like [`utf16_offset_to_codepoint`].
pub fn utf16_offset_to_byte(s: &str, utf16_offset: usize) -> usize {
    let mut utf16_count = 0;
    for (byte_index, ch) in s.char_indices() {
        if utf16_count >= utf16_offset {
            return byte_index;
        }
        utf16_count += ch.len_utf16();
    }
    s.len()
}

/// A UTF-8 string whose offsets and length are measured in UTF-16 code units, matching
/// Java/Kotlin strings (used by the Android backend).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Utf16String(pub String);

impl std::fmt::Display for Utf16String {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Utf16String {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl crate::widgets::PlatformTextType for Utf16String {
    type RefType<'a> = &'a str;

    fn len(&self) -> usize {
        self.0.encode_utf16().count()
    }

    fn replace(&self, range: std::ops::Range<usize>, with: &&str) -> Self {
        let start = utf16_offset_to_byte(&self.0, range.start);
        let end = utf16_offset_to_byte(&self.0, range.end).max(start);
        let mut s = self.0.clone();
        s.replace_range(start..end, with);
        Self(s)
    }

    fn as_str(&self) -> Option<&str> {
        Some(&self.0)
    }
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

    #[test]
    fn utf16_string_uses_utf16_offsets() {
        use crate::widgets::PlatformTextType;

        let text = Utf16String::from("aé🦀z");
        assert_eq!(text.len(), 5);
        assert_eq!(text.replace(1..4, &"X").0, "aXz");
        assert_eq!(text.replace(5..5, &"!").0, "aé🦀z!");
        assert_eq!(utf16_offset_to_byte("A🦀B", 2), 5);
    }
}
