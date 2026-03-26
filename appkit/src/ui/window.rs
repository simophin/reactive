use apple::ViewBuilder;
use apple::bindable::BindableView;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};

use super::context::{PARENT_VIEW, ViewParent};

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

pub struct Window {
    builder: ViewBuilder<NSWindow>,
    children: Vec<BoxedComponent>,
}

apple::view_props! {
    Window on NSWindow {
        pub title: String;
    }
}

impl AsMut<ViewBuilder<NSWindow>> for Window {
    fn as_mut(&mut self) -> &mut ViewBuilder<NSWindow> {
        &mut self.builder
    }
}

impl BindableView<NSWindow> for Window {}

impl Window {
    pub fn new(title: impl Signal<Value = String> + 'static, width: f64, height: f64) -> Self {
        let mut w = Self {
            builder: ViewBuilder::new(move |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
                let style = NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Resizable;
                unsafe {
                    NSWindow::initWithContentRect_styleMask_backing_defer(
                        mtm.alloc(),
                        rect,
                        style,
                        NSBackingStoreType::Buffered,
                        false,
                    )
                }
            }),
            children: Vec::new(),
        };
        w.as_mut().bind(PROP_TITLE, title);
        w
    }

    pub fn child(mut self, c: impl Component + 'static) -> Self {
        self.children.push(Box::new(c));
        self
    }
}

impl Component for Window {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Window { builder, children } = *self;

        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let delegate = AppWindowDelegate::new(mtm);

        let window = builder.setup(ctx);

        window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        window.center();
        window.makeKeyAndOrderFront(None);

        let content_view = window.contentView().unwrap();
        ctx.provide_context(&PARENT_VIEW, ViewParent::Window(content_view.clone()));

        ctx.on_cleanup(move || {
            let _ = delegate;
            let _ = window;
        });

        for child in children {
            let mut child_ctx = ctx.new_child();
            child.setup(&mut child_ctx);
        }
    }
}
