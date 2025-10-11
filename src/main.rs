use conformark::markdown_to_html;
use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");
    let output = markdown_to_html(&input);
    print!("{}", output);
}
