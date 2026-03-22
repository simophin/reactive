use apple::Prop;
use objc2_app_kit::{NSFont, NSTextAlignment, NSTextField};
use objc2_foundation::{MainThreadMarker, NSInteger, NSString};
use reactive_core::Signal;

use super::view_component::AppKitViewComponent;

pub type Text = AppKitViewComponent<NSTextField, ()>;

apple::view_props! {
    Text on NSTextField {
        string_value: String;
        alignment: NSTextAlignment;
        selectable: bool;
        maximum_number_of_lines: NSInteger;
    }
}

// font_size requires a NSFont conversion, so it can't be derived by view_props!
pub static PROP_FONT_SIZE: &Prop<Text, NSTextField, f64> = &Prop::new(|label, size| {
    label.setFont(Some(&NSFont::systemFontOfSize(size)));
});

impl Text {
    pub fn new_text(text: impl Signal<Value = String> + 'static) -> Self {
        let mut c = AppKitViewComponent::create(
            |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                NSTextField::labelWithString(&NSString::from_str(""), mtm)
            },
            |view| view.into_super().into_super(),
        );
        c.as_mut().bind(PROP_STRING_VALUE, text);
        c
    }
}
