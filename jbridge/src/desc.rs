use std::{borrow::Cow, str::FromStr};

use combine::{Parser, Stream, StreamOnce};
use jni::signature::Primitive;

#[derive(Clone, Debug)]
pub enum JavaTypeDescription {
    Single(JavaSingularTypeDescription<Cow<'static, str>>),
    Array(JavaSingularTypeDescription<()>),
}

#[derive(Clone, Debug)]
pub enum JavaSingularTypeDescription<O> {
    Primitive(Primitive),
    String,
    Object(O),
}

impl FromStr for JavaTypeDescription {
    type Err = <&'static str as StreamOnce>::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (r, _) = parse_java_type().parse(s)?;
        Ok(r)
    }
}

#[derive(Clone, Debug)]
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
    use combine::many1;
    use combine::parser::char::char;

    many1::<Vec<_>, _, _>(char('['))
        .with(parse_singular_type())
        .map(|t| JavaTypeDescription::Array(t))
        .or(parse_singular_type().map(|t| JavaTypeDescription::Single(t)))
}

fn parse_primitive<Input>() -> impl Parser<Input, Output = Primitive>
where
    Input: Stream<Token = char>,
{
    use combine::choice;
    use combine::parser::char::char;

    choice((
        char('B').map(|_| Primitive::Byte),
        char('C').map(|_| Primitive::Char),
        char('D').map(|_| Primitive::Double),
        char('F').map(|_| Primitive::Float),
        char('I').map(|_| Primitive::Int),
        char('J').map(|_| Primitive::Long),
        char('S').map(|_| Primitive::Short),
        char('Z').map(|_| Primitive::Boolean),
        char('V').map(|_| Primitive::Void),
    ))
}

fn parse_singular_type<Input>(
) -> impl Parser<Input, Output = JavaSingularTypeDescription<Cow<'static, str>>>
where
    Input: Stream<Token = char>,
{
    use combine::parser::char::char;
    use combine::{between, many1, satisfy};

    parse_primitive()
        .map(JavaSingularTypeDescription::Primitive)
        .or(
            between(char('L'), char(';'), many1(satisfy(|c| c != ';'))).map(|s: String| {
                if s == "java/lang/String" {
                    JavaSingularTypeDescription::String
                } else {
                    JavaSingularTypeDescription::Object(Cow::Owned(s))
                }
            }),
        )
}

fn parse_array_element_type<Input>() -> impl Parser<Input, Output = JavaSingularTypeDescription<()>>
where
    Input: Stream<Token = char>,
{
    use combine::{many1, satisfy};

    parse_primitive()
        .map(JavaSingularTypeDescription::Primitive)
        .or(many1(satisfy(|c| c != ';')).map(|_s: String| ()))
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
