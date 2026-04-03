use super::view_component::AppKitViewComponent;
use crate::view_component::{AppKitViewBuilder, NoChildView};
use apple::Prop;
use objc2_app_kit::*;
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::CGImage;
use objc2_foundation::*;
use reactive_core::{Signal, SignalExt};
use ui_core::widgets::Image;

pub type ImageView = AppKitViewComponent<NSImageView, NoChildView>;

apple::view_props! {
    ImageView on NSImageView {
        pub editable: bool;
        pub image_alignment: NSImageAlignment;
        pub image_scaling: NSImageScaling;
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ImageHandle(pub(super) CFRetained<CGImage>);

static PROP_IMAGE: &Prop<ImageView, NSImageView, ImageHandle> = &Prop::new(|view, handle| {
    let mtm = MainThreadMarker::new().unwrap();
    let img = NSImage::initWithCGImage_size(mtm.alloc(), &handle.0, NSSize::ZERO);
    view.setImage(Some(&img));
});

static PROP_ACCESSIBILITY_LABEL: &Prop<ImageView, NSImageView, Option<String>> =
    &Prop::new(|view, text| {
        view.setAccessibilityLabel(
            text.map(|s| NSString::from_str(&s))
                .as_ref()
                .map(|s| s.as_ref()),
        );
    });

impl Image for ImageView {
    type NativeHandle = ImageHandle;

    fn new<S: Into<String>>(
        image: impl Signal<Value = ImageHandle> + 'static,
        desc: Option<impl Signal<Value = S> + 'static>,
    ) -> Self {
        Self(
            AppKitViewBuilder::create_no_child(
                move |_| NSImageView::new(MainThreadMarker::new().unwrap()),
                |view| view.into_super().into_super(),
            )
            .bind(PROP_IMAGE, image)
            .bind(
                PROP_ACCESSIBILITY_LABEL,
                desc.map_value(|r| r.map(Into::into)),
            ),
        )
    }
}
