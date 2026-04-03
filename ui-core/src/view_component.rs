use crate::layout::{
    BOX_MODIFIERS, BoxModifierChain, ChildLayoutInfo, FLEX_PARENT_DATA, FlexParentData,
};
use crate::{Prop, ViewBuilder};
use reactive_core::{BoxedComponent, Component, ContextKey, IntoSignal, SetupContext, Signal, StoredSignal};

/// Describes how many child components a view has.
pub trait ChildStrategy {
    fn into_data(self) -> Vec<BoxedComponent>;
}

pub struct NoChild;
pub struct SingleChild(pub BoxedComponent);
pub struct AtMostOneChild(pub Option<BoxedComponent>);
pub struct MultipleChildren(pub Vec<BoxedComponent>);

impl ChildStrategy for NoChild {
    fn into_data(self) -> Vec<BoxedComponent> {
        Vec::new()
    }
}

impl ChildStrategy for SingleChild {
    fn into_data(self) -> Vec<BoxedComponent> {
        vec![self.0]
    }
}

impl ChildStrategy for AtMostOneChild {
    fn into_data(self) -> Vec<BoxedComponent> {
        match self.0 {
            Some(v) => vec![v],
            None => Vec::new(),
        }
    }
}

impl ChildStrategy for MultipleChildren {
    fn into_data(self) -> Vec<BoxedComponent> {
        self.0
    }
}

/// Platform-agnostic child entry: the native view handle plus layout metadata.
/// Stored in context so parent containers can add the child to their hierarchy.
#[derive(Clone, PartialEq, Eq)]
pub struct ChildEntry<NativeView> {
    pub native: NativeView,
    pub layout: ChildLayoutInfo,
}

struct IndexedChild<NativeView: 'static> {
    child_entry: StoredSignal<Option<ChildEntry<NativeView>>>,
    child_key: &'static ContextKey<StoredSignal<Option<ChildEntry<NativeView>>>>,
    child: BoxedComponent,
}

impl<NativeView: Clone + PartialEq + Eq + 'static> Component for IndexedChild<NativeView> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            child,
            child_entry,
            child_key,
        } = *self;
        ctx.set_context(child_key, child_entry.into_signal());
        ctx.boxed_child(child);
    }
}

/// Generic platform view builder. Wraps a [`ViewBuilder`] and adds:
/// - child component management via [`ChildStrategy`]
/// - reactive context wiring (parent slot signal, layout hints)
///
/// Each platform wraps this in a thin newtype that provides its specific
/// context keys and any platform-specific post-creation setup.
pub struct PlatformViewBuilder<Target, NativeView: 'static, C> {
    builder: ViewBuilder<Target>,
    children: C,
    into_native_view: fn(Target) -> NativeView,
    child_key: &'static ContextKey<StoredSignal<Option<ChildEntry<NativeView>>>>,
    children_key: &'static ContextKey<Vec<StoredSignal<Option<ChildEntry<NativeView>>>>>,
}

impl<Target: Clone + 'static, NativeView: Clone + PartialEq + Eq + 'static>
    PlatformViewBuilder<Target, NativeView, NoChild>
{
    pub fn create_no_child(
        creator: impl FnOnce(&mut SetupContext) -> Target + 'static,
        into_native_view: fn(Target) -> NativeView,
        child_key: &'static ContextKey<StoredSignal<Option<ChildEntry<NativeView>>>>,
        children_key: &'static ContextKey<Vec<StoredSignal<Option<ChildEntry<NativeView>>>>>,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: NoChild,
            into_native_view,
            child_key,
            children_key,
        }
    }
}

impl<Target: Clone + 'static, NativeView: Clone + PartialEq + Eq + 'static>
    PlatformViewBuilder<Target, NativeView, SingleChild>
{
    pub fn create_with_child(
        creator: impl FnOnce(&mut SetupContext) -> Target + 'static,
        into_native_view: fn(Target) -> NativeView,
        child_key: &'static ContextKey<StoredSignal<Option<ChildEntry<NativeView>>>>,
        children_key: &'static ContextKey<Vec<StoredSignal<Option<ChildEntry<NativeView>>>>>,
        child: BoxedComponent,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: SingleChild(child),
            into_native_view,
            child_key,
            children_key,
        }
    }
}

