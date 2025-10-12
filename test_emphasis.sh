#!/bin/bash
# Test script to count passing/failing emphasis tests

cd /Users/dmitriimurygin/projects/conformark

echo "Testing emphasis and strong emphasis..."

# Create a temporary test file
cat > /tmp/test_emphasis.rs << 'EOF'
use conformark::markdown_to_html;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct SpecTest {
    markdown: String,
    html: String,
    example: u32,
    section: String,
}

fn main() {
    let test_data = fs::read_to_string("tests/data/tests.json").unwrap();
    let tests: Vec<SpecTest> = serde_json::from_str(&test_data).unwrap();
    
    let emphasis_tests: Vec<_> = tests.iter()
        .filter(|t| t.section == "Emphasis and strong emphasis")
        .collect();
    
    let mut passed = 0;
    let mut failed = 0;
    let mut failed_examples = Vec::new();
    
    for test in emphasis_tests.iter() {
        let result = markdown_to_html(&test.markdown);
        if result == test.html {
            passed += 1;
        } else {
            failed += 1;
            failed_examples.push(test.example);
            if failed <= 5 {
                eprintln!("\n❌ Example {} FAILED:", test.example);
                eprintln!("  Input: {:?}", test.markdown);
                eprintln!("  Expected: {:?}", test.html);
                eprintln!("  Got: {:?}", result);
            }
        }
    }
    
    println!("\n📊 Emphasis Tests:");
    println!("  ✅ Passed: {}", passed);
    println!("  ❌ Failed: {}", failed);
    println!("  📈 Coverage: {:.1}%", (passed as f64 / (passed + failed) as f64) * 100.0);
    
    if failed > 0 {
        println!("\n  Failed examples: {:?}...", &failed_examples[..failed_examples.len().min(20)]);
    }
}
EOF

# Compile and run
rustc --edition 2024 -L target/debug/deps /tmp/test_emphasis.rs -o /tmp/test_emphasis --extern conformark=target/debug/libconformark.rlib --extern serde=/Users/dmitriimurygin/projects/conformark/target/debug/deps/libserde-dcfff38307ca962b.rlib --extern serde_json=/Users/dmitriimurygin/projects/conformark/target/debug/deps/libserde_json-f17fbd84c555502e.rlib 2>/dev/null && /tmp/test_emphasis
