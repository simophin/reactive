use syn::Ident;

pub struct JavaBinding {
    pub class: JavaPath,
    pub fields: Vec<JavaField>,
    pub methods: Vec<JavaMethod>,
}

pub struct JavaField {
    pub ty: JavaType,
    pub name: Ident,
}

pub struct JavaMethod {
    pub return_ty: JavaType,
    pub name: Ident,
    pub args: Vec<JavaType>,
}

#[derive(Clone)]
pub struct JavaPath {
    pub segments: Vec<Ident>,
}

#[derive(Clone)]
pub enum JavaType {
    Void,
    Primitive(Ident),
    PrimitiveArray(Ident),
    String,
    Object(JavaPath),
}

impl JavaPath {
    pub fn jni_name(&self) -> String {
        self.segments
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("/")
    }

    pub fn last(&self) -> &Ident {
        self.segments
            .last()
            .expect("JavaPath parsing always has at least one segment")
    }
}
