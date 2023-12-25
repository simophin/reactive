#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodDescription {
    pub name: String,
    pub signature: String,
    pub is_static: bool,
    pub argument_names: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

    fn get_package_and_name(&self) -> (Vec<String>, String) {
        let mut package = self
            .get_class_signature()
            .split('/')
            .map(|s| s.to_owned())
            .collect::<Vec<_>>();

        let name = package.pop().expect("a class name");

        (package, name)
    }
}
