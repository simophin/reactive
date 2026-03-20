use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{BoxedComponent, Component, ContextKey, SetupContext, Signal};

// ---------------------------------------------------------------------------
// ObjC helpers
// ---------------------------------------------------------------------------

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
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send![Self::alloc(mtm), init] }
    }
}

// Button callbacks keyed by the NSButton pointer (usize) so we don't need ObjC ivars.
thread_local! {
    static BUTTON_CALLBACKS: RefCell<HashMap<usize, Box<dyn Fn()>>> =
        RefCell::default();
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "AppButtonTarget"]
    struct AppButtonTarget;

    unsafe impl NSObjectProtocol for AppButtonTarget {}

    impl AppButtonTarget {
        #[unsafe(method(buttonClicked:))]
        fn button_clicked(&self, sender: &AnyObject) {
            let ptr = sender as *const AnyObject as usize;
            BUTTON_CALLBACKS.with(|map| {
                if let Some(cb) = map.borrow().get(&ptr) {
                    cb();
                }
            });
        }
    }
);

impl AppButtonTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send![Self::alloc(mtm), init] }
    }
}

// ---------------------------------------------------------------------------
// View context
// ---------------------------------------------------------------------------

/// A closure that adds an NSView to whatever parent is currently in scope.
/// Stack views provide `addArrangedSubview`; Window provides `addSubview`.
type ChildAdder = Rc<dyn Fn(Retained<NSView>) + 'static>;

static ADD_SUBVIEW: ContextKey<ChildAdder> = ContextKey::new();

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

pub struct Window {
    title: String,
    width: f64,
    height: f64,
    children: Vec<BoxedComponent>,
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

    pub fn child(mut self, c: impl Component + 'static) -> Self {
        self.children.push(Box::new(c));
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

        let delegate = AppWindowDelegate::new(mtm);
        window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        window.setTitle(&NSString::from_str(&self.title));
        window.center();
        window.makeKeyAndOrderFront(None);

        let content_view = window.contentView().unwrap();
        let adder: ChildAdder = Rc::new({
            let cv = content_view.clone();
            move |child: Retained<NSView>| {
                let bounds = cv.bounds();
                child.setFrame(bounds);
                cv.addSubview(&child);
            }
        });
        ctx.provide_context(&ADD_SUBVIEW, adder);

        ctx.on_cleanup(move || {
            let _ = delegate;
            let _ = window;
        });

        for child in self.children {
            let mut child_ctx = ctx.new_child();
            child.setup(&mut child_ctx);
        }
    }
}

// ---------------------------------------------------------------------------
// VStack / HStack
// ---------------------------------------------------------------------------

fn setup_stack(
    orientation: NSUserInterfaceLayoutOrientation,
    spacing: f64,
    children: Vec<BoxedComponent>,
    ctx: &mut SetupContext,
) {
    let mtm = MainThreadMarker::new().expect("must be on main thread");
    let stack: Retained<NSStackView> = unsafe { msg_send![NSStackView::alloc(mtm), init] };
    stack.setOrientation(orientation);
    stack.setSpacing(spacing);

    // Add ourselves to the parent.
    if let Some(adder) = ctx.use_context(&ADD_SUBVIEW) {
        adder.read()(stack.clone().into_super());
    }

    // Provide ourselves as the new parent for children.
    let adder: ChildAdder = Rc::new({
        let stack = stack.clone();
        move |child: Retained<NSView>| {
            stack.addArrangedSubview(&child);
        }
    });
    ctx.provide_context(&ADD_SUBVIEW, adder);

    ctx.on_cleanup(move || {
        let _ = stack;
    });

    for child in children {
        let mut child_ctx = ctx.new_child();
        child.setup(&mut child_ctx);
    }
}

pub struct VStack {
    spacing: f64,
    children: Vec<BoxedComponent>,
}

impl VStack {
    pub fn new() -> Self {
        Self {
            spacing: 8.0,
            children: Vec::new(),
        }
    }

    pub fn spacing(mut self, s: f64) -> Self {
        self.spacing = s;
        self
    }

    pub fn child(mut self, c: impl Component + 'static) -> Self {
        self.children.push(Box::new(c));
        self
    }
}

impl Component for VStack {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        setup_stack(
            NSUserInterfaceLayoutOrientation::Vertical,
            self.spacing,
            self.children,
            ctx,
        );
    }
}

pub struct HStack {
    spacing: f64,
    children: Vec<BoxedComponent>,
}

impl HStack {
    pub fn new() -> Self {
        Self {
            spacing: 8.0,
            children: Vec::new(),
        }
    }

    pub fn spacing(mut self, s: f64) -> Self {
        self.spacing = s;
        self
    }

    pub fn child(mut self, c: impl Component + 'static) -> Self {
        self.children.push(Box::new(c));
        self
    }
}

impl Component for HStack {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        setup_stack(
            NSUserInterfaceLayoutOrientation::Horizontal,
            self.spacing,
            self.children,
            ctx,
        );
    }
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

pub struct Text<S> {
    text: S,
    font_size: f64,
}

impl<S> Text<S> {
    pub fn new(text: S) -> Self {
        Self {
            text,
            font_size: 13.0,
        }
    }

    pub fn font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }
}

impl<S: Signal<Value = String> + 'static> Component for Text<S> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let label = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        label.setFont(Some(&NSFont::systemFontOfSize(self.font_size)));

        if let Some(adder) = ctx.use_context(&ADD_SUBVIEW) {
            adder.read()(label.clone().into_super().into_super());
        }

        let text = self.text;
        let label_ref = label.clone();
        ctx.create_effect(move |_, _: Option<()>| {
            label_ref.setStringValue(&NSString::from_str(&text.read()));
        });

        ctx.on_cleanup(move || {
            let _ = label;
        });
    }
}

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

pub struct Button<F> {
    title: String,
    on_click: F,
}

impl<F: Fn() + 'static> Button<F> {
    pub fn new(title: impl Into<String>, on_click: F) -> Self {
        Self {
            title: title.into(),
            on_click,
        }
    }
}

impl<F: Fn() + 'static> Component for Button<F> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let target = AppButtonTarget::new(mtm);

        // Cast Retained<AppButtonTarget> -> &AnyObject for NSButton's target parameter.
        let target_any: &AnyObject =
            unsafe { &*(&*target as *const AppButtonTarget as *const AnyObject) };

        let button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(&self.title),
                Some(target_any),
                Some(sel!(buttonClicked:)),
                mtm,
            )
        };

        let key = &*button as *const NSButton as usize;
        BUTTON_CALLBACKS.with(|map| {
            map.borrow_mut().insert(key, Box::new(self.on_click));
        });

        if let Some(adder) = ctx.use_context(&ADD_SUBVIEW) {
            adder.read()(button.clone().into_super().into_super());
        }

        ctx.on_cleanup(move || {
            BUTTON_CALLBACKS.with(|map| {
                map.borrow_mut().remove(&key);
            });
            let _ = target;
            let _ = button;
        });
    }
}
