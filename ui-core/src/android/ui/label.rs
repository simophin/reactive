use super::TextView;
use crate::Prop;
use crate::android::JavaObject;
use crate::widgets::{self, NativeView, TextAlignment};
use jni::objects::JValue;
use reactive_core::{Signal, SignalExt};

pub type Label = NativeView<JavaObject, TextView>;

pub static PROP_TEXT: Prop<Label, TextView, String> =
    Prop::new(|view, text| view.set_text("setText", &text));

/// Size in `sp`, so the user's font scale is respected.
pub static PROP_FONT_SIZE: Prop<Label, TextView, f64> =
    Prop::new(|view, size| view.call_void("setTextSize", "(F)V", &[JValue::Float(size as f32)]));

/// One of the `View.TEXT_ALIGNMENT_*` constants.
pub static PROP_TEXT_ALIGNMENT: Prop<Label, TextView, i32> = Prop::new(|view, alignment| {
    view.call_void("setTextAlignment", "(I)V", &[JValue::Int(alignment)])
});

const TEXT_ALIGNMENT_CENTER: i32 = 4;
const TEXT_ALIGNMENT_VIEW_START: i32 = 5;
const TEXT_ALIGNMENT_VIEW_END: i32 = 6;

impl widgets::Label for Label {
    fn new(text: impl Signal<Value = String> + 'static) -> Self {
        NativeView::from_parts(
            |ctx| TextView(super::new_view(ctx, "android/widget/TextView")),
            Into::into,
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
            PROP_TEXT_ALIGNMENT,
            alignment.map_value(|a| match a {
                TextAlignment::Leading => TEXT_ALIGNMENT_VIEW_START,
                TextAlignment::Center => TEXT_ALIGNMENT_CENTER,
                TextAlignment::Trailing => TEXT_ALIGNMENT_VIEW_END,
            }),
        )
    }
}
