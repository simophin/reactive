use super::image_view::ImageHandle;
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::{CGDataProvider, CGImage};
use objc2_image_io::CGImageSource;
use std::error::Error;
use std::ffi::c_void;
use std::ptr::NonNull;
use thiserror::Error;
use ui_core::widgets::ImageCodec;

#[derive(Error, Debug)]
enum ImageDecodeError {
    #[error("failed to create CGImageSource from data")]
    CreateSource,
    #[error("failed to create CGImage from source")]
    CreateImage,
}

unsafe extern "C-unwind" fn release_vec(info: *mut c_void, _data: NonNull<c_void>, _size: usize) {
    drop(Box::from_raw(info as *mut Vec<u8>));
}

fn decode_with_provider(
    provider: CFRetained<CGDataProvider>,
) -> Result<ImageHandle, Box<dyn Error + Send + Sync>> {
    let source = unsafe { CGImageSource::with_data_provider(&provider, None) }
        .ok_or_else(|| Box::new(ImageDecodeError::CreateSource) as Box<_>)?;
    unsafe {
        source
            .image_at_index(0, None)
            .ok_or_else(|| Box::new(ImageDecodeError::CreateImage) as Box<_>)
            .map(ImageHandle)
    }
}

pub struct AppKitImageCodec;

impl ImageCodec for AppKitImageCodec {
    type NativeHandle = ImageHandle;

    fn decode_static(
        data: &'static [u8],
    ) -> Result<Self::NativeHandle, Box<dyn Error + Send + Sync>> {
        let provider = unsafe {
            CGDataProvider::with_data(std::ptr::null_mut(), data.as_ptr().cast(), data.len(), None)
        }
        .ok_or_else(|| Box::<dyn Error + Send + Sync>::from("failed to create CGDataProvider"))?;
        decode_with_provider(provider)
    }

    fn decode_owned(data: Vec<u8>) -> Result<Self::NativeHandle, Box<dyn Error + Send + Sync>> {
        let ptr = data.as_ptr().cast::<c_void>();
        let len = data.len();
        let info = Box::into_raw(Box::new(data)).cast::<c_void>();
        let provider = unsafe { CGDataProvider::with_data(info, ptr, len, Some(release_vec)) }
            .ok_or_else(|| {
                Box::<dyn Error + Send + Sync>::from("failed to create CGDataProvider")
            })?;
        decode_with_provider(provider)
    }
}
