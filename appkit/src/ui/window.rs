use crate::context::CHILDREN_VIEWS;
use crate::ui::layout::{MountedChild, activate_constraints, mount_child_to_parent, pin_edges};
use crate::view_component::AppKitViewBuilder;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
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

pub struct Window {
    child: BoxedComponent,
    title: Box<dyn Signal<Value = String>>,
    initial_width: f64,
    initial_height: f64,
}

// pub type Window = AppKitViewComponent<NSWindow, SingleChildView>;

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
        Self {
            child: Box::new(child),
            title: Box::new(title),
            initial_width: width,
            initial_height: height,
        }
    }
}

impl Component for Window {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            title,
            child,
            initial_width,
            initial_height,
        } = *self;

        let window = AppKitViewBuilder::create_with_child(
            move |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let delegate = AppWindowDelegate::new(mtm);
                let rect = NSRect::new(
                    NSPoint::new(0.0, 0.0),
                    NSSize::new(initial_width, initial_height),
                );
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
                pin_content_view_to_content_layout_guide(&window);
                window.center();
                window.makeKeyAndOrderFront(None);

                window
            },
            |window| window.contentView().unwrap(),
            child,
        )
        .bind(PROP_TITLE, title)
        .setup(ctx);

        let parent = window.contentView().unwrap();

        if let Some(children_view) = ctx.use_context(&CHILDREN_VIEWS) {
            ctx.create_effect(move |_, prev: Option<Option<MountedChild>>| {
                if let Some(child) = children_view.read().first() {
                    if let Some(Some(prev)) = prev {
                        prev.unmount(&parent);
                    }

                    if let Some(child_entry) = child.read() {
                        return Some(mount_child_to_parent(
                            &window.contentView().unwrap(),
                            child_entry,
                        ));
                    }
                }

                None
            });
        }
    }
}

fn pin_content_view_to_content_layout_guide(window: &NSWindow) {
    let Some(content_view) = window.contentView() else {
        return;
    };
    let Some(content_layout_guide) = window
        .contentLayoutGuide()
        .and_then(|guide| guide.downcast::<NSLayoutGuide>().ok())
    else {
        return;
    };

    content_view.setTranslatesAutoresizingMaskIntoConstraints(false);
    activate_constraints(&pin_edges(&content_view, &content_layout_guide));
}
