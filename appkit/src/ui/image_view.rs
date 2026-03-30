use super::view_component::AppKitViewComponent;
use crate::view_component::{AppKitViewBuilder, NoChildView};
use apple::Prop;
use objc2_app_kit::*;
use objc2_core_foundation::{CFData, CFRetained};
use objc2_core_graphics::CGImage;
use objc2_foundation::*;
use objc2_image_io::CGImageSource;
use reactive_core::Signal;
use std::error::Error;
use thiserror::Error;

pub type ImageView = AppKitViewComponent<NSImageView, NoChildView>;

apple::view_props! {
    ImageView on NSImageView {
        pub editable: bool;
        pub image_alignment: NSImageAlignment;
        pub image_scaling: NSImageScaling;
    }
}

pub struct ImageHandle(CFRetained<CGImage>);

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

#[derive(Error, Debug)]
enum ImageDecodeError {
    #[error("failed to create CGImageSource from data")]
    CreateSource,
    #[error("failed to create CGImage from source")]
    CreateImage,
}

impl TryFrom<Vec<u8>> for ImageHandle {
    type Error = Box<dyn Error>;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let source = unsafe { CGImageSource::with_data(&CFData::from_bytes(&value), None) }
            .ok_or_else(|| Box::new(ImageDecodeError::CreateSource) as Box<dyn Error>)?;

        unsafe {
            Ok(Self(source.image_at_index(0, None).ok_or_else(|| {
                Box::new(ImageDecodeError::CreateImage) as Box<dyn Error>
            })?))
        }
    }
}

impl ui_core::widgets::Image for ImageView {
    type NativeHandle = ImageHandle;

    fn new(
        image: impl Signal<Value = Self::NativeHandle> + 'static,
        desc: Option<impl Signal<Value = String> + 'static>,
    ) -> Self {
        Self(
            AppKitViewBuilder::create_no_child(
                move |_| NSImageView::new(MainThreadMarker::new().unwrap()),
                |view| view.into_super().into_super(),
            )
            .bind(PROP_IMAGE, image)
            .bind(PROP_ACCESSIBILITY_LABEL, desc),
        )
    }
}
