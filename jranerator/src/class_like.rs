pub struct MethodDescription {
    pub name: String,
    pub signature: String,
    pub is_static: bool,
    pub argument_names: Option<Vec<String>>,
}

pub struct FieldDescription {
    pub name: String,
    pub signature: String,
    pub is_static: bool,
    pub is_final: bool,
}

pub trait ClassLike {
    fn get_class_signature(&self) -> String;
    fn get_public_methods(&self) -> Vec<MethodDescription>;
    fn get_public_fields(&self) -> Vec<FieldDescription>;
}
