use objc2::Message;
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use reactive_core::{Component, SetupContext};
use std::marker::PhantomData;
use ui_core::widgets::{Modifier, NativeView, WithModifier};

pub struct AppKitNativeView<V: 'static, Tag>(
    pub NativeView<Retained<NSView>, Retained<V>>,
    pub PhantomData<Tag>,
);

impl<V: Message + 'static, Tag> Component for AppKitNativeView<V, Tag> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        self.0.setup_in_component(ctx);
    }
}

impl<V: Message + 'static, Tag> WithModifier for AppKitNativeView<V, Tag> {
    fn modifier(self, modifier: Modifier) -> Self {
        Self(self.0.modifier(modifier), PhantomData)
    }
}
