use std::{fs::File, io::Write};

use jranerator::generate;

fn main() {
    let file = File::open("./FragmentProductReviewsBinding.class").expect("To open file");
    let source_code = generate(&file, None);

    let mut source = File::create("src/gen.rs").expect("To create file");
    source
        .write_all(source_code.as_bytes())
        .expect("To write file");
}
