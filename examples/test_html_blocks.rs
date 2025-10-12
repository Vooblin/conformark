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

    let html_block_tests: Vec<_> = tests
        .iter()
        .filter(|t| t.section == "HTML blocks")
        .collect();

    let mut passed = 0;
    let mut failed = 0;
    let mut failed_examples = Vec::new();

    for test in html_block_tests.iter() {
        let result = markdown_to_html(&test.markdown);
        if result == test.html {
            passed += 1;
        } else {
            failed += 1;
            failed_examples.push(test.example);
            if failed <= 10 {
                eprintln!("\n❌ Example {} FAILED:", test.example);
                eprintln!("  Input: {:?}", test.markdown);
                eprintln!("  Expected: {:?}", test.html);
                eprintln!("  Got: {:?}", result);
            }
        }
    }

    println!("\n📊 HTML Block Tests:");
    println!("  ✅ Passed: {}/{}", passed, html_block_tests.len());
    println!("  ❌ Failed: {}", failed);
    println!(
        "  📈 Coverage: {:.1}%",
        (passed as f64 / html_block_tests.len() as f64) * 100.0
    );

    if !failed_examples.is_empty() {
        println!("\n  Failed examples: {:?}", failed_examples);
    }
}
