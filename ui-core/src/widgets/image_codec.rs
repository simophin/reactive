use std::error::Error;

pub trait ImageCodec {
    type NativeHandle: Clone + Eq + Send + Sync + 'static;

    fn decode_static(data: &'static [u8]) -> Result<Self::NativeHandle, Box<dyn Error + Send + Sync>>;
    fn decode_owned(data: Vec<u8>) -> Result<Self::NativeHandle, Box<dyn Error + Send + Sync>>;
}
