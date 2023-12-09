use std::borrow::Cow;

use jni::signature::Primitive;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::char;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::IResult;
use Primitive::*;

use crate::{JavaArrayElementDescription, JavaMethodDescription, JavaTypeDescription};

pub fn parse_primitive(s: &str) -> IResult<&str, Primitive> {
    alt((
        map(tag("Z"), |_| Boolean),
        map(tag("B"), |_| Byte),
        map(tag("C"), |_| Char),
        map(tag("S"), |_| Short),
        map(tag("I"), |_| Int),
        map(tag("J"), |_| Long),
        map(tag("F"), |_| Float),
        map(tag("D"), |_| Double),
        map(tag("V"), |_| Void),
    ))(s)
}

pub fn parse_object(s: &str) -> IResult<&str, &str> {
    delimited(char('L'), take_until(";"), tag(";"))(s)
}

fn parse_array_element_primitive(s: &str) -> IResult<&str, JavaArrayElementDescription> {
    alt((
        map(tag("Z"), |_| JavaArrayElementDescription::Boolean),
        map(tag("B"), |_| JavaArrayElementDescription::Byte),
        map(tag("C"), |_| JavaArrayElementDescription::Char),
        map(tag("S"), |_| JavaArrayElementDescription::Short),
        map(tag("I"), |_| JavaArrayElementDescription::Int),
        map(tag("J"), |_| JavaArrayElementDescription::Long),
        map(tag("F"), |_| JavaArrayElementDescription::Float),
        map(tag("D"), |_| JavaArrayElementDescription::Double),
    ))(s)
}

pub fn parse_array_element(s: &str) -> IResult<&str, JavaArrayElementDescription> {
    let first_alpha_index =
        s.chars()
            .position(|c| c != '[')
            .ok_or(nom::Err::Error(nom::error::Error::new(
                s,
                nom::error::ErrorKind::Eof,
            )))?;

    alt((
        map(parse_array_element_primitive, move |p| {
            if first_alpha_index == 0 {
                p
            } else {
                JavaArrayElementDescription::ObjectLike {
                    signature: Cow::Borrowed(&s[..(first_alpha_index + 1)]),
                }
            }
        }),
        map(parse_object, move |class_name| {
            JavaArrayElementDescription::ObjectLike {
                signature: Cow::Borrowed(&s[..(first_alpha_index + class_name.len() + 2)]),
            }
        }),
    ))(&s[first_alpha_index..])
}

pub fn parse_java_type(s: &str) -> IResult<&str, JavaTypeDescription> {
    alt((
        map(parse_primitive, JavaTypeDescription::Primitive),
        map(parse_object, |s| {
            if s == "java/lang/String" {
                JavaTypeDescription::String
            } else {
                JavaTypeDescription::Object {
                    class_name: Cow::Borrowed(s),
                }
            }
        }),
        map(
            preceded(char('['), parse_array_element),
            JavaTypeDescription::Array,
        ),
    ))(s)
}

pub fn parse_java_method(s: &str) -> IResult<&str, JavaMethodDescription> {
    let (remaining, arguments) = delimited(tag("("), many0(parse_java_type), tag(")"))(s)?;
    let (remaining, return_type) = parse_java_type(remaining)?;
    Ok((
        remaining,
        JavaMethodDescription {
            arguments,
            return_type,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_primitive_works() {
        assert_eq!(parse_primitive("Z"), Ok(("", Boolean)));
        assert_eq!(parse_primitive("B"), Ok(("", Byte)));
        assert_eq!(parse_primitive("C"), Ok(("", Char)));
        assert_eq!(parse_primitive("S"), Ok(("", Short)));
        assert_eq!(parse_primitive("I"), Ok(("", Int)));
        assert_eq!(parse_primitive("J"), Ok(("", Long)));
        assert_eq!(parse_primitive("F"), Ok(("", Float)));
        assert_eq!(parse_primitive("D"), Ok(("", Double)));
    }

    #[test]
    fn parse_object_works() {
        assert_eq!(
            parse_object("Ljava/lang/String;"),
            Ok(("", "java/lang/String"))
        );
    }

    #[test]
    fn parse_array_element_works() {
        assert_eq!(
            parse_array_element("Ljava/lang/String;"),
            Ok((
                "",
                JavaArrayElementDescription::ObjectLike {
                    signature: Cow::Borrowed("Ljava/lang/String;")
                }
            ))
        );

        assert_eq!(
            parse_array_element("I"),
            Ok(("", JavaArrayElementDescription::Int))
        );

        assert_eq!(
            parse_array_element("[[[I"),
            Ok((
                "",
                JavaArrayElementDescription::ObjectLike {
                    signature: Cow::Borrowed("[[[I")
                }
            ))
        );

        assert_eq!(
            parse_array_element("[[[Ljava/lang/String;J"),
            Ok((
                "J",
                JavaArrayElementDescription::ObjectLike {
                    signature: Cow::Borrowed("[[[Ljava/lang/String;")
                }
            ))
        );
    }

    #[test]
    fn parse_java_type_works() {
        assert_eq!(
            parse_java_type("Ljava/lang/String;"),
            Ok(("", JavaTypeDescription::String))
        );

        assert_eq!(
            parse_java_type("[[[Ljava/lang/String;S"),
            Ok((
                "S",
                JavaTypeDescription::Array(JavaArrayElementDescription::ObjectLike {
                    signature: Cow::Borrowed("[[Ljava/lang/String;")
                })
            ))
        );

        assert_eq!(
            parse_java_type("[ZD"),
            Ok((
                "D",
                JavaTypeDescription::Array(JavaArrayElementDescription::Boolean)
            ))
        );

        assert_eq!(
            parse_java_type("JI"),
            Ok(("I", JavaTypeDescription::Primitive(Long)))
        );
    }

    #[test]
    fn parse_java_method_works() {
        assert_eq!(
            parse_java_method("(Ljava/lang/String;I)JZ"),
            Ok((
                "Z",
                JavaMethodDescription {
                    arguments: vec![
                        JavaTypeDescription::String,
                        JavaTypeDescription::Primitive(Int)
                    ],
                    return_type: JavaTypeDescription::Primitive(Long)
                }
            ))
        );
    }
}
