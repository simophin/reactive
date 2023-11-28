use std::fs::File;

use classy::read_class;
use quote::format_ident;
use syn::parse_file;

pub fn main() {
    // let file = File::open("/Users/fanchao.liu/Kogan/Kogan.com-Android/data/build/tmp/kotlin-classes/auDebug/com/kogan/data/local/prefs/KoganPreferences.class").expect("To open file");
    // let class_file = read_class(file).expect("To read class file");

    // let output = convert::convert_class(
    //     syn::Visibility::Inherited,
    //     format_ident!("Accessor"),
    //     &class_file,
    // );
    // let output = prettyplease::unparse(&parse_file(&output.to_string()).unwrap());

    // println!("{}", output);
}
