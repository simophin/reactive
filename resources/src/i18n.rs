use std::marker::PhantomData;

use crate::assets::AssetDescriptor;
use crate::qualifier::ResourceContext;

/// The value stored in each variant of a translation descriptor.
///
/// Carries the resolved template string plus a phantom `M` that enforces
/// at compile time that only the matching [`Message`] type can be used for
/// substitution.
pub struct TranslationData<M> {
    pub value: &'static str,
    _phantom: PhantomData<fn(&M)>,
}

impl<M> TranslationData<M> {
    pub const fn new(value: &'static str) -> Self {
        Self {
            value,
            _phantom: PhantomData,
        }
    }
}

// Manual Copy/Clone so the impls don't require M: Copy.
impl<M> Copy for TranslationData<M> {}
impl<M> Clone for TranslationData<M> {
    fn clone(&self) -> Self {
        *self
    }
}

// ---------------------------------------------------------------------------
// Resolution on ResourceContext
// ---------------------------------------------------------------------------

impl ResourceContext {
    /// Pick the best-matching template string from `desc` for this context.
    pub fn resolve_translation<M: Message>(
        &self,
        desc: &AssetDescriptor<TranslationData<M>>,
    ) -> &'static str {
        self.asset(desc).value
    }

    /// Resolve the template for `desc` then apply `msg`'s parameter substitution.
    pub fn translate<M: Message>(
        &self,
        desc: &AssetDescriptor<TranslationData<M>>,
        msg: &M,
    ) -> String {
        msg.apply(self.resolve_translation(desc))
    }
}

// ---------------------------------------------------------------------------
// Type-safe message API
// ---------------------------------------------------------------------------

/// A type-safe i18n message that substitutes its parameters into a resolved
/// template string.
pub trait Message {
    fn apply(&self, template: &str) -> String;
}

impl Message for () {
    fn apply(&self, template: &str) -> String {
        template.to_owned()
    }
}

/// Replace occurrences of `{ $name }` (with any internal whitespace) in
/// `template` with `value`.
///
/// Used by generated [`Message`] implementations.
pub fn replace_param(template: &str, name: &str, value: &str) -> String {
    let needle = format!("${name}");
    let mut result = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        result.push_str(&rest[..open]);
        rest = &rest[open + 1..];
        if let Some(close) = rest.find('}') {
            let inner = rest[..close].trim();
            if inner == needle {
                result.push_str(value);
            } else {
                result.push('{');
                result.push_str(&rest[..close]);
                result.push('}');
            }
            rest = &rest[close + 1..];
        } else {
            result.push('{');
        }
    }
    result.push_str(rest);
    result
}