impl<Target: Clone + 'static, NativeView: Clone + PartialEq + Eq + 'static>
    PlatformViewBuilder<Target, NativeView, AtMostOneChild>
{
    pub fn create_with_optional_child(
        creator: impl FnOnce(&mut SetupContext) -> Target + 'static,
        into_native_view: fn(Target) -> NativeView,
        child_key: &'static ContextKey<StoredSignal<Option<ChildEntry<NativeView>>>>,
        children_key: &'static ContextKey<Vec<StoredSignal<Option<ChildEntry<NativeView>>>>>,
        child: Option<BoxedComponent>,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: AtMostOneChild(child),
            into_native_view,
            child_key,
            children_key,
        }
    }
}

impl<Target: Clone + 'static, NativeView: Clone + PartialEq + Eq + 'static>
    PlatformViewBuilder<Target, NativeView, MultipleChildren>
{
    pub fn create_multiple_child(
        creator: impl FnOnce(&mut SetupContext) -> Target + 'static,
        into_native_view: fn(Target) -> NativeView,
        child_key: &'static ContextKey<StoredSignal<Option<ChildEntry<NativeView>>>>,
        children_key: &'static ContextKey<Vec<StoredSignal<Option<ChildEntry<NativeView>>>>>,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: MultipleChildren(Vec::new()),
            into_native_view,
            child_key,
            children_key,
        }
    }

    pub fn add_child(mut self, child: BoxedComponent) -> Self {
        self.children.0.push(child);
        self
    }
}

impl<Target: Clone + 'static, NativeView: Clone + PartialEq + Eq + 'static, C>
    PlatformViewBuilder<Target, NativeView, C>
{
    pub fn bind<FrameworkType, ValueType>(
        mut self,
        prop: &'static Prop<FrameworkType, Target, ValueType>,
        value: impl Signal<Value = ValueType> + 'static,
    ) -> Self
    where
        ValueType: 'static,
    {
        self.builder.bind(prop, value);
        self
    }

    /// Set up the view, wiring reactive parent/child context.
    ///
    /// `on_native_view` is called immediately after the native view is created,
    /// before context registration — use it for platform-specific post-creation
    /// setup (e.g. AppKit's `setTranslatesAutoresizingMaskIntoConstraints`).
    pub fn setup(
        self,
        ctx: &mut SetupContext,
        on_native_view: impl FnOnce(&NativeView),
    ) -> Target
    where
        C: ChildStrategy,
    {
        let PlatformViewBuilder {
            builder,
            children,
            into_native_view,
            child_key,
            children_key,
        } = self;

        let target = builder.setup(ctx);
        let native = into_native_view(target.clone());

        on_native_view(&native);

        if let Some(child_entry_signal) = ctx.use_context(child_key) {
            let native = native.clone();
            let box_modifiers = ctx.use_context(&BOX_MODIFIERS);
            let flex_parent_data = ctx.use_context(&FLEX_PARENT_DATA);
            ctx.create_effect(move |_, _| {
                child_entry_signal
                    .read()
                    .update_if_changes(Some(ChildEntry {
                        native: native.clone(),
                        layout: ChildLayoutInfo {
                            box_modifiers: box_modifiers.read().unwrap_or_default(),
                            flex: flex_parent_data.read().unwrap_or_default(),
                        },
                    }));
            });
        }

        ctx.set_context(&BOX_MODIFIERS, BoxModifierChain::default().into_signal());
        ctx.set_context(&FLEX_PARENT_DATA, FlexParentData::default().into_signal());

        let children_data = children.into_data();
        if !children_data.is_empty() {
            let signals_signal = ctx.provide_context(
                children_key,
                (0..children_data.len())
                    .map(|_| ctx.create_signal(None))
                    .collect::<Vec<_>>(),
            );
            let signals = signals_signal.read();
            for (child, entry_signal) in children_data.into_iter().zip(signals.into_iter()) {
                ctx.boxed_child(Box::new(IndexedChild {
                    child,
                    child_entry: entry_signal,
                    child_key,
                }));
            }
        }

        target
    }
}

/// A [`Component`] that wraps a [`PlatformViewBuilder`].
/// For platforms that do not need post-creation setup; use the platform-specific
/// builder wrapper when you need that (e.g. `AppKitViewBuilder`).
pub struct PlatformViewComponent<Target, NativeView: 'static, C>(
    pub PlatformViewBuilder<Target, NativeView, C>,
);

impl<Target: Clone + 'static, NativeView: Clone + PartialEq + Eq + 'static, C: ChildStrategy>
    Component for PlatformViewComponent<Target, NativeView, C>
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        self.0.setup(ctx, |_| {});
    }
}
