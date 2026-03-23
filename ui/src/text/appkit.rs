use reactive_core::Signal;

pub type Text = appkit::Text;

impl super::TextComponent for Text {
    fn new(text: impl Signal<Value = String> + 'static) -> Self {
        Text::new_text(text)
    }
}
