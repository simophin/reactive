use reactive_core::ContextKey;
use std::num::NonZeroUsize;

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
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlignment {
    Leading,
    Center,
    Trailing,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoxModifier {
    Padding(EdgeInsets),
    Align(Alignment),
    SizedBox {
        width: Option<usize>,
        height: Option<usize>,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BoxModifierChain {
    pub modifiers: Vec<BoxModifier>,
}

impl BoxModifierChain {
    pub fn with_appended(mut self, modifier: BoxModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlexParentData {
    pub flex: Option<NonZeroUsize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChildLayoutInfo {
    pub box_modifiers: BoxModifierChain,
    pub flex: FlexParentData,
}

pub static BOX_MODIFIERS: ContextKey<BoxModifierChain> = ContextKey::new();
pub static FLEX_PARENT_DATA: ContextKey<FlexParentData> = ContextKey::new();

use reactive_core::Signal;

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
        assert!(matches!(
            CrossAxisAlignment::default(),
            CrossAxisAlignment::Stretch
        ));
    }

    #[test]
    fn main_axis_alignment_default_is_start() {
        assert!(matches!(
            MainAxisAlignment::default(),
            MainAxisAlignment::Start
        ));
    }

    #[test]
    fn box_modifier_chain_default() {
        let chain = BoxModifierChain::default();
        assert!(chain.modifiers.is_empty());
    }

    #[test]
    fn flex_parent_data_default() {
        let flex = FlexParentData::default();
        assert!(flex.flex.is_none());
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

impl Signal for TextAlignment {
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
