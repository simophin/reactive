use objc2::rc::Retained;
use objc2::{MainThreadOnly, msg_send};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{BoxedComponent, IntoSignal, Signal};

use super::view_component::AppKitViewComponent;

pub type Stack = AppKitViewComponent<NSStackView, Vec<BoxedComponent>>;

apple::view_props! {
    Stack on NSStackView {
        orientation: NSUserInterfaceLayoutOrientation;
        spacing: f64;
    }
}

impl Stack {
    pub fn new_stack(
        orientation: impl Signal<Value = NSUserInterfaceLayoutOrientation> + 'static,
    ) -> Self {
        let mut c = AppKitViewComponent::create(
            |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let stack: Retained<NSStackView> =
                    unsafe { msg_send![NSStackView::alloc(mtm), init] };
                stack
            },
            |view| view.into_super(),
        );
        c.as_mut().bind(PROP_ORIENTATION, orientation);
        c
    }

    pub fn new_vertical_stack() -> Self {
        Self::new_stack(NSUserInterfaceLayoutOrientation::Vertical.into_signal())
    }

    pub fn new_horizontal_stack() -> Self {
        Self::new_stack(NSUserInterfaceLayoutOrientation::Horizontal.into_signal())
    }
}
