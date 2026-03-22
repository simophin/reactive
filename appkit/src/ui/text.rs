use apple::Prop;
use apple::ViewBuilder;
use apple::bindable::BindableView;
use objc2_app_kit::{NSFont, NSTextAlignment, NSTextField};
use objc2_foundation::{MainThreadMarker, NSInteger, NSString};
use reactive_core::{Component, SetupContext, Signal};

use super::context::PARENT_VIEW;

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

pub struct Text {
    builder: ViewBuilder<NSTextField>,
}

impl BindableView<NSTextField> for Text {
    fn get_builder(&mut self) -> &mut ViewBuilder<NSTextField> {
        &mut self.builder
    }
}

impl Text {
    pub fn new(text: impl Signal<Value = String> + 'static) -> Self {
        let mut builder = ViewBuilder::new(|_| {
            let mtm = MainThreadMarker::new().expect("must be on main thread");
            NSTextField::labelWithString(&NSString::from_str(""), mtm)
        });
        builder.bind(PROP_STRING_VALUE, text);
        Self { builder }
    }
}

impl Component for Text {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let label = self.builder.setup(ctx);

        if let Some(parent) = ctx.use_context(&PARENT_VIEW) {
            parent
                .read()
                .add_child(label.clone().into_super().into_super());
        }

        ctx.on_cleanup(move || {
            let _ = label;
        });
    }
}
