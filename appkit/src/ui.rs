use std::ffi::c_void;

use objc2::ffi::{OBJC_ASSOCIATION_RETAIN_NONATOMIC, objc_setAssociatedObject};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{ClassType, DefinedClass, MainThreadOnly, define_class, msg_send, sel};
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

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Box<dyn Fn()>]
    #[name = "ActionTarget"]
    struct ActionTarget;

    unsafe impl NSObjectProtocol for ActionTarget {}

    impl ActionTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, _sender: &AnyObject) {
            self.ivars()();
        }
    }
);

// Unique address used as the association key.
static ACTION_TARGET_KEY: u8 = 0;

impl ActionTarget {
    fn new(callback: impl Fn() + 'static, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Box::new(callback) as Box<dyn Fn()>);
        unsafe { msg_send![super(this), init] }
    }

    fn as_object(&self) -> &AnyObject {
        self.as_super().as_super()
    }

    /// Returns a reference usable as a target, and attaches `self` to `owner`
    /// via an associated object (RETAIN policy). The owner now keeps the target
    /// alive — no external bookkeeping needed.
    fn attach_to(this: Retained<Self>, owner: &AnyObject) {
        let key = &ACTION_TARGET_KEY as *const u8 as *const c_void;
        let value = this.as_object() as *const AnyObject as *mut AnyObject;
        unsafe {
            objc_setAssociatedObject(
                owner as *const AnyObject as *mut AnyObject,
                key,
                value,
                OBJC_ASSOCIATION_RETAIN_NONATOMIC,
            );
        }
        // `self` drops here, releasing our +1.
        // The association's retain keeps the count at 1.
    }
}

// ---------------------------------------------------------------------------
// View context
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum ViewParent {
    Window(Retained<NSView>),
    Stack(Retained<NSStackView>),
}

impl ViewParent {
    fn add_child(&self, child: Retained<NSView>) {
        match self {
            ViewParent::Window(parent) => {
                child.setFrame(parent.bounds());
                parent.addSubview(&child);
            }
            ViewParent::Stack(stack) => {
                stack.addArrangedSubview(&child);
            }
        }
    }
}

static PARENT_VIEW: ContextKey<ViewParent> = ContextKey::new();

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
        ctx.provide_context(&PARENT_VIEW, ViewParent::Window(content_view.clone()));

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

    if let Some(parent) = ctx.use_context(&PARENT_VIEW) {
        parent.read().add_child(stack.clone().into_super());
    }
    ctx.provide_context(&PARENT_VIEW, ViewParent::Stack(stack.clone()));

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

        if let Some(parent) = ctx.use_context(&PARENT_VIEW) {
            parent.read().add_child(label.clone().into_super().into_super());
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
        let target = ActionTarget::new(self.on_click, mtm);

        let button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(&self.title),
                Some(target.as_object()),
                Some(sel!(performAction:)),
                mtm,
            )
        };

        // Transfer ownership of the target to the button via associated object.
        // When the button is deallocated, it releases the target automatically.
        ActionTarget::attach_to(target, button.as_super().as_super());

        if let Some(parent) = ctx.use_context(&PARENT_VIEW) {
            parent.read().add_child(button.clone().into_super().into_super());
        }

        // The button now owns the target via associated object.
        // We only need to keep the button alive.
        ctx.on_cleanup(move || {
            let _button = button;
        });
    }
}
