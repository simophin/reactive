use super::flex::invalidate_layout;
use crate::Prop;
use crate::widgets::{NativeView, TextAlignment};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_foundation::NSString;
use objc2_ui_kit::{NSTextAlignment, UIFont, UILabel, UIView};
use reactive_core::{Signal, SignalExt};

pub type Label = NativeView<Retained<UIView>, Retained<UILabel>>;

pub static PROP_TEXT: Prop<Label, Retained<UILabel>, String> = Prop::new(|view, text| {
    view.setText(Some(&NSString::from_str(&text)));
    invalidate_layout(view);
});

pub static PROP_FONT_SIZE: Prop<Label, Retained<UILabel>, f64> = Prop::new(|view, size| {
    let font = UIFont::systemFontOfSize(size);
    unsafe { view.setFont(Some(&font)) };
    invalidate_layout(view);
});

pub static PROP_ALIGNMENT: Prop<Label, Retained<UILabel>, NSTextAlignment> =
    Prop::new(|view, alignment| view.setTextAlignment(alignment));

impl crate::widgets::Label for Label {
    fn new(text: impl Signal<Value = String> + 'static) -> Self {
        NativeView::from_parts(
            |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let label = UILabel::new(mtm);
                // Wrap onto as many lines as the width given by the parent needs.
                label.setNumberOfLines(0);
                label
            },
            |label| label.into_super(),
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_TEXT, text)
    }

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self {
        self.bind(PROP_FONT_SIZE, size)
    }

    fn alignment(self, alignment: impl Signal<Value = TextAlignment> + 'static) -> Self {
        self.bind(
            PROP_ALIGNMENT,
            alignment.map_value(|align| match align {
                TextAlignment::Leading => NSTextAlignment::Left,
                TextAlignment::Center => NSTextAlignment::Center,
                TextAlignment::Trailing => NSTextAlignment::Right,
            }),
        )
    }
}
