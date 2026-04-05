use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, Token,
};

use super::ast::{JavaBinding, JavaField, JavaMethod, JavaPath, JavaType};

enum JavaMember {
    Field(JavaField),
    Method(JavaMethod),
}

impl Parse for JavaBinding {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let kw: Ident = input.parse()?;
        if kw != "class" {
            return Err(syn::Error::new(kw.span(), "expected `class`"));
        }

        let class = input.parse::<JavaPath>()?;
        let content;
        braced!(content in input);

        let mut fields = Vec::new();
        let mut methods = Vec::new();
        while !content.is_empty() {
            match content.parse::<JavaMember>()? {
                JavaMember::Field(field) => fields.push(field),
                JavaMember::Method(method) => methods.push(method),
            }
        }

        Ok(Self {
            class,
            fields,
            methods,
        })
    }
}

impl Parse for JavaMember {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ty = input.parse::<JavaType>()?;
        let name = input.parse::<Ident>()?;

        if input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            let args = Punctuated::<JavaType, Token![,]>::parse_terminated(&content)?
                .into_iter()
                .collect();

            input.parse::<Token![;]>()?;

            Ok(Self::Method(JavaMethod {
                return_ty: ty,
                name,
                args,
            }))
        } else {
            input.parse::<Token![;]>()?;
            Ok(Self::Field(JavaField { ty, name }))
        }
    }
}

impl Parse for JavaPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut segments = vec![input.parse()?];
        while input.peek(Token![.]) {
            input.parse::<Token![.]>()?;
            segments.push(input.parse()?);
        }

        Ok(Self { segments })
    }
}

impl Parse for JavaType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse::<JavaPath>()?;
        let first = path
            .segments
            .first()
            .expect("JavaType parsing always has at least one segment")
            .clone();

        if path.segments.len() == 1 {
            let ty = match first.to_string().as_str() {
                "void" => Self::Void,
                "boolean" | "byte" | "char" | "short" | "int" | "long" | "float" | "double" => {
                    Self::Primitive(first.clone())
                }
                "String" => Self::String,
                _ => Self::Object(path),
            };

            if input.peek(syn::token::Bracket) {
                let content;
                bracketed!(content in input);
                if !content.is_empty() {
                    return Err(syn::Error::new(content.span(), "expected `[]`"));
                }
                return match ty {
                    Self::Primitive(ident) => Ok(Self::PrimitiveArray(ident)),
                    _ => Err(syn::Error::new(
                        first.span(),
                        "only primitive arrays are supported in declare_jni_binding fields",
                    )),
                };
            }

            return Ok(ty);
        }

        Ok(Self::Object(path))
    }
}
