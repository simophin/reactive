use crate::Prop;
use crate::widgets::{Image, NativeView};
use gtk4::accessible::AccessibleExtManual;
use gtk4::gdk::Texture;
use gtk4::prelude::AccessibleExt;
use reactive_core::{Signal, SignalExt};

pub type ImageView = NativeView<gtk4::Window, gtk4::Picture>;

#[derive(Clone, PartialEq, Eq)]
pub struct ImageHandle(pub(super) Texture);

pub static PROP_IMAGE: Prop<ImageView, gtk4::Picture, ImageHandle> =
    Prop::new(|picture, handle| {
        picture.set_paintable(Some(&handle.0));
    });

pub static PROP_ACCESSIBILITY_LABEL: Prop<ImageView, gtk4::Picture, Option<String>> =
    Prop::new(|picture, text| {
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
        NativeView::new(
            |_| {
                let picture = gtk4::Picture::new();
                picture.set_can_shrink(true);
                picture.set_content_fit(gtk4::ContentFit::ScaleDown);
                picture
            },
            |p| p.upcast(),
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_IMAGE, image)
        .bind(
            PROP_ACCESSIBILITY_LABEL,
            desc.map_value(|r| r.map(Into::into)),
        )
    }
}
