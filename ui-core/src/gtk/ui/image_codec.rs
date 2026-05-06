use super::image_view::ImageHandle;
use crate::widgets::ImageCodec;
use gtk4::gdk::Texture;
use gtk4::glib::Bytes;
use std::error::Error;

fn decode_bytes(bytes: Bytes) -> Result<ImageHandle, Box<dyn Error + Send + Sync>> {
    Texture::from_bytes(&bytes)
        .map(ImageHandle)
        .map_err(|e| Box::new(e) as Box<_>)
}

pub struct GtkImageCodec;

impl ImageCodec for GtkImageCodec {
    type NativeHandle = ImageHandle;

    fn decode_static(
        data: &'static [u8],
    ) -> Result<Self::NativeHandle, Box<dyn Error + Send + Sync>> {
        decode_bytes(Bytes::from_static(data))
    }

    fn decode_owned(data: Vec<u8>) -> Result<Self::NativeHandle, Box<dyn Error + Send + Sync>> {
        decode_bytes(Bytes::from_owned(data))
    }
}
