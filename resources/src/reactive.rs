//! Optional reactive integration for the `resources` crate.
//!
//! Enabled with the `reactive` feature.

use reactive_core::{ContextKey, ReadSignal, SetupContext, Signal, StoredSignal};

use crate::{AssetDescriptor, BinaryData, Message, ResourceContext, TranslationData};

static RESOURCE_CTX_KEY: ContextKey<ResourceContext> = ContextKey::new();

/// Handle to a [`ResourceContext`] signal in the component tree.
///
/// Returned by [`use_resource_context`].  Methods mirror [`ResourceContext`]'s
/// own resolve/format API but produce reactive memos instead of plain values.
pub struct ReactiveResourceContext(Option<ReadSignal<ResourceContext>>);

impl ReactiveResourceContext {
    /// Memo: resolves a translation template reactively (no parameters).
    pub fn resolve_translation<M: Message + 'static>(
        &self,
        ctx: &mut SetupContext,
        desc: &'static AssetDescriptor<TranslationData<M>>,
    ) -> ReadSignal<&'static str> {
        let signal = self.0.clone();
        ctx.create_memo(move || {
            let rc = signal.as_ref().map(|s| s.read()).unwrap_or_default();
            rc.resolve_translation(desc)
        })
    }

    /// Memo: resolves and formats a translation with a fixed message (no
    /// reactive parameters).
    pub fn translate<M: Message + 'static>(
        &self,
        ctx: &mut SetupContext,
        desc: &'static AssetDescriptor<TranslationData<M>>,
        msg: M,
    ) -> ReadSignal<String> {
        let signal = self.0.clone();
        ctx.create_memo(move || {
            let rc = signal.as_ref().map(|s| s.read()).unwrap_or_default();
            rc.translate(desc, &msg)
        })
    }

    /// Memo: resolves and formats a translation whose parameters come from a
    /// factory (use this when parameters depend on other signals).
    pub fn translate_with<M: Message + 'static>(
        &self,
        ctx: &mut SetupContext,
        desc: &'static AssetDescriptor<TranslationData<M>>,
        mut msg_factory: impl FnMut() -> M + 'static,
    ) -> ReadSignal<String> {
        let signal = self.0.clone();
        ctx.create_memo(move || {
            let rc = signal.as_ref().map(|s| s.read()).unwrap_or_default();
            rc.translate(desc, &msg_factory())
        })
    }

    /// Memo: resolves a binary asset reactively.
    pub fn resolve_asset(
        &self,
        ctx: &mut SetupContext,
        asset: &'static AssetDescriptor<BinaryData>,
    ) -> ReadSignal<BinaryData> {
        let signal = self.0.clone();
        ctx.create_memo(move || {
            *signal
                .as_ref()
                .map(|s| s.read())
                .unwrap_or_default()
                .asset(asset)
        })
    }
}

// ---------------------------------------------------------------------------
// Context injection
// ---------------------------------------------------------------------------

/// Inject a [`ResourceContext`] signal into the reactive context tree.
///
/// Returns a [`StoredSignal`] so the caller can push updates via
/// `set_and_notify_changes`.
pub fn provide_resource_context(
    ctx: &mut SetupContext,
    initial: ResourceContext,
) -> StoredSignal<ResourceContext> {
    ctx.provide_context(&RESOURCE_CTX_KEY, initial)
}

/// Read the [`ResourceContext`] signal injected by an ancestor component.
pub fn use_resource_context(ctx: &SetupContext) -> ReactiveResourceContext {
    ReactiveResourceContext(ctx.use_context(&RESOURCE_CTX_KEY))
}
