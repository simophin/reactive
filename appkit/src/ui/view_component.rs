use apple::ViewBuilder;
use apple::bindable::BindableView;
use objc2::Message;
use objc2::rc::Retained;
use objc2_app_kit::{NSStackView, NSView};
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use ui_core::layout::LAYOUT_HINTS;

use super::context::{PARENT_VIEW, ViewParent};
use super::flex::{ChildEntry, CHILD_VIEW_REGISTRY, activate_fill};

pub struct AppKitViewComponent<V, Children> {
    builder: ViewBuilder<V>,
    children: Children,
    into_nsview: fn(Retained<V>) -> Retained<NSView>,
}

impl<V: Message, Children> AsMut<ViewBuilder<V>> for AppKitViewComponent<V, Children> {
    fn as_mut(&mut self) -> &mut ViewBuilder<V> {
        &mut self.builder
    }
}

impl<V: Message, Children> BindableView<V> for AppKitViewComponent<V, Children> {}

impl<V: Message, Children: Default> AppKitViewComponent<V, Children> {
    pub(super) fn create(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: Default::default(),
            into_nsview,
        }
    }
}

impl<V> AppKitViewComponent<V, Vec<BoxedComponent>> {
    pub fn child(mut self, c: impl Component + 'static) -> Self {
        self.children.push(Box::new(c));
        self
    }
}

impl<V> AppKitViewComponent<V, Option<BoxedComponent>> {
    pub fn child(mut self, c: impl Component + 'static) -> Self {
        self.children.replace(Box::new(c));
        self
    }
}

trait IntoVec<T> {
    fn into_vec(self) -> Vec<T>;
}

impl<T> IntoVec<T> for Vec<T> {
    fn into_vec(self) -> Vec<T> {
        self
    }
}

impl<T> IntoVec<T> for Option<T> {
    fn into_vec(self) -> Vec<T> {
        self.into_iter().collect()
    }
}

impl<T> IntoVec<T> for () {
    fn into_vec(self) -> Vec<T> {
        Vec::new()
    }
}

impl<V: Message, Children: IntoVec<BoxedComponent>> Component for AppKitViewComponent<V, Children> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let AppKitViewComponent { builder, children, into_nsview } = *self;
        let view = builder.setup(ctx);
        let nsview = into_nsview(view);
        nsview.setTranslatesAutoresizingMaskIntoConstraints(false);

        let hints = ctx.use_context(&LAYOUT_HINTS).map(|s| s.read()).unwrap_or_default();

        if let Some(parent) = ctx.use_context(&PARENT_VIEW) {
            let parent = parent.read();
            parent.add_child(nsview.clone());
            ctx.on_cleanup({
                let nsview = nsview.clone();
                move || nsview.removeFromSuperview()
            });

            if let Some(registry) = ctx.use_context(&CHILD_VIEW_REGISTRY) {
                registry.read().borrow_mut().push(ChildEntry { view: nsview.clone(), hints });
            } else if let ViewParent::View(parent_nsview) = &parent {
                activate_fill(&nsview, parent_nsview, &hints);
            }
            // ViewParent::Stack: NSStackView arranges its children; no extra constraints needed.
        }

        // Provide a parent context for any children of this view.
        let any: &objc2::runtime::AnyObject = &*nsview;
        if let Some(stack) = any.downcast_ref::<NSStackView>() {
            ctx.provide_context(&PARENT_VIEW, ViewParent::Stack(stack.retain()));
        } else {
            ctx.provide_context(&PARENT_VIEW, ViewParent::View(nsview.clone()));
        }

        ctx.on_cleanup(move || drop(nsview));

        for child in children.into_vec() {
            let mut child_ctx = ctx.new_child();
            child.setup(&mut child_ctx);
        }
    }
}
