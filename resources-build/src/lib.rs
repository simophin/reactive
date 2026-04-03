//! Build-script helper for the `resources` crate.
//!
//! Call [`generate`] from your crate's `build.rs`:
//!
//! ```rust,no_run
//! fn main() {
//!     resources_build::generate("res", "strings").unwrap();
//! }
//! ```
//!
//! Then expose the generated file at the crate root:
//!
//! ```rust,ignore
//! include!(concat!(env!("OUT_DIR"), "/resources.rs"));
//! ```
//!
//! This produces:
//! - A `pub mod assets { … }` tree of typed `AssetDescriptor` constants.
//! - A `pub mod strings { … }` of message structs implementing `resources::Message`.

mod assets;
mod i18n;
mod names;

use std::fmt::Write as _;
use std::path::Path;
use std::{fs, io};

/// Generate `$OUT_DIR/resources.rs`.
///
/// `res_dir` and `strings_dir` are paths relative to the crate's manifest
/// directory (i.e. relative to `CARGO_MANIFEST_DIR`). Either directory may
/// be absent — its section is simply omitted from the output.
pub fn generate(res_dir: &str, strings_dir: &str) -> io::Result<()> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");

    let res_path = Path::new(&manifest_dir).join(res_dir);
    let strings_path = Path::new(&manifest_dir).join(strings_dir);

    println!("cargo:rerun-if-changed={}", res_path.display());
    println!("cargo:rerun-if-changed={}", strings_path.display());

    let mut output = String::new();

    if res_path.is_dir() {
        writeln!(output, "{}", assets::gen_assets(&res_path, res_dir)?).unwrap();
    }

    if strings_path.is_dir() {
        writeln!(output, "{}", i18n::gen_i18n(&strings_path, strings_dir)?).unwrap();
    }

    fs::write(Path::new(&out_dir).join("resources.rs"), output)
}
