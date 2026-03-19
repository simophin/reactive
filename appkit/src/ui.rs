use objc2::rc::Retained;
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{Component, ContextKey, SetupContext, Signal};

/// Context key for the parent NSView that child components add themselves to.
static PARENT_VIEW: ContextKey<Retained<NSView>> = ContextKey::new();

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

pub struct Window {
    title: String,
    width: f64,
    height: f64,
    children: Vec<reactive_core::BoxedComponent>,
}

impl Window {
    pub fn new(title: impl Into<String>, width: f64, height: f64) -> Self {
        Self {
            title: title.into(),
            width,
            height,
            children: Vec::new(),
        }
    }

    pub fn child(mut self, component: impl Component + 'static) -> Self {
        self.children.push(Box::new(component));
        self
    }
}

impl Component for Window {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let mtm = MainThreadMarker::new().expect("must be on main thread");

        let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(self.width, self.height));
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

        window.setTitle(&NSString::from_str(&self.title));
        window.center();
        window.makeKeyAndOrderFront(None);

        if let Some(content_view) = window.contentView() {
            ctx.provide_context(&PARENT_VIEW, content_view);
        }

        ctx.on_cleanup(move || {
            let _ = window;
        });

        for child in self.children {
            let mut child_ctx = ctx.new_child();
            child.setup(&mut child_ctx);
        }
    }
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

pub struct Text {
    signal: Signal<String>,
}

impl Text {
    pub fn new(signal: Signal<String>) -> Self {
        Self { signal }
    }
}

impl Component for Text {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let mtm = MainThreadMarker::new().expect("must be on main thread");

        let label = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        label.setFont(Some(&NSFont::systemFontOfSize(24.0)));

        if let Some(parent_signal) = ctx.use_context(&PARENT_VIEW) {
            ctx.access(parent_signal, |parent_view: &Retained<NSView>| {
                let bounds = parent_view.bounds();
                label.setFrame(bounds);
                parent_view.addSubview(&label);
            });
        }

        let label_cleanup = label.clone();
        ctx.on_cleanup(move || {
            label_cleanup.removeFromSuperview();
        });

        let signal = self.signal;
        ctx.create_effect(move |ectx, _: Option<&mut ()>| {
            let text = ectx.read(signal);
            label.setStringValue(&NSString::from_str(&text));
        });
    }
}
