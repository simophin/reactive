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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassSignature(pub String);

impl ClassSignature {
    pub fn packages(&self) -> Vec<&str> {
        let mut segments: Vec<_> = self.0.split('/').collect();
        if segments.len() > 0 {
            segments.pop();
        }

        segments
    }

    pub fn name(&self) -> &str {
        match self.0.split('/').last() {
            Some(name) => name,
            None => &self.0,
        }
    }
}

pub trait ClassLike {
    fn get_class_signature(&self) -> ClassSignature;
    fn get_public_methods(&self) -> Vec<MethodDescription>;
    fn get_public_fields(&self) -> Vec<FieldDescription>;
    fn get_superclass(&self) -> Option<ClassSignature>;
    fn get_interfaces(&self) -> Vec<ClassSignature>;
}
