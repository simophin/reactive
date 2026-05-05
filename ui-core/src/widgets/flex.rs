use crate::widgets::{ModifierKey, WithModifier};
use reactive_core::{Component, Signal};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FlexUnit {
    Absolute(usize),
    Percent(u8),
}

impl Default for FlexUnit {
    fn default() -> Self {
        FlexUnit::Absolute(0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum JustifyContent {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum AlignItems {
    #[default]
    Stretch,
    Center,
    Start,
    End,
    Baseline,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum AlignContent {
    #[default]
    Stretch,
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct FlexProps {
    pub direction: FlexDirection,
    pub wrap: FlexWrap,
    pub gap: FlexUnit,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_content: AlignContent,
}

#[derive(Copy, Clone, Default)]
pub struct FlexScope;

pub static KEY_ORDER: ModifierKey<isize> = ModifierKey::new();
pub static KEY_FLEX_GROW: ModifierKey<f32> = ModifierKey::new();
pub static KEY_FLEX_SHRINK: ModifierKey<f32> = ModifierKey::new();
pub static KEY_FLEX_BASIS: ModifierKey<FlexUnit> = ModifierKey::new();
pub static KEY_ALIGN_SELF: ModifierKey<AlignItems> = ModifierKey::new();

impl FlexScope {
    pub fn order(&self) -> &'static ModifierKey<isize> {
        &KEY_ORDER
    }

    pub fn flex_grow(&self) -> &'static ModifierKey<f32> {
        &KEY_FLEX_GROW
    }

    pub fn flex_shrink(&self) -> &'static ModifierKey<f32> {
        &KEY_FLEX_SHRINK
    }

    pub fn flex_basis(&self) -> &'static ModifierKey<FlexUnit> {
        &KEY_FLEX_BASIS
    }

    pub fn align_self(&self) -> &'static ModifierKey<AlignItems> {
        &KEY_ALIGN_SELF
    }

    pub fn flex_grow_shrink(&self) -> &'static ModifierKey<f32> {
        &KEY_FLEX_SHRINK
    }
}

pub trait Flex: WithModifier + Component + 'static {
    fn new(props: impl Signal<Value = FlexProps> + 'static) -> Self;

    fn with_child<C: Component + 'static>(self, factory: impl FnOnce(FlexScope) -> C) -> Self;
}
