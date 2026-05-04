use std::ops::Range;

use objc2::rc::Retained;
use objc2_foundation::NSString;

/// The source-of-truth state for an editable text view on Apple platforms.
///
/// `text` is a `Retained<NSString>` — immutable and ref-counted, so cloning
/// is a cheap retain-count bump with no allocation or re-encoding.
///
/// `selection` is a `Range<usize>` of **UTF-16 code unit** offsets — the
/// native unit of `NSRange` — so it maps directly to and from the platform
/// API with no conversion.
///
/// For cross-platform code that needs to convert between UTF-16 code units
/// and Unicode codepoints, see `ui_utils::encoding`.
#[derive(Clone, Debug)]
pub struct TextInputState {
    pub text: Retained<NSString>,
    pub selection: Range<usize>,
}

impl TextInputState {
    pub fn new(text: impl AsRef<str>) -> Self {
        let ns = NSString::from_str(text.as_ref());
        let len = ns.length();
        Self {
            text: ns,
            selection: len..len,
        }
    }
}

impl PartialEq for TextInputState {
    fn eq(&self, other: &Self) -> bool {
        self.selection == other.selection && self.text.isEqualToString(&other.text)
    }
}
