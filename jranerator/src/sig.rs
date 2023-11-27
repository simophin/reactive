use combine::{Parser, Stream};
use jni::signature::Primitive;

pub enum JavaType {
    Primitive(Primitive),
    String,
    Object(String),
    Array(Box<JavaType>),
}

pub struct JavaSignature {
    pub arguments: Vec<JavaType>,
    pub return_type: JavaType,
}

fn parse_java_type<Input>() -> impl Parser<Input, Output = JavaType>
where
    Input: Stream<Token = char>,
{
    use combine::choice;
    use combine::parser::char::char;
    use combine::parser::char::string;

    choice((
        char('B').map(|_| JavaType::Primitive(Primitive::Byte)),
        char('C').map(|_| JavaType::Primitive(Primitive::Char)),
        char('D').map(|_| JavaType::Primitive(Primitive::Double)),
        char('F').map(|_| JavaType::Primitive(Primitive::Float)),
        char('I').map(|_| JavaType::Primitive(Primitive::Int)),
        char('J').map(|_| JavaType::Primitive(Primitive::Long)),
        char('S').map(|_| JavaType::Primitive(Primitive::Short)),
        char('Z').map(|_| JavaType::Primitive(Primitive::Boolean)),
        char('V').map(|_| JavaType::Primitive(Primitive::Void)),
        string("Ljava/lang/String;").map(|_| JavaType::String),
        char('L').with(parse_object),
        char('[').with(parse_array),
    ))
}

fn parse_args<Input>() -> impl Parser<Input, Output = Vec<JavaType>>
where
    Input: Stream<Token = char>,
{
    use combine::parser::char::char;
    use combine::parser::repeat::many;
    use combine::parser::sequence::between;

    let arg = between(char('('), char(')'), many(parse_java_type()));

    arg
}
