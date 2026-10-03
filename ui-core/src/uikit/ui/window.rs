//! On iOS a `Window` is a `UIWindow` for the connected `UIWindowScene`, with a
//! root view controller whose view hosts the content inside the safe area.
//! The initial size is ignored: the window always fills the scene.

use crate::widgets::{CommonModifiers, CommonWindow, EdgeInsets, Modifier, NativeViewRegistry};
use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSObjectProtocol, NSString};
use objc2_ui_kit::{UIColor, UIView, UIViewController, UIWindow};
use reactive_core::{Component, ComponentId, SetupContext, Signal, StoredSignal};
use std::cell::RefCell;
use std::rc::Rc;

pub type Window = CommonWindow<Retained<UIView>>;

define_class!(
    // SAFETY: UIView has no subclassing requirements and this class does not
    // implement Drop.
    #[unsafe(super(UIView))]
    #[thread_kind = MainThreadOnly]
    #[name = "ReactiveRootView"]
    #[ivars = RefCell<Option<(Retained<UIView>, EdgeInsets)>>]
    struct RootView;

    unsafe impl NSObjectProtocol for RootView {}

    impl RootView {
        #[unsafe(method(layoutSubviews))]
        fn layout_subviews(&self) {
            unsafe {
                let _: () = msg_send![super(self), layoutSubviews];
            }
            self.layout_content();
        }

        #[unsafe(method(safeAreaInsetsDidChange))]
        fn safe_area_insets_did_change(&self) {
            unsafe {
                let _: () = msg_send![super(self), safeAreaInsetsDidChange];
            }
            self.setNeedsLayout();
        }
    }
);

impl RootView {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(RefCell::new(None));
        unsafe { msg_send![super(this), init] }
    }

    fn set_content(&self, content: Option<(Retained<UIView>, EdgeInsets)>) {
        let old = self.ivars().replace(content.clone());
        if let Some((old, _)) = old
            && content.as_ref().is_none_or(|(new, _)| new != &old)
        {
            old.removeFromSuperview();
        }
        if let Some((view, _)) = &content {
            self.addSubview(view);
        }
        self.setNeedsLayout();
    }

    /// Fills the safe area, minus the content's padding modifier.
    fn layout_content(&self) {
        let Some((view, padding)) = self.ivars().borrow().clone() else {
            return;
        };

        let bounds = self.bounds();
        let safe = self.safeAreaInsets();
        let left = safe.left + padding.left as f64;
        let top = safe.top + padding.top as f64;
        let right = safe.right + padding.right as f64;
        let bottom = safe.bottom + padding.bottom as f64;

        view.setFrame(CGRect::new(
            CGPoint::new(bounds.origin.x + left, bounds.origin.y + top),
            CGSize::new(
                (bounds.size.width - left - right).max(0.0),
                (bounds.size.height - top - bottom).max(0.0),
            ),
        ));
    }
}

struct WindowViewRegistry {
    current_view: StoredSignal<Option<(Retained<UIView>, Modifier)>>,
}

impl NativeViewRegistry<Retained<UIView>> for WindowViewRegistry {
    fn update_view(&self, _component_id: ComponentId, view: Retained<UIView>, modifier: Modifier) {
        self.current_view.update(Some((view, modifier)));
    }

    fn clear_view(&self, _component_id: ComponentId, view: Retained<UIView>) {
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
            initial_size,
            ..
        } = *self;
        tracing::debug!(
            ?initial_size,
            "a UIWindow always fills its scene; size ignored"
        );

        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let scene = super::window_scene(ctx);

        let root_view = RootView::new(mtm);
        root_view.setBackgroundColor(Some(&UIColor::systemBackgroundColor()));

        let controller = UIViewController::new(mtm);
        controller.setView(Some(&root_view));

        let window = UIWindow::initWithWindowScene(UIWindow::alloc(mtm), &scene);
        window.setRootViewController(Some(&controller));
        window.makeKeyAndVisible();

        // The scene does not retain its windows; this component does.
        ctx.on_cleanup({
            let window = window.clone();
            move || {
                window.setHidden(true);
                window.setRootViewController(None);
            }
        });

        let current_view = ctx.create_signal(None);
        ctx.set_static_context(
            &super::VIEW_REGISTRY_KEY,
            Rc::new(WindowViewRegistry {
                current_view: current_view.clone(),
            }) as Rc<dyn NativeViewRegistry<_>>,
        );

        ctx.boxed_child(child);

        ctx.create_effect(move |_, _| {
            let content = current_view.read().map(|(view, modifier)| {
                let padding = modifier.get_paddings().read().unwrap_or_default();
                (view, padding)
            });
            root_view.set_content(content);
        });

        ctx.create_effect(move |_, _| {
            let title = NSString::from_str(&title.read());
            controller.setTitle(Some(&title));
            scene.setTitle(Some(&title));
        });
    }
}
