use crate::uikit::ui::view_component::{UIKitViewBuilder, UIKitViewComponent, NoChildView};
use apple::Prop;
use objc2_ui_kit::UILabel;
use objc2_foundation::{MainThreadMarker, NSString};
use reactive_core::{Signal, SignalExt};
use ui_core::layout::types::TextAlignment;
use objc2_ui_kit::NSTextAlignment;

pub type Label = UIKitViewComponent<<UUILabel, NoChildView>;

apple::view_props! {
    Label on UILabel {
        text: String;
        textAlignment: NSTextAlignment;
    }
}

impl ui_core::widgets::Label for Label {
    fn new(text: impl Signal<<ValueValue = String> + 'static) -> Self {
        Self(
            UIKitViewBuilder::create_no_child(
                |_| {
                    let mtm = MainThreadMarker::new().expect("must be on main thread");
                    UILabel::new(mtm)
                },
                |v| v,
            )
            .bind(PROP_TEXT, text),
        )
    }

    fn alignment(self, alignment: impl Signal<<ValueValue = TextAlignment> + 'static) -> Self {
        Self(self.0.bind(
            PROP_TEXTALIGNMENT,
            alignment.map_value(|a| match a {
                TextAlignment::Leading => NSTextAlignment::Left,
                TextAlignment::Center => NSTextAlignment::Center,
                TextAlignment::Trailing => NSTextAlignment::Right,
            }),
        ))
    }
}
