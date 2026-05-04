use crate::apple_view_props;
use crate::widgets;
use crate::widgets::{CommonModifiers, EdgeInsets, Modifier, NativeViewRegistry};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{
    BoxedComponent, Component, ComponentId, IntoSignal, SetupContext, Signal, StoredSignal,
};
use std::rc::Rc;

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "AppWindowDelegate"]
    struct AppWindowDelegate;

    unsafe impl NSObjectProtocol for AppWindowDelegate {}

    unsafe impl NSWindowDelegate for AppWindowDelegate {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _: &AnyObject) {
            crate::appkit::stop_app();
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

apple_view_props! {
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

struct WindowViewRegistry {
    current_view: StoredSignal<Option<(Retained<NSView>, Modifier)>>,
}

impl NativeViewRegistry<Retained<NSView>> for WindowViewRegistry {
    fn update_view(&self, _component_id: ComponentId, view: Retained<NSView>, modifier: Modifier) {
        self.current_view.update(Some((view, modifier)));
    }

    fn clear_view(&self, _component_id: ComponentId, view: Retained<NSView>) {
        if self.current_view.read().as_ref().map(|s| &s.0) == Some(&view) {
            self.current_view.update(None);
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

        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let delegate = AppWindowDelegate::new(mtm);
        let rect = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(initial_width, initial_height),
        );
        let style =
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Resizable;
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

        let current_view = ctx.create_signal(None);
        let registry = WindowViewRegistry {
            current_view: current_view.clone(),
        };

        ctx.set_context(
            &super::VIEW_REGISTRY_KEY,
            (Rc::new(registry) as Rc<dyn NativeViewRegistry<_>>).into_signal(),
        );

        ctx.boxed_child(child);

        ctx.create_effect(move |_, _| {
            let current_view = current_view.read();
            let Some((current_view, modifier)) = current_view else {
                window.setContentView(None);
                return;
            };

            apply_content_view(
                &window,
                &current_view,
                modifier.get_paddings().read().unwrap_or_default(),
            );
        });
    }
}

fn apply_content_view(window: &NSWindow, view: &NSView, paddings: EdgeInsets) {
    window.setContentView(Some(view));
    let Some(content_layout_guide) = window
        .contentLayoutGuide()
        .and_then(|guide| guide.downcast::<NSLayoutGuide>().ok())
    else {
        return;
    };

    view.setTranslatesAutoresizingMaskIntoConstraints(false);

    let leading = view
        .leadingAnchor()
        .constraintEqualToAnchor(&content_layout_guide.leadingAnchor());
    leading.setConstant(paddings.left as f64);

    let trailing = view
        .trailingAnchor()
        .constraintEqualToAnchor(&content_layout_guide.trailingAnchor());
    trailing.setConstant(-(paddings.right as f64));

    let top = view
        .topAnchor()
        .constraintEqualToAnchor(&content_layout_guide.topAnchor());
    top.setConstant(paddings.top as f64);

    let bottom = view
        .bottomAnchor()
        .constraintEqualToAnchor(&content_layout_guide.bottomAnchor());
    bottom.setConstant(-(paddings.bottom as f64));

    let constraints = NSArray::from_retained_slice(&[leading, trailing, top, bottom]);
    NSLayoutConstraint::activateConstraints(&constraints);
}
