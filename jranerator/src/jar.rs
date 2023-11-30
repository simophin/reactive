use std::{
    fs::File,
    io::{Read, Seek},
    path::Path,
};

use classy::{read_class, ACC_PUBLIC, ACC_SYNTHETIC};
use convert_case::Casing;
use proc_macro2::TokenStream;
use syn::parse_file;
use zip::ZipArchive;

use crate::{class_like::ClassLike, GenerateError};

#[derive(Debug)]
pub struct Module {
    pub name: String,
    bindings: Vec<(String, TokenStream)>,
    children: Vec<Module>,
}

pub fn generate_from_jar(
    jar_path: impl Read + Seek,
    root_module: &mut Module,
) -> Result<(), GenerateError> {
    let mut archive = ZipArchive::new(jar_path)?;
    let len = archive.len();

    for i in 0..len {
        let file = archive.by_index(i)?;
        let path = file.enclosed_name().unwrap().to_owned();
        match path.extension() {
            Some(ext) if ext.eq("class") => {}
            _ => {
                println!("Skipping non-class file {}", path.display());
                continue;
            }
        }

        let java_class = match read_class(file) {
            Ok(java_class) => java_class,

            Err(err) => {
                eprintln!("Error reading class file {}: {err:?}", path.display());
                continue;
            }
        };

        if java_class.access_flags & ACC_PUBLIC == 0 || java_class.access_flags & ACC_SYNTHETIC != 0
        {
            eprintln!("Skipping non-public class {}", path.display());
            continue;
        }

        if java_class.get_class_signature().contains("$") {
            eprintln!("Skipping class with $ sign {}", path.display());
            continue;
        }

        let (mut modules, contents) = super::generate::generate_binding(&java_class);
        let struct_name = modules.pop().expect("To have a struct name");
        root_module.add_contents(&modules, struct_name, contents);
    }

    Ok(())
}

impl Module {
    pub fn new(name: String) -> Self {
        Self {
            name,
            bindings: Vec::new(),
            children: Vec::new(),
        }
    }

    fn add_contents(
        &mut self,
        ascendant_modules: &[String],
        struct_name: String,
        contents: TokenStream,
    ) {
        match ascendant_modules.get(0) {
            None => self.bindings.push((struct_name, contents)),

            Some(module) => match self.children.binary_search_by_key(&module, |m| &m.name) {
                Ok(index) => {
                    self.children[index].add_contents(
                        &ascendant_modules[1..],
                        struct_name,
                        contents,
                    );
                }

                Err(index) => {
                    let mut new_module = Module {
                        name: module.to_string(),
                        bindings: Vec::new(),
                        children: Vec::new(),
                    };

                    new_module.add_contents(&ascendant_modules[1..], struct_name, contents);
                    self.children.insert(index, new_module);
                }
            },
        }
    }

    pub fn write_to(&self, dst: &Path) -> Result<(), GenerateError> {
        use std::io::Write;

        std::fs::create_dir_all(dst).map_err(|err| GenerateError::DestinationError(err))?;

        let mut mod_file =
            File::create(dst.join("mod.rs")).map_err(|err| GenerateError::DestinationError(err))?;

        for (name, binding) in &self.bindings {
            let file_name = format!("{}.rs", name.to_case(convert_case::Case::Snake));
            let mut file = File::create(dst.join(&file_name))
                .map_err(|err| GenerateError::DestinationError(err))?;

            file.write_all(
                prettyplease::unparse(&parse_file(&binding.to_string()).unwrap()).as_bytes(),
            )
            .map_err(|err| GenerateError::DestinationError(err))?;

            write!(mod_file, "mod {file_name};\npub use {file_name}::*\n")
                .map_err(|err| GenerateError::DestinationError(err))?;
        }

        for module in &self.children {
            write!(mod_file, "pub mod {};\n", module.name)
                .map_err(|err| GenerateError::DestinationError(err))?;

            let module_path = dst.join(&module.name);
            module.write_to(&module_path)?;
        }

        Ok(())
    }
}

//let contents = ;
