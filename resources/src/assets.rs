use std::borrow::Cow;

use crate::qualifier::{QualifierSet, ResourceContext, best_match};

/// Embedded binary data for a static asset.
#[derive(Copy, Clone)]
pub struct BinaryData(pub &'static [u8]);

/// One variant of a resource — a qualifier set paired with a value of type `T`.
pub struct AssetVariant<T> {
    pub qualifiers: QualifierSet,
    pub value: T,
}

// Manual Clone so the impl doesn't require T: Clone in the derive bound.
impl<T: Clone> Clone for AssetVariant<T> {
    fn clone(&self) -> Self {
        AssetVariant {
            qualifiers: self.qualifiers,
            value: self.value.clone(),
        }
    }
}

/// A resource descriptor with a guaranteed default variant.
///
/// `other_variants` uses `Cow<'static, [...]>` so static descriptors can use
/// a borrowed slice while runtime descriptors can use an owned `Vec`.
///
/// ## Usage
///
/// ```ignore
/// // Static (generated):
/// let data: BinaryData = ctx.asset(&assets::icons::CLOSE);
///
/// // Runtime:
/// let desc = AssetDescriptor {
///     default_variant: AssetVariant { qualifiers: QualifierSet::default(), value: my_data },
///     other_variants: Cow::Owned(vec![...]),
/// };
/// ```
pub struct AssetDescriptor<T: Clone + 'static> {
    pub default_variant: AssetVariant<T>,
    pub other_variants: Cow<'static, [AssetVariant<T>]>,
}

impl ResourceContext {
    /// Return the best-matching value for this context, falling back to
    /// `default_variant` if no variant scores.
    pub fn asset<'a, T: Clone + 'static>(&self, desc: &'a AssetDescriptor<T>) -> &'a T {
        let candidates = std::iter::once(&desc.default_variant)
            .chain(desc.other_variants.iter())
            .map(|v| (v.qualifiers, &v.value));
        best_match(candidates, self).unwrap_or(&desc.default_variant.value)
    }
}
