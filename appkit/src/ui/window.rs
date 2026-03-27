use crate::view_component::{AppKitViewBuilder, AppKitViewComponent, SingleChildView};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{Component, Signal};
use ui_core::widgets;

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "AppWindowDelegate"]
    struct AppWindowDelegate;

    unsafe impl NSObjectProtocol for AppWindowDelegate {}

    unsafe impl NSWindowDelegate for AppWindowDelegate {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _: &AnyObject) {
            crate::stop_app();
        }
    }
);

impl AppWindowDelegate {
    pub(super) fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send![Self::alloc(mtm), init] }
    }
}

pub type Window = AppKitViewComponent<NSWindow, SingleChildView>;

apple::view_props! {
    Window on NSWindow {
        pub title: String;
    }
}

impl widgets::Window for Window {
    fn new(
        title: impl Signal<Value = String> + 'static,
        child: impl Component + 'static,
        width: f64,
        height: f64,
    ) -> Self {
        Self(
            AppKitViewBuilder::create_with_child(
                move |_| {
                    let mtm = MainThreadMarker::new().expect("must be on main thread");
                    let delegate = AppWindowDelegate::new(mtm);
                    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
                    let style = NSWindowStyleMask::Titled
                        | NSWindowStyleMask::Closable
                        | NSWindowStyleMask::Resizable;
                    let window = unsafe {
                        NSWindow::initWithContentRect_styleMask_backing_defer(
                            mtm.alloc(),
                            rect,
                            style,
                            NSBackingStoreType::Buffered,
                            false,
                        )
                    };

                    window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
                    window.center();
                    window.makeKeyAndOrderFront(None);

                    window
                },
                |window| window.contentView().unwrap(),
                Box::new(child),
            )
            .bind(PROP_TITLE, title),
        )
    }
}
