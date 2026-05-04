use glib::object::Cast;
use gtk4::prelude::DialogExtManual;
use gtk4::{Overlay, Widget};
use reactive_core::{
    BoxedComponent, Component, ComponentId, IntoSignal, ReactiveScope, SetupContext, Signal,
    StoredSignal,
};
use std::collections::HashSet;
use std::rc::Rc;
use ui_core::widgets::{Alignment, Modifier, NativeView, NativeViewRegistry, Stack};

pub struct GtkStack {
    children: Vec<BoxedComponent>,
    alignment: Option<Box<dyn Signal<Value = Alignment>>>,
}

struct StackViewRegistry {
    scope: ReactiveScope,
    children: StoredSignal<Rc<Vec<(ComponentId, Widget, Modifier)>>>,
}

impl NativeViewRegistry<Widget> for StackViewRegistry {
    fn update_view(&self, component_id: ComponentId, view: Widget, modifier: Modifier) {
        self.children.update_with(|children| {
            if let Err(insertion) = children
                .binary_search_by(|(id, _, _)| self.scope.compare_components(*id, component_id))
            {
                Rc::make_mut(children).insert(insertion, (component_id, view, modifier));
                return true;
            }

            false
        })
    }

    fn clear_view(&self, component_id: ComponentId, view: Widget) {
        self.children.update_with(|children| {
            if let Some(index) = children.iter().position(|c| c.0 == component_id) {
                Rc::make_mut(children).remove(index);
                return true;
            }

            false
        });
    }
}

impl Stack for GtkStack {
    fn new() -> Self {
        Self {
            children: Vec::new(),
            alignment: None,
        }
    }

    fn alignment(mut self, alignment: impl Signal<Value = Alignment> + 'static) -> Self {
        self.alignment = Some(Box::new(alignment));
        self
    }

    fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl Component for GtkStack {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let GtkStack {
            children,
            alignment,
        } = *self;

        let overlay = NativeView::new(
            |_| Overlay::new(),
            |w| w.upcast(),
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        );

        let overlay = overlay.setup_in_component(ctx);

        let children_view = ctx.create_signal(Default::default());
        let registry = StackViewRegistry {
            scope: ctx.scope(),
            children: children_view.clone(),
        };

        let registry: Rc<dyn NativeViewRegistry<_>> = Rc::new(registry);
        ctx.set_context(&super::VIEW_REGISTRY_KEY, registry.into_signal());

        for child in children {
            ctx.boxed_child(child);
        }

        ctx.create_effect(move |_, added_children| {
            let mut added_children: HashSet<_> = added_children.unwrap_or_default();

            for (_, view, modifier) in children_view.read().iter() {
                if !added_children.contains(view) {
                    added_children.insert(view.clone());
                    overlay.ins
                    overlay.add_overlay(view);
                }
            }

            added_children
        });
    }
}
