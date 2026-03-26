#[derive(Clone, Copy, Debug, Default)]
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
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Alignment {
    TopLeading,
    Top,
    TopTrailing,
    Leading,
    #[default]
    Center,
    Trailing,
    BottomLeading,
    Bottom,
    BottomTrailing,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum CrossAxisAlignment {
    #[default]
    Stretch,
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum MainAxisAlignment {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Layout hints propagated via context from logical layout components to the
/// nearest real view component below them. Real view components consume and
/// reset these hints so they don't leak to grandchildren.
#[derive(Clone, Copy, Debug, Default)]
pub struct LayoutHints {
    pub padding: EdgeInsets,
    /// Override the default fill/stretch positioning within the parent.
    pub alignment: Option<Alignment>,
    pub fixed_width: Option<usize>,
    pub fixed_height: Option<usize>,
    /// Non-zero flex factor for Row/Column children.
    pub flex: Option<NonZeroUsize>,
}

use reactive_core::{ContextKey, Signal};
use std::num::NonZeroUsize;
pub static LAYOUT_HINTS: ContextKey<LayoutHints> = ContextKey::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_insets_all() {
        let e = EdgeInsets::all(8);
        assert_eq!(e.top, 8);
        assert_eq!(e.right, 8);
        assert_eq!(e.bottom, 8);
        assert_eq!(e.left, 8);
    }

    #[test]
    fn edge_insets_symmetric() {
        let e = EdgeInsets::symmetric(4, 12);
        assert_eq!(e.top, 4);
        assert_eq!(e.bottom, 4);
        assert_eq!(e.left, 12);
        assert_eq!(e.right, 12);
    }

    #[test]
    fn edge_insets_default_is_zero() {
        let e = EdgeInsets::default();
        assert_eq!(e.top, 0);
        assert_eq!(e.right, 0);
        assert_eq!(e.bottom, 0);
        assert_eq!(e.left, 0);
    }

    #[test]
    fn alignment_default_is_center() {
        assert!(matches!(Alignment::default(), Alignment::Center));
    }

    #[test]
    fn cross_axis_alignment_default_is_stretch() {
        assert!(matches!(CrossAxisAlignment::default(), CrossAxisAlignment::Stretch));
    }

    #[test]
    fn main_axis_alignment_default_is_start() {
        assert!(matches!(MainAxisAlignment::default(), MainAxisAlignment::Start));
    }

    #[test]
    fn layout_hints_default() {
        let h = LayoutHints::default();
        assert_eq!(h.padding.top, 0);
        assert!(h.alignment.is_none());
        assert!(h.fixed_width.is_none());
        assert!(h.fixed_height.is_none());
        assert!(h.flex.is_none());
    }

    #[test]
    fn signal_impls_return_copy() {
        let e = EdgeInsets::all(3);
        assert_eq!(Signal::read(&e).top, 3);

        let a = Alignment::TopLeading;
        assert!(matches!(Signal::read(&a), Alignment::TopLeading));

        let c = CrossAxisAlignment::End;
        assert!(matches!(Signal::read(&c), CrossAxisAlignment::End));

        let m = MainAxisAlignment::SpaceBetween;
        assert!(matches!(Signal::read(&m), MainAxisAlignment::SpaceBetween));
    }
}

impl Signal for EdgeInsets {
    type Value = Self;

    fn read(&self) -> Self::Value {
        *self
    }
}

impl Signal for Alignment {
    type Value = Self;

    fn read(&self) -> Self::Value {
        *self
    }
}

impl Signal for CrossAxisAlignment {
    type Value = Self;

    fn read(&self) -> Self::Value {
        *self
    }
}

impl Signal for MainAxisAlignment {
    type Value = Self;

    fn read(&self) -> Self::Value {
        *self
    }
}
