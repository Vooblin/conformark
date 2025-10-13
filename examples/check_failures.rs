use conformark::markdown_to_html;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SpecTest {
    markdown: String,
    html: String,
    example: u32,
    start_line: u32,
    end_line: u32,
    section: String,
}

fn main() {
    let test_data = fs::read_to_string("tests/data/tests.json").unwrap();
    let tests: Vec<SpecTest> = serde_json::from_str(&test_data).unwrap();

    let failing_examples = [294, 570, 618, 627, 628, 631];

    for &example in &failing_examples {
        if let Some(test) = tests.iter().find(|t| t.example == example) {
            let result = markdown_to_html(&test.markdown);

            println!("\n=== Example {} ({}) ===", test.example, test.section);
            println!("Input:\n{}", test.markdown);
            println!("\nExpected:\n{}", test.html);
            println!("\nActual:\n{}", result);
            println!("\nMatch: {}", result == test.html);
        }
    }
}
