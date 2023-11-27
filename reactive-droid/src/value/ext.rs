use jni::objects::{JObject, JValueGen};

pub trait JValueGenExt<'a> {
    fn as_value_ref(&self) -> JValueGen<&JObject<'a>>;
}

impl<'a, S> JValueGenExt<'a> for JValueGen<S>
where
    S: AsRef<JObject<'a>>,
{
    fn as_value_ref(&self) -> JValueGen<&JObject<'a>> {
        match self {
            JValueGen::Object(v) => JValueGen::Object(v.as_ref()),
            JValueGen::Bool(v) => JValueGen::Bool(*v),
            JValueGen::Byte(v) => JValueGen::Byte(*v),
            JValueGen::Char(v) => JValueGen::Char(*v),
            JValueGen::Short(v) => JValueGen::Short(*v),
            JValueGen::Int(v) => JValueGen::Int(*v),
            JValueGen::Long(v) => JValueGen::Long(*v),
            JValueGen::Float(v) => JValueGen::Float(*v),
            JValueGen::Double(v) => JValueGen::Double(*v),
            JValueGen::Void => JValueGen::Void,
        }
    }
}
