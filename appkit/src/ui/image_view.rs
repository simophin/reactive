use apple::Prop;
use objc2::rc::Retained;
use objc2::{MainThreadOnly, msg_send};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::Signal;

use super::view_component::AppKitViewComponent;

pub type ImageView = AppKitViewComponent<NSImageView, ()>;

apple::view_props! {
    ImageView on NSImageView {
        pub editable: bool;
        pub image_alignment: NSImageAlignment;
        pub image_scaling: NSImageScaling;
    }
}

/// Loads a named image (bundle asset or AppKit built-in) and sets it.
/// Uses a custom Prop because the setter takes `Option<&NSImage>`.
pub static PROP_IMAGE_NAME: &Prop<ImageView, NSImageView, String> = &Prop::new(|iv, name| {
    let ns_name = NSString::from_str(&name);
    let image = NSImage::imageNamed(&ns_name);
    iv.setImage(image.as_deref());
});

impl ImageView {
    pub fn new_image(name: impl Signal<Value = String> + 'static) -> Self {
        let mut c = AppKitViewComponent::create(
            |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let iv: Retained<NSImageView> = unsafe { msg_send![NSImageView::alloc(mtm), init] };
                iv
            },
            |view: Retained<NSImageView>| view.into_super().into_super(),
        );
        c.as_mut().bind(PROP_IMAGE_NAME, name);
        c
    }
}
