use std::{
    cell::RefCell,
    fs::File,
    io::{Read, Seek},
    path::Path,
    rc::Rc,
};

use classy::{read_class, ACC_PUBLIC, ACC_SYNTHETIC};
use zip::ZipArchive;

use crate::{
    class_like::ClassLike, generate::generate_binding, utils::java_name_to_rust_name, GenerateError,
};

pub trait ZipRead: Read + Seek + 'static {}

impl<T: Read + Seek + 'static> ZipRead for T {}

struct Binding {
    name: String,
    archive: Rc<RefCell<ZipArchive<Box<dyn ZipRead>>>>,
    index: usize,
}

pub struct Module {
    pub name: String,
    bindings: Vec<Binding>,
    children: Vec<Module>,
}

pub fn generate_from_jar(
    jar: Box<dyn ZipRead>,
    root_module: &mut Module,
) -> Result<(), GenerateError> {
    let archive = Rc::new(RefCell::new(ZipArchive::new(jar)?));
    let len = archive.borrow().len();

    for i in 0..len {
        let mut borrowed_archive = archive.borrow_mut();
        let file = borrowed_archive.by_index(i)?;
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

        let (package, name) = java_class.get_package_and_name();
        let name = java_name_to_rust_name(&name);

        let modules = package
            .into_iter()
            .map(|n| java_name_to_rust_name(&n))
            .collect::<Vec<_>>();

        root_module.add_contents(
            &modules,
            Binding {
                name,
                archive: archive.clone(),
                index: i,
            },
        );
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

    fn add_contents(&mut self, ascendant_modules: &[String], binding: Binding) {
        match ascendant_modules.get(0) {
            None => self.bindings.push(binding),

            Some(module) => match self.children.binary_search_by_key(&module, |m| &m.name) {
                Ok(index) => {
                    self.children[index].add_contents(&ascendant_modules[1..], binding);
                }

                Err(index) => {
                    let mut new_module = Module {
                        name: module.to_string(),
                        bindings: Vec::new(),
                        children: Vec::new(),
                    };

                    new_module.add_contents(&ascendant_modules[1..], binding);
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

        writeln!(
            mod_file,
            r"#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_camel_case_types)]"
        )
        .map_err(|err| GenerateError::DestinationError(err))?;

        for Binding {
            name,
            archive,
            index,
        } in &self.bindings
        {
            let file_name = format!("{name}.rs");
            let mut file = File::create(dst.join(&file_name))
                .map_err(|err| GenerateError::DestinationError(err))?;

            let contents = generate_binding(
                &read_class(archive.borrow_mut().by_index(*index)?)
                    .map_err(|e| GenerateError::InvalidClassFile(e))?,
                None,
            );

            file.write_all(contents.as_bytes())
                .map_err(|err| GenerateError::DestinationError(err))?;

            write!(mod_file, "mod {name};\npub use {name}::*;\n")
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
