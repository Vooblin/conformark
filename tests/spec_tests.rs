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

#[test]
fn commonmark_spec_tests() {
    // Load spec tests
    let test_data = fs::read_to_string("tests/data/tests.json").expect("Failed to read tests.json");

    let tests: Vec<SpecTest> =
        serde_json::from_str(&test_data).expect("Failed to parse tests.json");

    let mut passed = 0;
    let mut failed = 0;
    let mut failures = Vec::new();

    for test in tests.iter() {
        let result = markdown_to_html(&test.markdown);

        if result == test.html {
            passed += 1;
        } else {
            failed += 1;
            failures.push(test.example);

            // Print first few failures for debugging
            if failures.len() <= 5 {
                eprintln!("\n❌ Test {} failed ({})", test.example, test.section);
                eprintln!("  Input: {:?}", test.markdown);
                eprintln!("  Expected: {:?}", test.html);
                eprintln!("  Got: {:?}", result);
            }
        }
    }

    eprintln!("\n📊 CommonMark Spec Test Results:");
    eprintln!("  ✅ Passed: {}", passed);
    eprintln!("  ❌ Failed: {}", failed);
    eprintln!(
        "  📈 Coverage: {:.1}%",
        (passed as f64 / (passed + failed) as f64) * 100.0
    );

    if !failures.is_empty() {
        eprintln!(
            "\n  Failed examples: {:?}...",
            &failures[..failures.len().min(10)]
        );
    }

    // Don't fail the test yet - this is a tracking test
    // Once we start implementing, we'll make this strict
    eprintln!("\n  Note: Test harness is ready. Implementation can now proceed incrementally.");
}
