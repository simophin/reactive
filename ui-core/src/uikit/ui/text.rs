use objc2_foundation::{MainThreadMarker, NSString};
use objc2_ui_kit::{UIFont, UILabel};
use reactive_core::{Component, SetupContext, Signal};

use super::context::PARENT_VIEW;

pub struct Text<S> {
    text: S,
    font_size: f64,
}

impl<S> Text<S> {
    pub fn new(text: S) -> Self {
        Self {
            text,
            font_size: 13.0,
        }
    }

    pub fn font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }
}

impl<S: Signal<Value = String> + 'static> Component for Text<S> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let label = UILabel::new(mtm);
        label.setFont(Some(&UIFont::systemFontOfSize(self.font_size)));

        if let Some(parent) = ctx.use_context(&PARENT_VIEW) {
            parent
                .read()
                .add_child(label.clone().into_super().into_super());
        }

        let text = self.text;
        let label_ref = label.clone();
        ctx.create_effect(move |_, _: Option<()>| {
            label_ref.setText(Some(&NSString::from_str(&text.read())));
        });

        ctx.on_cleanup(move || {
            let _ = label;
        });
    }
}
