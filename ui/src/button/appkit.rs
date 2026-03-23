use reactive_core::Signal;

pub use appkit::Button;

impl super::ButtonComponent for Button {
    fn new(title: impl Signal<Value = String> + 'static, on_click: impl Fn() + 'static) -> Self {
        Button::new_button(title, on_click)
    }
}
