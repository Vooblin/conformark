# Test Harness Implementation

This commit establishes the foundation for test-driven development by implementing a comprehensive CommonMark spec test harness.

## What was added

### Core Infrastructure

- **AST module** (`src/ast.rs`): Basic node types for document tree
- **Parser module** (`src/parser.rs`): Stub parser returning empty documents
- **Renderer module** (`src/renderer.rs`): HTML renderer with proper escaping
- **Library API** (`src/lib.rs`): Public `markdown_to_html()` function

### Test Infrastructure  

- **Spec test harness** (`tests/spec_tests.rs`): Loads and runs all 655 CommonMark v0.31.2 examples
- Parses `tests/data/tests.json` automatically
- Reports pass/fail statistics with detailed failure info
- Shows coverage percentage (currently 0.2% - 1 of 655 tests passing)

### Dependencies

- Added `serde` with derive feature for AST serialization
- Added `serde_json` for loading test data

## Current Status

```text
📊 CommonMark Spec Test Results:
  ✅ Passed: 1
  ❌ Failed: 654  
  📈 Coverage: 0.2%
```

The test harness is fully operational and ready for incremental parser implementation. Each feature can now be developed test-first, validated against the authoritative spec examples.

## Next Steps

With the test infrastructure in place, development can proceed incrementally:

1. Implement basic block parsing (paragraphs, headings, code blocks)
2. Add inline parsing (emphasis, links, code spans)
3. Expand AST node types as needed
4. Track progress via spec test coverage

All 655 CommonMark examples are now executable and guide the implementation path.
