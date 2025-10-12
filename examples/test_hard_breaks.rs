use conformark::markdown_to_html;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct TestCase {
    markdown: String,
    html: String,
    example: u32,
    section: String,
}

fn main() {
    let content = fs::read_to_string("tests/data/tests.json").expect("Failed to read tests");
    let tests: Vec<TestCase> = serde_json::from_str(&content).expect("Failed to parse JSON");

    let hard_break_tests: Vec<&TestCase> = tests
        .iter()
        .filter(|t| t.section == "Hard line breaks")
        .collect();

    println!(
        "Testing {} hard line break tests...\n",
        hard_break_tests.len()
    );

    let mut passed = 0;
    let mut failed = 0;

    for test in hard_break_tests {
        let result = markdown_to_html(&test.markdown);
        if result == test.html {
            passed += 1;
            println!("✅ Test {} passed", test.example);
        } else {
            failed += 1;
            println!("❌ Test {} failed", test.example);
            println!("  Input: {:?}", test.markdown);
            println!("  Expected: {:?}", test.html);
            println!("  Got: {:?}\n", result);
        }
    }

    println!("\n📊 Hard Line Breaks Results:");
    println!("  ✅ Passed: {}", passed);
    println!("  ❌ Failed: {}", failed);
    println!(
        "  📈 Coverage: {:.1}%",
        (passed as f64 / (passed + failed) as f64) * 100.0
    );
}
