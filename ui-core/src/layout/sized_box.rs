use reactive_core::{Component, SetupContext, Signal};

use super::{BoxModifier, with_appended_box_modifier};

/// Constrains a child to a fixed width and/or height.
pub struct SizedBox<W, H, C> {
    pub width: W,
    pub height: H,
    pub child: C,
}

pub struct MustHaveChild;

impl SizedBox<(), (), ()> {
    pub fn width<W>(width: W) -> SizedBox<W, Option<usize>, MustHaveChild>
    where
        W: Signal<Value = usize>,
    {
        SizedBox {
            width,
            height: None,
            child: MustHaveChild,
        }
    }

    pub fn height<H>(height: H) -> SizedBox<Option<usize>, H, MustHaveChild>
    where
        H: Signal<Value = usize>,
    {
        SizedBox {
            width: None,
            height,
            child: MustHaveChild,
        }
    }

    pub fn sized<W, H>(width: W, height: H) -> SizedBox<W, H, MustHaveChild>
    where
        W: Signal<Value = usize>,
        H: Signal<Value = usize>,
    {
        SizedBox {
            width,
            height,
            child: MustHaveChild,
        }
    }

    pub fn squared<S>(size: S) -> SizedBox<S, S, MustHaveChild>
    where
        S: Signal<Value = usize> + Clone,
    {
        SizedBox {
            width: size.clone(),
            height: size,
            child: MustHaveChild,
        }
    }
}

impl<W, H, C> SizedBox<W, H, C> {
    pub fn child<NC>(self, child: NC) -> SizedBox<W, H, NC>
    where
        NC: Component + 'static,
    {
        SizedBox {
            width: self.width,
            height: self.height,
            child,
        }
    }
}

impl<W, H, C> Component for SizedBox<W, H, C>
where
    W: Signal + 'static,
    H: Signal + 'static,
    C: Component + 'static,
    <W as Signal>::Value: Into<Option<usize>> + 'static,
    <H as Signal>::Value: Into<Option<usize>> + 'static,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            width,
            height,
            child,
        } = *self;

        with_appended_box_modifier(ctx, move || BoxModifier::SizedBox {
            width: width.read().into(),
            height: height.read().into(),
        });

        ctx.child(child);
    }
}
