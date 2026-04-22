use crate::widgets::modifier::{Modifier, ModifierKey};
use reactive_core::{IntoSignal, Signal, SignalExt};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EdgeInsets {
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
    pub left: usize,
}

impl EdgeInsets {
    pub fn all(v: usize) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    pub fn symmetric(vertical: usize, horizontal: usize) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    pub fn plus(mut self, insets: &EdgeInsets) -> Self {
        self.top += insets.top;
        self.right += insets.right;
        self.bottom += insets.bottom;
        self.left += insets.left;

        self
    }
}

impl Signal for EdgeInsets {
    type Value = Self;

    fn read(&self) -> Self::Value {
        *self
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum SizeSpec {
    Fixed(usize),
    #[default]
    Unspecified,
}

impl From<usize> for SizeSpec {
    fn from(value: usize) -> Self {
        SizeSpec::Fixed(value)
    }
}

impl SizeSpec {
    pub fn plus(self, new: Self) -> Self {
        match (self, new) {
            (SizeSpec::Unspecified, _) | (_, SizeSpec::Fixed(_)) => new,
            (SizeSpec::Fixed(old), SizeSpec::Unspecified) => SizeSpec::Fixed(old),
        }
    }
}

pub trait CommonModifiers {
    fn paddings(self, edge_insets: impl Signal<Value = EdgeInsets> + 'static) -> Self;
    fn get_paddings(&self) -> impl Signal<Value = Option<EdgeInsets>> + 'static;

    fn width(self, width: impl Signal<Value = usize> + 'static) -> Self
    where
        Self: Sized,
    {
        self.sized(width, || SizeSpec::Unspecified)
    }

    fn height(self, height: impl Signal<Value = usize> + 'static) -> Self
    where
        Self: Sized,
    {
        self.sized(|| SizeSpec::Unspecified, height)
    }

    fn sized<W, H>(
        self,
        width: impl Signal<Value = W> + 'static,
        height: impl Signal<Value = H> + 'static,
    ) -> Self
    where
        W: Into<SizeSpec>,
        H: Into<SizeSpec>;

    fn get_size(&self) -> impl Signal<Value = (SizeSpec, SizeSpec)> + 'static;
}

static PADDINGS_KEY: ModifierKey<EdgeInsets> =
    ModifierKey::with_merger(|old_signal, new_value| old_signal.read().plus(&new_value));

static SIZES_KEY: ModifierKey<(SizeSpec, SizeSpec)> =
    ModifierKey::with_merger(|old_signal, (new_width, new_height)| {
        let (old_width, old_height) = old_signal.read();
        (old_width.plus(new_width), old_height.plus(new_height))
    });

impl CommonModifiers for Modifier {
    fn paddings(self, edge_insets: impl Signal<Value = EdgeInsets> + 'static) -> Self {
        self.with(&PADDINGS_KEY, edge_insets)
    }

    fn get_paddings(&self) -> impl Signal<Value = Option<EdgeInsets>> + 'static {
        self.get(&PADDINGS_KEY)
    }

    fn sized<W, H>(
        self,
        width: impl Signal<Value = W> + 'static,
        height: impl Signal<Value = H> + 'static,
    ) -> Self
    where
        W: Into<SizeSpec>,
        H: Into<SizeSpec>,
    {
        self.with(&SIZES_KEY, move || {
            (width.read().into(), height.read().into())
        })
    }

    fn get_size(&self) -> impl Signal<Value = (SizeSpec, SizeSpec)> + 'static {
        self.get(&SIZES_KEY).map_value(|v| v.unwrap_or_default())
    }
}
