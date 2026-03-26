use reactive_core::{Component, SetupContext, Signal};

use super::with_updated_hints;

/// Constrains a child to a fixed width and/or height.
pub struct SizedBox<
    W: Signal<Value = Option<usize>>,
    H: Signal<Value = Option<usize>>,
    C: Component,
> {
    pub width: W,
    pub height: H,
    pub child: C,
}

impl<S: Signal<Value = Option<usize>> + Clone, C: Component> SizedBox<S, S, C> {
    pub fn square(size: S, child: C) -> Self {
        Self {
            width: size.clone(),
            height: size,
            child,
        }
    }
}

impl<
    W: Signal<Value = Option<usize>> + 'static,
    H: Signal<Value = Option<usize>> + 'static,
    C: Component,
> Component for SizedBox<W, H, C>
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let SizedBox {
            width,
            height,
            child,
        } = *self;

        with_updated_hints(ctx, move |h| {
            h.fixed_width = width.read();
            h.fixed_height = height.read();
        });

        ctx.child(child);
    }
}
