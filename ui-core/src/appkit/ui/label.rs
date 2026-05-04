use crate::appkit::native::AppKitNativeView;
use apple::Prop;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSFont, NSTextAlignment, NSTextField};
use objc2_foundation::NSString;
use reactive_core::{Signal, SignalExt};
use ui_core::widgets::{NativeView, TextAlignment};

pub type Label = AppKitNativeView<NSTextField, ()>;

apple::view_props! {
    Label on NSTextField {
        stringValue: String;
        alignment: NSTextAlignment;
    }
}

pub static PROP_FONT_SIZE: Prop<Label, NSTextField, f64> = Prop::new(|view, size| {
    let font = NSFont::systemFontOfSize(size);
    view.setFont(Some(&font));
});

impl ui_core::widgets::Label for Label {
    fn new(text: impl Signal<Value = String> + 'static) -> Self {
        Self(
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
            .bind(PROP_STRINGVALUE, text),
            Default::default(),
        )
    }

    fn font_size(mut self, size: impl Signal<Value = f64> + 'static) -> Self {
        Self(self.0.bind(PROP_FONT_SIZE, size), Default::default())
    }

    fn alignment(mut self, alignment: impl Signal<Value = TextAlignment> + 'static) -> Self {
        Self(
            self.0.bind(
                PROP_ALIGNMENT,
                alignment.map_value(|align| match align {
                    TextAlignment::Leading => NSTextAlignment::Left,
                    TextAlignment::Center => NSTextAlignment::Center,
                    TextAlignment::Trailing => NSTextAlignment::Right,
                }),
            ),
            Default::default(),
        )
    }
}
