use crate::widgets::NativeView;
use crate::{Prop, apple_view_props};
use objc2::rc::Retained;
use objc2_app_kit::*;
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::CGImage;
use objc2_foundation::*;
use reactive_core::{Signal, SignalExt};

pub type ImageView = NativeView<Retained<NSView>, Retained<NSImageView>>;

apple_view_props! {
    ImageView on NSImageView {
        pub editable: bool;
        pub image_alignment: NSImageAlignment;
        pub image_scaling: NSImageScaling;
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ImageHandle(pub(super) CFRetained<CGImage>);

static PROP_IMAGE: Prop<ImageView, Retained<NSImageView>, ImageHandle> =
    Prop::new(|view, handle| {
        let mtm = MainThreadMarker::new().unwrap();
        let img = NSImage::initWithCGImage_size(mtm.alloc(), &handle.0, NSSize::ZERO);
        view.setImage(Some(&img));
    });

static PROP_ACCESSIBILITY_LABEL: Prop<ImageView, Retained<NSImageView>, Option<String>> =
    Prop::new(|view, text| {
        view.setAccessibilityLabel(
            text.map(|s| NSString::from_str(&s))
                .as_ref()
                .map(|s| s.as_ref()),
        );
    });

impl crate::widgets::Image for ImageView {
    type NativeHandle = ImageHandle;

    fn new<S: Into<String>>(
        image: impl Signal<Value = ImageHandle> + 'static,
        desc: Option<impl Signal<Value = S> + 'static>,
    ) -> Self {
        NativeView::new(
            move |_| NSImageView::new(MainThreadMarker::new().unwrap()),
            |view| view.into_super().into_super(),
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_IMAGE, image)
        .bind(
            PROP_ACCESSIBILITY_LABEL,
            desc.map_value(|s| s.map(|s| s.into())),
        )
    }
}
