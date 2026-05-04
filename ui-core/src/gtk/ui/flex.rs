use gtk4::ffi::*;
use gtk4::prelude::FixedExt;
use gtk4::{Fixed, Widget};
use reactive_core::BoxedComponent;
use reactive_core::{Component, ComponentId, SetupContext, Signal};
use std::ptr;
use taffy;
use ui_core::widgets::taffy::TaffyTreeManager;
use ui_core::widgets::{
    FlexProps, FlexScope, KEY_ALIGN_SELF, KEY_FLEX_BASIS, KEY_FLEX_GROW, KEY_FLEX_SHRINK,
    KEY_ORDER, Modifier, NativeViewRegistry, WithModifier,
};

pub struct Flex {
    props: Box<dyn Signal<Value = FlexProps>>,
    children: Vec<BoxedComponent>,
    modifier: Modifier,
}

struct FlexViewRegistry {
    tree: TaffyTreeManager<Widget>,
    my_view: Fixed,
}

impl NativeViewRegistry<Widget> for FlexViewRegistry {
    fn update_view(&self, component_id: ComponentId, view: Widget, modifier: Modifier) {
        self.my_view.put(&view, 0.0, 0.0);
        self.tree
            .upsert_node(component_id, view, modifier, Default::default());
    }

    fn clear_view(&self, component_id: ComponentId, view: Widget) {
        self.my_view.remove(&view);
        self.tree.remove_node(component_id, view);
    }
}

impl Component for Flex {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Flex {
            props,
            children,
            modifier: _,
        } = *self;

        let builder = children.into_iter().fold(
            GtkViewBuilder::create_multiple_child(
                |_| unsafe { gtk_fixed_new() },
                |w| w as *mut GtkWidget,
            ),
            |builder, child| builder.add_child(child),
        );

        let fixed = builder.setup(ctx);

        let tree = TaffyTreeManager::new(ctx.scope());

        let props_clone = props.clone();
        ctx.create_effect(move |_, _| {
            let style = flex_props_to_style(&props_clone.read());
            tree.set_root_style(style);
        });

        if let Some(children_widgets) = ctx.use_context(&CHILDREN_WIDGETS) {
            let fixed_clone = fixed;
            let tree_clone = tree.clone();
            ctx.create_effect(move |_, _| {
                let entries: Vec<_> = children_widgets
                    .read()
                    .into_iter()
                    .filter_map(|s| s.read())
                    .collect();

                for entry in &entries {
                    let style = modifier_to_style(&entry.modifier);
                    tree_clone.upsert_node(
                        entry.component_id,
                        entry.native,
                        entry.modifier.clone(),
                        style,
                    );
                }

                // Compute layout
                let available = taffy::Size {
                    width: taffy::AvailableSpace::Definite(800.0), // TODO: get actual size
                    height: taffy::AvailableSpace::Definite(600.0),
                };

                tree_clone.compute_layout(available, |_available, _view| {
                    // Measure view
                    taffy::Size {
                        width: 100.0,
                        height: 100.0,
                    } // TODO: measure
                });

                // Apply layouts
                for (view, layout) in tree_clone.children_layouts() {
                    unsafe {
                        gtk_fixed_move(
                            fixed_clone as *mut GtkFixed,
                            view,
                            layout.location.x,
                            layout.location.y,
                        );
                        gtk_widget_set_size_request(
                            view,
                            layout.size.width as i32,
                            layout.size.height as i32,
                        );
                    }
                }
            });
        }
    }
}

impl WithModifier for Flex {
    fn modifier(mut self, modifier: Modifier) -> Self {
        self.modifier = modifier;
        self
    }
}

impl ui_core::widgets::Flex for Flex {
    fn new(props: impl Signal<Value = FlexProps> + 'static) -> Self {
        Flex {
            props: Box::new(props),
            children: Vec::new(),
            modifier: Modifier::default(),
        }
    }

    fn with_child<C: Component + 'static>(mut self, factory: impl FnOnce(FlexScope) -> C) -> Self {
        let child = factory(FlexScope);
        self.children.push(Box::new(child));
        self
    }
}
