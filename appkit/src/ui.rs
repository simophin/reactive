use objc2::rc::Retained;
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{Component, ContextKey, SetupContext, Signal};

/// Context key for the parent NSView that child components add themselves to.
static PARENT_VIEW: ContextKey<Option<Retained<NSView>>> = ContextKey::new();

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

        window.setTitle(&NSString::from_str(&self.title));
        window.center();
        window.makeKeyAndOrderFront(None);

        ctx.provide_context(&PARENT_VIEW, window.contentView());

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

pub struct Text<T> {
    text: T,
}

impl<T> Text<T> {
    pub fn new(text: T) -> Self {
        Self { text }
    }
}

impl<T: Signal<Value = String> + 'static> Component for Text<T> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let parent_view = ctx.use_context(&PARENT_VIEW);
        let current_view = ctx.provide_context(&PARENT_VIEW, None);

        ctx.on_cleanup({
            let current_view = current_view.clone();
            move || {
                let Some(v) = current_view.cloned() else {
                    return;
                };

                v.removeFromSuperview();
            }
        });

        let text = self.text;
        ctx.create_effect(move |_, _| {
            let value = text.cloned();

            let current_view: Retained<NSTextField> = match current_view.cloned() {
                Some(v) => v.downcast().ok().unwrap(),
                None => {
                    let label = NSTextField::labelWithString(&NSString::from_str(""), mtm);
                    label.setFont(Some(&NSFont::systemFontOfSize(24.0)));
                    current_view.update(|v| {
                        v.replace(label.clone().into_super().into_super());
                        false
                    });

                    label
                }
            };

            current_view.setStringValue(&NSString::from_str(&value));

            let parent_view = parent_view.as_ref().map(|s| s.cloned()).flatten();
            if parent_view.is_some() && !current_view.isDescendantOf(parent_view.as_ref().unwrap())
            {
                current_view.removeFromSuperview();
                let parent_view = parent_view.unwrap();
                let bounds = parent_view.bounds();
                current_view.setFrame(bounds);
                parent_view.addSubview(&current_view);
            }

            ()
        });
    }
}
