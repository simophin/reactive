use super::flex::invalidate_layout;
use crate::Prop;
use crate::widgets::{self, ImageCodec, NativeView};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_foundation::{NSData, NSString};
use objc2_ui_kit::{NSObjectUIAccessibility, UIImage, UIImageView, UIView, UIViewContentMode};
use reactive_core::{Signal, SignalExt};
use std::error::Error;

pub type Image = NativeView<Retained<UIView>, Retained<UIImageView>>;

/// A decoded `UIImage`. `UIImage` is immutable and thread-safe, so handles can
/// be decoded off the main thread. Equality is identity.
#[derive(Clone, Debug)]
pub struct ImageHandle(pub Retained<UIImage>);

impl PartialEq for ImageHandle {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&*self.0, &*other.0)
    }
}

impl Eq for ImageHandle {}

pub static PROP_IMAGE: Prop<Image, Retained<UIImageView>, ImageHandle> =
    Prop::new(|view, handle| {
        view.setImage(Some(&handle.0));
        invalidate_layout(view);
    });

pub static PROP_ACCESSIBILITY_LABEL: Prop<Image, Retained<UIImageView>, Option<String>> =
    Prop::new(|view, text| {
        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let text = text.map(|s| NSString::from_str(&s));
        view.setIsAccessibilityElement(text.is_some(), mtm);
        view.setAccessibilityLabel(text.as_deref(), mtm);
    });

impl widgets::Image for Image {
    type NativeHandle = ImageHandle;

    fn new<S: Into<String>>(
        image: impl Signal<Value = ImageHandle> + 'static,
        desc: Option<impl Signal<Value = S> + 'static>,
    ) -> Self {
        NativeView::from_parts(
            |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let view = UIImageView::new(mtm);
                view.setContentMode(UIViewContentMode::ScaleAspectFit);
                view
            },
            |view| view.into_super(),
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_IMAGE, image)
        .bind(
            PROP_ACCESSIBILITY_LABEL,
            desc.map_value(|s| s.map(Into::into)),
        )
    }
}

/// Decodes images with `UIImage(data:)`. Safe to call from any thread.
pub struct UIImageCodec;

impl UIImageCodec {
    fn decode(data: Retained<NSData>) -> Result<ImageHandle, Box<dyn Error + Send + Sync>> {
        UIImage::imageWithData(&data)
            .map(ImageHandle)
            .ok_or_else(|| "UIImage could not decode the image".into())
    }
}

impl ImageCodec for UIImageCodec {
    type NativeHandle = ImageHandle;

    fn decode_static(data: &'static [u8]) -> Result<ImageHandle, Box<dyn Error + Send + Sync>> {
        Self::decode(NSData::with_bytes(data))
    }

    fn decode_owned(data: Vec<u8>) -> Result<ImageHandle, Box<dyn Error + Send + Sync>> {
        Self::decode(NSData::from_vec(data))
    }
}
