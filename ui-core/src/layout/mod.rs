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
    Alignment, CrossAxisAlignment, EdgeInsets, LAYOUT_HINTS, LayoutHints, MainAxisAlignment,
};

use reactive_core::{SetupContext, SignalExt};

fn with_updated_hints(ctx: &mut SetupContext, update: impl Fn(&mut LayoutHints) + 'static) {
    ctx.set_context(
        &LAYOUT_HINTS,
        ctx.use_context(&LAYOUT_HINTS).map_value(move |h| {
            let mut hints = h.unwrap_or_default();
            update(&mut hints);
            hints
        }),
    );
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::num::NonZeroUsize;
    use std::rc::Rc;

    use reactive_core::{Component, ReactiveScope, SetupContext, Signal};

    use super::*;

    /// Runs `component`, capturing the `LayoutHints` visible to its child via context.
    ///
    /// The hints are passed to `check` for assertions.  Because setup is
    /// synchronous the `Rc<Cell<…>>` trick avoids `RefCell` borrow dance.
    macro_rules! with_hints {
        ($component:expr, |$h:ident| $body:expr) => {{
            let captured: Rc<Cell<Option<LayoutHints>>> = Rc::new(Cell::new(None));
            let cap2 = Rc::clone(&captured);

            let scope = ReactiveScope::default();
            let mut ctx = SetupContext::new_root(&scope);

            Box::new(Padding {
                // Use an identity-Padding (zero insets) as the outer shell so
                // that `child` receives a `SetupContext` from which it can call
                // `use_context`.  The real component-under-test is nested inside.
                insets: EdgeInsets::default(),
                child: {
                    let comp = $component;
                    move |wrapper_ctx: &mut SetupContext| {
                        Box::new(comp).setup(wrapper_ctx);
                        // After the component's setup ran and stored its hints
                        // on wrapper_ctx, add a leaf child that reads them.
                        wrapper_ctx.child(Box::new(move |leaf: &mut SetupContext| {
                            cap2.set(leaf.use_context(&LAYOUT_HINTS).map(|s| s.read()));
                        }));
                    }
                },
            })
            .setup(&mut ctx);

            let $h = captured.get().expect("hints not captured");
            $body
        }};
    }

    #[test]
    fn padding_sets_insets() {
        with_hints!(
            Padding {
                insets: EdgeInsets::all(16),
                child: ()
            },
            |h| {
                assert_eq!(h.padding.top, 16);
                assert_eq!(h.padding.right, 16);
                assert_eq!(h.padding.bottom, 16);
                assert_eq!(h.padding.left, 16);
            }
        );
    }

    #[test]
    fn padding_symmetric() {
        with_hints!(
            Padding {
                insets: EdgeInsets::symmetric(8, 24),
                child: ()
            },
            |h| {
                assert_eq!(h.padding.top, 8);
                assert_eq!(h.padding.bottom, 8);
                assert_eq!(h.padding.left, 24);
                assert_eq!(h.padding.right, 24);
            }
        );
    }

    #[test]
    fn center_sets_center_alignment() {
        with_hints!(Center { child: () }, |h| {
            assert!(matches!(h.alignment, Some(Alignment::Center)));
        });
    }

    #[test]
    fn align_sets_given_alignment() {
        with_hints!(
            Align {
                alignment: Alignment::TopLeading,
                child: ()
            },
            |h| {
                assert!(matches!(h.alignment, Some(Alignment::TopLeading)));
            }
        );
    }

    #[test]
    fn sized_box_sets_width_and_height() {
        with_hints!(
            SizedBox {
                width: Some(100usize),
                height: Some(50usize),
                child: ()
            },
            |h| {
                assert_eq!(h.fixed_width, Some(100));
                assert_eq!(h.fixed_height, Some(50));
            }
        );
    }

    #[test]
    fn sized_box_square() {
        with_hints!(SizedBox::square(Some(64usize), ()), |h| {
            assert_eq!(h.fixed_width, Some(64));
            assert_eq!(h.fixed_height, Some(64));
        });
    }

    #[test]
    fn sized_box_partial_dimensions() {
        with_hints!(
            SizedBox {
                width: Some(200usize),
                height: None::<usize>,
                child: ()
            },
            |h| {
                assert_eq!(h.fixed_width, Some(200));
                assert!(h.fixed_height.is_none());
            }
        );
    }

    #[test]
    fn expanded_sets_flex() {
        let flex = NonZeroUsize::new(2).unwrap();
        with_hints!(
            Expanded {
                flex: Some(flex),
                child: ()
            },
            |h| {
                assert_eq!(h.flex, Some(flex));
            }
        );
    }

    #[test]
    fn expanded_flex_one() {
        let flex = NonZeroUsize::new(1).unwrap();
        with_hints!(
            Expanded {
                flex: Some(flex),
                child: ()
            },
            |h| {
                assert_eq!(h.flex.unwrap().get(), 1);
            }
        );
    }

    #[test]
    fn no_hints_without_layout_component() {
        // Verify that a plain child component sees no hints when none are set.
        let captured: Rc<Cell<Option<LayoutHints>>> = Rc::new(Cell::new(None));
        let cap2 = Rc::clone(&captured);

        let scope = ReactiveScope::default();
        let mut ctx = SetupContext::new_root(&scope);
        ctx.child(Box::new(move |leaf: &mut SetupContext| {
            cap2.set(leaf.use_context(&LAYOUT_HINTS).map(|s| s.read()));
        }));

        assert!(captured.get().is_none());
    }

    #[test]
    fn nested_padding_and_center_accumulate() {
        // Padding wraps Center: leaf child should see both padding and alignment.
        let captured: Rc<Cell<Option<LayoutHints>>> = Rc::new(Cell::new(None));
        let cap2 = Rc::clone(&captured);

        let scope = ReactiveScope::default();
        let mut ctx = SetupContext::new_root(&scope);

        Box::new(Padding {
            insets: EdgeInsets::all(10),
            child: move |inner: &mut SetupContext| {
                Box::new(Center {
                    child: move |leaf: &mut SetupContext| {
                        cap2.set(leaf.use_context(&LAYOUT_HINTS).map(|s| s.read()));
                    },
                })
                .setup(inner);
            },
        })
        .setup(&mut ctx);

        let h = captured.get().expect("hints not captured");
        assert_eq!(h.padding.top, 10);
        assert!(matches!(h.alignment, Some(Alignment::Center)));
    }
}
