use std::str::FromStr;

use combine::{Parser, Stream, StreamOnce};
use jni::signature::Primitive;

pub enum JavaTypeDescription {
    Primitive(Primitive),
    String,
    Object(String),
    Array(Box<JavaTypeDescription>),
}

impl FromStr for JavaTypeDescription {
    type Err = <&'static str as StreamOnce>::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (r, _) = parse_java_type().parse(s)?;
        Ok(r)
    }
}

pub struct JavaMethodDescription {
    pub arguments: Vec<JavaTypeDescription>,
    pub return_type: JavaTypeDescription,
}

impl FromStr for JavaMethodDescription {
    type Err = <&'static str as StreamOnce>::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (arguments, s) = parse_args().parse(s)?;
        let (return_type, _) = parse_java_type().parse(s)?;

        Ok(JavaMethodDescription {
            arguments,
            return_type,
        })
    }
}

fn parse_java_type<Input>() -> impl Parser<Input, Output = JavaTypeDescription>
where
    Input: Stream<Token = char>,
{
    use combine::parser::char::char;

    char('[')
        .with(parse_java_primitive_or_object())
        .map(|t| JavaTypeDescription::Array(Box::new(t)))
        .or(parse_java_primitive_or_object())
}

fn parse_java_primitive_or_object<Input>() -> impl Parser<Input, Output = JavaTypeDescription>
where
    Input: Stream<Token = char>,
{
    use combine::parser::char::char;
    use combine::{between, choice, many, satisfy};

    choice((
        char('B').map(|_| JavaTypeDescription::Primitive(Primitive::Byte)),
        char('C').map(|_| JavaTypeDescription::Primitive(Primitive::Char)),
        char('D').map(|_| JavaTypeDescription::Primitive(Primitive::Double)),
        char('F').map(|_| JavaTypeDescription::Primitive(Primitive::Float)),
        char('I').map(|_| JavaTypeDescription::Primitive(Primitive::Int)),
        char('J').map(|_| JavaTypeDescription::Primitive(Primitive::Long)),
        char('S').map(|_| JavaTypeDescription::Primitive(Primitive::Short)),
        char('Z').map(|_| JavaTypeDescription::Primitive(Primitive::Boolean)),
        char('V').map(|_| JavaTypeDescription::Primitive(Primitive::Void)),
        between(char('L'), char(';'), many(satisfy(|c| c != ';'))).map(|s| {
            if s == "java/lang/String" {
                JavaTypeDescription::String
            } else {
                JavaTypeDescription::Object(s)
            }
        }),
    ))
}

fn parse_args<Input>() -> impl Parser<Input, Output = Vec<JavaTypeDescription>>
where
    Input: Stream<Token = char>,
{
    use combine::parser::char::char;
    use combine::parser::repeat::many;
    use combine::parser::sequence::between;

    let arg = between(char('('), char(')'), many(parse_java_type()));

    arg
}
