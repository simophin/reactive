use crate::widgets::{CommonModifiers, Modifier, SizeSpec};
use reactive_core::Signal;
use std::rc::Rc;
use taffy::{
    AlignContent, AlignItems, AlignSelf, BoxGenerationMode, BoxSizing, CoreStyle, Dimension,
    Direction, FlexDirection, FlexWrap, FlexboxContainerStyle, FlexboxItemStyle, JustifyContent,
    LengthPercentage, LengthPercentageAuto, Overflow, Point, Position, Rect, Size,
};

impl From<super::flex::FlexUnit> for Dimension {
    fn from(value: super::flex::FlexUnit) -> Self {
        match value {
            super::flex::FlexUnit::Absolute(length) => Dimension::length(length as f32),
            super::flex::FlexUnit::Percent(percent) => Dimension::percent(percent as f32 / 100.0),
        }
    }
}

impl From<super::flex::FlexUnit> for LengthPercentage {
    fn from(value: super::flex::FlexUnit) -> Self {
        match value {
            super::flex::FlexUnit::Absolute(length) => LengthPercentage::length(length as f32),
            super::flex::FlexUnit::Percent(percent) => {
                LengthPercentage::percent(percent as f32 / 100.0)
            }
        }
    }
}

impl From<SizeSpec> for Dimension {
    fn from(value: SizeSpec) -> Self {
        match value {
            SizeSpec::Fixed(length) => Dimension::length(length as f32),
            SizeSpec::Unspecified => Dimension::auto(),
        }
    }
}

impl CoreStyle for Modifier {
    type CustomIdent = Rc<str>;

    fn size(&self) -> Size<Dimension> {
        let (w, h) = self.get_size().read();
        Size {
            width: w.into(),
            height: h.into(),
        }
    }

    fn padding(&self) -> Rect<LengthPercentage> {
        let paddings = self.get_paddings().read().unwrap_or_default();
        Rect {
            left: LengthPercentage::length(paddings.left as f32),
            right: LengthPercentage::length(paddings.right as f32),
            top: LengthPercentage::length(paddings.top as f32),
            bottom: LengthPercentage::length(paddings.bottom as f32),
        }
    }
}

impl FlexboxItemStyle for Modifier {
    fn flex_basis(&self) -> Dimension {
        self.get(&super::flex::KEY_FLEX_BASIS)
            .read()
            .map(|basis| basis.into())
            .unwrap_or(Dimension::auto())
    }

    fn flex_grow(&self) -> f32 {
        self.get(&super::flex::KEY_FLEX_GROW).read().unwrap_or(0.0)
    }

    fn flex_shrink(&self) -> f32 {
        self.get(&super::flex::KEY_FLEX_SHRINK)
            .read()
            .unwrap_or(0.0)
    }

    fn align_self(&self) -> Option<AlignSelf> {
        match self.get(&super::flex::KEY_ALIGN_SELF).read() {
            Some(super::flex::AlignItems::Start) => Some(AlignSelf::Start),
            Some(super::flex::AlignItems::Center) => Some(AlignSelf::Center),
            Some(super::flex::AlignItems::End) => Some(AlignSelf::End),
            Some(super::flex::AlignItems::Stretch) => Some(AlignSelf::Stretch),
            Some(super::flex::AlignItems::Baseline) => Some(AlignSelf::Baseline),
            None => None,
        }
    }
}

pub struct ModifierAndFlexProps<'a>(pub &'a Modifier, pub &'a super::flex::FlexProps);

impl<'a> CoreStyle for ModifierAndFlexProps<'a> {
    type CustomIdent = Rc<str>;

    fn box_generation_mode(&self) -> BoxGenerationMode {
        self.0.box_generation_mode()
    }

    fn is_block(&self) -> bool {
        self.0.is_block()
    }

    fn is_compressible_replaced(&self) -> bool {
        self.0.is_compressible_replaced()
    }

    fn box_sizing(&self) -> BoxSizing {
        self.0.box_sizing()
    }

    fn direction(&self) -> Direction {
        self.0.direction()
    }

    fn overflow(&self) -> Point<Overflow> {
        self.0.overflow()
    }

    fn scrollbar_width(&self) -> f32 {
        self.0.scrollbar_width()
    }

    fn position(&self) -> Position {
        self.0.position()
    }

    fn inset(&self) -> Rect<LengthPercentageAuto> {
        self.0.inset()
    }

    fn size(&self) -> Size<Dimension> {
        self.0.size()
    }

    fn min_size(&self) -> Size<Dimension> {
        self.0.min_size()
    }

    fn max_size(&self) -> Size<Dimension> {
        self.0.max_size()
    }

    fn aspect_ratio(&self) -> Option<f32> {
        self.0.aspect_ratio()
    }

    fn margin(&self) -> Rect<LengthPercentageAuto> {
        self.0.margin()
    }

    fn padding(&self) -> Rect<LengthPercentage> {
        self.0.padding()
    }

    fn border(&self) -> Rect<LengthPercentage> {
        self.0.border()
    }
}

impl<'a> FlexboxContainerStyle for ModifierAndFlexProps<'a> {
    fn flex_direction(&self) -> FlexDirection {
        match self.1.direction {
            super::flex::FlexDirection::Row => FlexDirection::Row,
            super::flex::FlexDirection::RowReverse => FlexDirection::RowReverse,
            super::flex::FlexDirection::Column => FlexDirection::Column,
            super::flex::FlexDirection::ColumnReverse => FlexDirection::ColumnReverse,
        }
    }

    fn flex_wrap(&self) -> FlexWrap {
        match self.1.wrap {
            super::flex::FlexWrap::NoWrap => FlexWrap::NoWrap,
            super::flex::FlexWrap::Wrap => FlexWrap::Wrap,
            super::flex::FlexWrap::WrapReverse => FlexWrap::WrapReverse,
        }
    }

    fn gap(&self) -> Size<LengthPercentage> {
        Size {
            width: self.1.gap.into(),
            height: self.1.gap.into(),
        }
    }

    fn align_content(&self) -> Option<AlignContent> {
        Some(match self.1.align_content {
            super::flex::AlignContent::Stretch => AlignContent::Stretch,
            super::flex::AlignContent::Center => AlignContent::Center,
            super::flex::AlignContent::End => AlignContent::End,
            super::flex::AlignContent::SpaceBetween => AlignContent::SpaceBetween,
            super::flex::AlignContent::SpaceAround => AlignContent::SpaceAround,
            super::flex::AlignContent::Start => AlignContent::Start,
            super::flex::AlignContent::SpaceEvenly => AlignContent::SpaceEvenly,
        })
    }

    fn align_items(&self) -> Option<AlignItems> {
        Some(match self.1.align_items {
            super::flex::AlignItems::Stretch => AlignItems::Stretch,
            super::flex::AlignItems::Center => AlignItems::Center,
            super::flex::AlignItems::End => AlignItems::End,
            super::flex::AlignItems::Start => AlignItems::Start,
            super::flex::AlignItems::Baseline => AlignItems::Baseline,
        })
    }

    fn justify_content(&self) -> Option<JustifyContent> {
        Some(match self.1.justify_content {
            super::flex::JustifyContent::Start => JustifyContent::Start,
            super::flex::JustifyContent::Center => JustifyContent::Center,
            super::flex::JustifyContent::End => JustifyContent::End,
            super::flex::JustifyContent::SpaceEvenly => JustifyContent::SpaceEvenly,
            super::flex::JustifyContent::SpaceAround => JustifyContent::SpaceAround,
            super::flex::JustifyContent::SpaceBetween => JustifyContent::SpaceBetween,
        })
    }
}
