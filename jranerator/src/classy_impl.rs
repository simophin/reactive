use classy::{Attribute, ClassFile, ACC_FINAL, ACC_PUBLIC, ACC_STATIC, ACC_SYNTHETIC};

use crate::class_like::{ClassLike, ClassSignature, FieldDescription, MethodDescription};

impl ClassLike for ClassFile {
    fn get_public_methods(&self) -> Vec<MethodDescription> {
        self.method_info
            .iter()
            .filter(|&m| m.access_flags & ACC_PUBLIC != 0 && m.access_flags & ACC_SYNTHETIC == 0)
            .map(|m| MethodDescription {
                name: self
                    .get_constant_utf8(m.name_index)
                    .expect("a method name")
                    .to_owned(),
                signature: self
                    .get_constant_utf8(m.descriptor_index)
                    .expect("a signature")
                    .to_owned(),
                is_static: m.access_flags & ACC_STATIC != 0,
                argument_names: m
                    .attributes
                    .iter()
                    .find_map(|x| match x {
                        Attribute::MethodParameters(params) => Some(params),
                        _ => None,
                    })
                    .map(|params| {
                        params
                            .iter()
                            .map(|(name_index, _)| {
                                self.get_constant_utf8(*name_index)
                                    .expect("a parameter name")
                                    .to_owned()
                            })
                            .collect()
                    }),
            })
            .filter(|m| !m.name.starts_with("access$"))
            .collect()
    }

    fn get_public_fields(&self) -> Vec<FieldDescription> {
        self.field_info
            .iter()
            .filter(|&f| f.access_flags & ACC_PUBLIC != 0)
            .map(|f| FieldDescription {
                name: self
                    .get_constant_utf8(f.name_index)
                    .expect("a field name")
                    .to_owned(),
                signature: self
                    .get_constant_utf8(f.descriptor_index)
                    .expect("a signature")
                    .to_owned(),
                is_static: f.access_flags & ACC_STATIC != 0,
                is_final: f.access_flags & ACC_FINAL != 0,
            })
            .collect()
    }

    fn get_class_signature(&self) -> ClassSignature {
        match &self.constant_pool[self.this_class as usize - 1] {
            classy::Constant::ClassInfo { name_index, .. } => ClassSignature(
                self.get_constant_utf8(*name_index)
                    .expect("a class signature")
                    .to_owned(),
            ),
            v => panic!("Unexpected class info class signature: {v:?}"),
        }
    }

    fn get_superclass(&self) -> Option<ClassSignature> {
        self.get_constant_utf8(self.super_class)
            .map(|s| ClassSignature(s.to_owned()))
            .ok()
    }

    fn get_interfaces(&self) -> Vec<ClassSignature> {
        self.interfaces
            .iter()
            .map(|&i| {
                self.get_constant_utf8(i)
                    .map(|s| ClassSignature(s.to_owned()))
                    .expect("an interface signature")
            })
            .collect()
    }
}
