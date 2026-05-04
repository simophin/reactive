use crate::widgets::{NativeView, TextAlignment};
use crate::{Prop, apple_view_props};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSFont, NSTextAlignment, NSTextField, NSView};
use objc2_foundation::NSString;
use reactive_core::{Signal, SignalExt};

pub type Label = NativeView<Retained<NSView>, Retained<NSTextField>>;

apple_view_props! {
    Label on NSTextField {
        stringValue: String;
        alignment: NSTextAlignment;
    }
}

pub static PROP_FONT_SIZE: Prop<Label, Retained<NSTextField>, f64> = Prop::new(|view, size| {
    let font = NSFont::systemFontOfSize(size);
    view.setFont(Some(&font));
});

impl crate::widgets::Label for Label {
    fn new(text: impl Signal<Value = String> + 'static) -> Self {
        NativeView::new(
            move |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let label = NSTextField::wrappingLabelWithString(&NSString::new(), mtm);
                label.setSelectable(false);
                label
            },
            |n| n.into_super().into_super(),
            move |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_STRINGVALUE, text)
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
