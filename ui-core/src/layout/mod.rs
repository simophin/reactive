mod align;
mod center;
mod expanded;
mod padding;
mod sized_box;
pub mod types;

pub use align::Align;
pub use center::Center;
pub use expanded::Expanded;
pub use padding::Padding;
pub use sized_box::SizedBox;
pub use types::{
    Alignment, BOX_MODIFIERS, BoxModifier, BoxModifierChain, ChildLayoutInfo, CrossAxisAlignment,
    EdgeInsets, FLEX_PARENT_DATA, FlexParentData, MainAxisAlignment,
};

use reactive_core::{SetupContext, SignalExt};

fn with_appended_box_modifier(ctx: &mut SetupContext, modifier: BoxModifier) {
    ctx.set_context(
        &BOX_MODIFIERS,
        ctx.use_context(&BOX_MODIFIERS)
            .map_value(move |chain| chain.unwrap_or_default().with_appended(modifier.clone())),
    );
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::num::NonZeroUsize;
    use std::rc::Rc;

    use reactive_core::{Component, ReactiveScope, SetupContext, Signal};

    use super::*;

    macro_rules! with_layout_info {
        ($component:expr, |$info:ident| $body:expr) => {{
            let captured: Rc<RefCell<Option<ChildLayoutInfo>>> = Rc::new(RefCell::new(None));
            let cap2 = Rc::clone(&captured);

            let scope = ReactiveScope::default();
            let mut ctx = SetupContext::new_root(&scope);

            Box::new({
                let comp = $component;
                move |wrapper_ctx: &mut SetupContext| {
                    Box::new(comp).setup(wrapper_ctx);
                    wrapper_ctx.boxed_child(Box::new(move |leaf: &mut SetupContext| {
                        *cap2.borrow_mut() = Some(ChildLayoutInfo {
                            box_modifiers: leaf
                                .use_context(&BOX_MODIFIERS)
                                .map(|s| s.read())
                                .unwrap_or_default(),
                            flex: leaf
                                .use_context(&FLEX_PARENT_DATA)
                                .map(|s| s.read())
                                .unwrap_or_default(),
                        });
                    }));
                }
            })
            .setup(&mut ctx);

            let $info = captured.borrow().clone().expect("layout info not captured");
            $body
        }};
    }

    #[test]
    fn padding_appends_padding_modifier() {
        with_layout_info!(
            Padding {
                insets: EdgeInsets::all(16),
                child: ()
            },
            |info| {
                assert_eq!(
                    info.box_modifiers.modifiers,
                    vec![BoxModifier::Padding(EdgeInsets::all(16))]
                );
            }
        );
    }

    #[test]
    fn center_appends_align_modifier() {
        with_layout_info!(Center { child: () }, |info| {
            assert_eq!(
                info.box_modifiers.modifiers,
                vec![BoxModifier::Align(Alignment::Center)]
            );
        });
    }

    #[test]
    fn align_appends_given_alignment() {
        with_layout_info!(
            Align {
                alignment: Alignment::TopLeading,
                child: ()
            },
            |info| {
                assert_eq!(
                    info.box_modifiers.modifiers,
                    vec![BoxModifier::Align(Alignment::TopLeading)]
                );
            }
        );
    }

    #[test]
    fn sized_box_appends_sized_box_modifier() {
        with_layout_info!(
            SizedBox {
                width: Some(100usize),
                height: Some(50usize),
                child: ()
            },
            |info| {
                assert_eq!(
                    info.box_modifiers.modifiers,
                    vec![BoxModifier::SizedBox {
                        width: Some(100),
                        height: Some(50)
                    }]
                );
            }
        );
    }

    #[test]
    fn expanded_sets_flex_parent_data() {
        let flex = NonZeroUsize::new(2).unwrap();
        with_layout_info!(
            Expanded {
                flex: Some(flex),
                child: ()
            },
            |info| {
                assert_eq!(info.flex.flex, Some(flex));
            }
        );
    }

    #[test]
    fn no_layout_data_without_layout_component() {
        let captured: Rc<RefCell<Option<ChildLayoutInfo>>> = Rc::new(RefCell::new(None));
        let cap2 = Rc::clone(&captured);

        let scope = ReactiveScope::default();
        let ctx = SetupContext::new_root(&scope);
        ctx.boxed_child(Box::new(move |leaf: &mut SetupContext| {
            *cap2.borrow_mut() = Some(ChildLayoutInfo {
                box_modifiers: leaf
                    .use_context(&BOX_MODIFIERS)
                    .map(|s| s.read())
                    .unwrap_or_default(),
                flex: leaf
                    .use_context(&FLEX_PARENT_DATA)
                    .map(|s| s.read())
                    .unwrap_or_default(),
            });
        }));

        let info = captured.borrow().clone().expect("layout info not captured");
        assert!(info.box_modifiers.modifiers.is_empty());
        assert!(info.flex.flex.is_none());
    }

    #[test]
    fn nested_modifiers_preserve_order() {
        let captured: Rc<RefCell<Option<ChildLayoutInfo>>> = Rc::new(RefCell::new(None));
        let cap2 = Rc::clone(&captured);

        let scope = ReactiveScope::default();
        let mut ctx = SetupContext::new_root(&scope);

        Box::new(Padding {
            insets: EdgeInsets::all(10),
            child: Center {
                child: move |leaf: &mut SetupContext| {
                    *cap2.borrow_mut() = Some(ChildLayoutInfo {
                        box_modifiers: leaf
                            .use_context(&BOX_MODIFIERS)
                            .map(|s| s.read())
                            .unwrap_or_default(),
                        flex: leaf
                            .use_context(&FLEX_PARENT_DATA)
                            .map(|s| s.read())
                            .unwrap_or_default(),
                    });
                },
            },
        })
        .setup(&mut ctx);

        let info = captured.borrow().clone().expect("layout info not captured");
        assert_eq!(
            info.box_modifiers.modifiers,
            vec![
                BoxModifier::Padding(EdgeInsets::all(10)),
                BoxModifier::Align(Alignment::Center),
            ]
        );
    }
}
