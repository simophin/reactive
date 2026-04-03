use crate::view_component::{GtkViewBuilder, GtkViewComponent, NoChildWidget};
use gtk4::prelude::*;
use reactive_core::{Signal, SignalExt};
use std::error::Error;
use std::sync::Arc;
use ui_core::Prop;
use ui_core::widgets::Image;

pub type ImageView = GtkViewComponent<gtk4::Picture, NoChildWidget>;

/// A decoded image, stored as the raw encoded bytes.
/// The actual `gdk::Texture` is created on the main thread when displayed.
#[derive(Clone)]
pub struct ImageHandle(pub Arc<[u8]>);

impl PartialEq for ImageHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ImageHandle {}

impl<'a> TryFrom<&'a [u8]> for ImageHandle {
    type Error = Box<dyn Error + Send + Sync>;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.is_empty() {
            return Err("empty image data".into());
        }
        Ok(ImageHandle(Arc::from(data)))
    }
}

pub static PROP_IMAGE: &Prop<ImageView, gtk4::Picture, ImageHandle> =
    &Prop::new(|picture, handle| {
        let bytes = gtk4::glib::Bytes::from_owned(handle.0.to_vec());
        match gtk4::gdk::Texture::from_bytes(&bytes) {
            Ok(texture) => picture.set_paintable(Some(&texture)),
            Err(e) => eprintln!("Failed to create texture: {e}"),
        }
    });

pub static PROP_ACCESSIBILITY_LABEL: &Prop<ImageView, gtk4::Picture, Option<String>> =
    &Prop::new(|picture, text| {
        picture.set_accessible_role(gtk4::AccessibleRole::Img);
        if let Some(text) = text {
            picture.update_property(&[gtk4::accessible::Property::Label(&text)]);
        }
    });

impl Image for ImageView {
    type NativeHandle = ImageHandle;

    fn new<S: Into<String>>(
        image: impl Signal<Value = ImageHandle> + 'static,
        desc: Option<impl Signal<Value = S> + 'static>,
    ) -> Self {
        Self(
            GtkViewBuilder::create_no_child(|_| gtk4::Picture::new(), |p| p.upcast())
                .bind(PROP_IMAGE, image)
                .bind(
                    PROP_ACCESSIBILITY_LABEL,
                    desc.map_value(|r| r.map(Into::into)),
                ),
        )
    }
}
