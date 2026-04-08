use crate::context::{CHILD_VIEW, CHILDREN_VIEWS};
use apple::Prop;
use objc2::rc::Retained;
use objc2::{Message, msg_send};
use objc2_foundation::NSString;
use objc2_ui_kit::UIView;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use std::any::type_name;
use ui_core::{AtMostOneChild, MultipleChildren, NoChild, PlatformViewBuilder, SingleChild};

pub use ui_core::{
    AtMostOneChild as AtMostOneChildView, ChildStrategy as ChildViewStrategy,
    MultipleChildren as MultipleChildView, NoChild as NoChildView, SingleChild as SingleChildView,
};

pub struct UIKitViewBuilder<<VV, C> {
    inner: PlatformViewBuilder<<RetRetained<<VV>, Retained<<UIViewUIView>, C>,
    debug_identifier: Option<<StringString>,
}

impl<<VV: Message + 'static> UIKitViewBuilder<<VV, NoChild> {
    pub fn create_no_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<<VV> + 'static,
        into_uiview: fn(Retained<<VV>) -> Retained<<UIViewUIView>,
    ) -> Self {
        let mut inner = PlatformViewBuilder::create_no_child(
            creator,
            into_uiview,
            &CHILD_VIEW,
            &CHILDREN_VIEWS,
        );
        set_layout_after_set(&mut inner, into_uiview);
        Self {
            inner,
            debug_identifier: None,
        }
    }
}

impl<<VV: Message + 'static> UIKitViewBuilder<<VV, SingleChild> {
    pub fn create_with_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<<VV> + 'static,
        into_uiview: fn(Retained<<VV>) -> Retained<<UIViewUIView>,
        child: BoxedComponent,
    ) -> Self {
        let mut inner = PlatformViewBuilder::create_with_child(
            creator,
            into_uiview,
            &CHILD_VIEW,
            &CHILDREN_VIEWS,
            child,
        );
        set_layout_after_set(&mut inner, into_uiview);
        Self {
            inner,
            debug_identifier: None,
        }
    }
}

impl<<VV: Message + 'static> UIKitViewBuilder<<VV, MultipleChildren> {
    pub fn create_multiple_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<<VV> + 'static,
        into_uiview: fn(Retained<<VV>) -> Retained<<UIViewUIView>,
    ) -> Self {
        let mut inner = PlatformViewBuilder::create_multiple_child(
            creator,
            into_uiview,
            &CHILD_VIEW,
            &CHILDREN_VIEWS,
        );
        set_layout_after_set(&mut inner, into_uiview);
        Self {
            inner,
            debug_identifier: None,
        }
    }

    pub fn add_child(mut self, c: BoxedComponent) -> Self {
        self.inner = self.inner.add_child(c);
        self
    }
}

impl<<VV: Message + 'static> UIKitViewBuilder<<VV, AtMostOneChild> {
    pub fn create_with_optional_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<<VV> + 'static,
        into_uiview: fn(Retained<<VV>) -> Retained<<UIViewUIView>,
        child: Option<<BoxBoxedComponent>,
    ) -> Self {
        let mut inner = PlatformViewBuilder::create_with_optional_child(
            creator,
            into_uiview,
            &CHILD_VIEW,
            &CHILDREN_VIEWS,
            child,
        );
        set_layout_after_set(&mut inner, into_uiview);
        Self {
            inner,
            debug_identifier: None,
        }
    }
}

impl<<VV: 'static, C> UIKitViewBuilder<<VV, C> {
    pub fn debug_identifier(mut self, identifier: impl Into<<StringString>) -> Self {
        self.debug_identifier = Some(identifier.into());
        self
    }

    pub fn bind<<TT, ValueType>(
        mut self,
        prop: &'static Prop<<TT, V, ValueType>,
        value: impl Signal<<ValueValue = ValueType> + 'static,
    ) -> Self
    where
        V: Message,
        ValueType: 'static,
    {
        self.inner = self.inner.bind(prop, value);
        self
    }

    pub fn setup(self, ctx: &mut SetupContext) -> Retained<<VV>
    where
        V: Message,
        C: ChildViewStrategy,
    {
        let UIKitViewBuilder {
            inner,
            debug_identifier,
        } = self;
        inner.setup(
            ctx,
            |uiview| {
                uiview.setTranslatesAutoresizingMaskIntoConstraints(false);
                let name = debug_identifier.unwrap_or_else(|| short_type_name::<<VV>().to_string());
                unsafe {
                    let _: () = msg_send![&*uiview, setAccessibilityIdentifier: Some(&NSString::from_str(&name))];
                }
            },
            |native, layout| ui_core::ChildEntry { native, layout },
        )
    }
}

fn short_type_name<<TT>() -> &'static str {
    type_name::<<TT>().rsplit("::").next().unwrap_or("UIView")
}

fn set_layout_after_set<<VV: Message + 'static, C>(
    inner: &mut PlatformViewBuilder<<RetRetained<<VV>, Retained<<UIViewUIView>, C>,
    into_uiview: fn(Retained<<VV>) -> Retained<<UIViewUIView>,
) {
    inner.set_after_set(move |view: &Retained<<VV>| {
        let uiview = into_uiview(view.clone());
        unsafe {
            if let Some(sv) = uiview.superview() {
                let _: () = msg_send![&*sv, setNeedsLayout];
            }
        }
    });
}

pub struct UIKitViewComponent<<VV, C>(pub UIKitViewBuilder<<VV, C>);

impl<<VV: Message + 'static, C: ChildViewStrategy> Component for UIKitViewComponent<<VV, C> {
    fn setup(self: Box<<SelfSelf>, ctx: &mut SetupContext) {
        self.0.setup(ctx);
    }
}
