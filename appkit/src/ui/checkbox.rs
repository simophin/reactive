use apple::Prop;
use objc2_app_kit::*;
use ui_core::widgets::NativeView;

pub type Checkbox = NativeView<NSButton, usize>;

apple::view_props! {
    Checkbox on NSButton {
        title: String;
        pub enabled: bool;
    }
}

pub static PROP_CHECKED: Prop<Checkbox, NSButton, bool> = Prop::new(|btn, checked| {
    btn.setState(if checked {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
});
