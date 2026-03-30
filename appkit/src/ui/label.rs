use crate::view_component::{AppKitViewBuilder, AppKitViewComponent, NoChildView};
use apple::Prop;
use objc2_app_kit::{NSFont, NSTextAlignment, NSTextField};
use objc2_foundation::{MainThreadMarker, NSString};
use reactive_core::{Signal, SignalExt};
use ui_core::layout::types::TextAlignment;

pub type Label = AppKitViewComponent<NSTextField, NoChildView>;

apple::view_props! {
    Label on NSTextField {
        stringValue: String;
        alignment: NSTextAlignment;
    }
}

pub static PROP_FONT_SIZE: &Prop<Label, NSTextField, f64> = &Prop::new(|view, size| {
    let font = NSFont::systemFontOfSize(size);
    view.setFont(Some(&font));
});

impl ui_core::widgets::Label for Label {
    fn new(text: impl Signal<Value = String> + 'static) -> Self {
        Self(
            AppKitViewBuilder::create_no_child(
                |_| {
                    let mtm = MainThreadMarker::new().expect("must be on main thread");
                    let label = NSTextField::wrappingLabelWithString(&NSString::new(), mtm);
                    label.setSelectable(false);
                    label
                },
                |v| v.into_super().into_super(),
            )
            .bind(PROP_STRINGVALUE, text),
        )
    }

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self {
        Self(self.0.bind(PROP_FONT_SIZE, size))
    }

    fn alignment(self, alignment: impl Signal<Value = TextAlignment> + 'static) -> Self {
        Self(self.0.bind(
            PROP_ALIGNMENT,
            alignment.map_value(|a| match a {
                TextAlignment::Leading => NSTextAlignment::Left,
                TextAlignment::Center => NSTextAlignment::Center,
                TextAlignment::Trailing => NSTextAlignment::Right,
            }),
        ))
    }
}
