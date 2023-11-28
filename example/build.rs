use std::{fs::File, io::Write};

use jranerator::generate;

fn main() {
    let file = File::open("/Users/fanchao.liu/Kogan/Kogan.com-Android/data/build/tmp/kotlin-classes/auDebug/com/kogan/data/local/prefs/KoganPreferences.class").expect("To open file");
    let source_code = generate(&file, "KoganPreference");

    let mut source = File::create("kogan_prefs.rs").expect("To create file");
    source
        .write_all(source_code.as_bytes())
        .expect("To write file");
}
