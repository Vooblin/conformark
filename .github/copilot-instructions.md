# Copilot Instructions for Conformark

A CommonMark v0.31.2 parser in Rust (edition 2024) with **100% spec compliance** (655/655 tests passing).

## Quick Start (First 60 Seconds)

```bash
cargo test -- --nocapture                  # See test results + coverage (100%)
echo "**bold**" | cargo run                # Test CLI parser
cargo run --example test_emphasis         # Run 132 emphasis tests (100% passing!)
cargo run --example check_failures        # Analyze any failing tests
```

**Making changes?** Follow the 3-step pattern: AST enum variant → parser method → renderer match arm. Tests track progress but never fail (non-blocking).

**Finding methods?** Use `grep -n "fn is_\|fn parse_\|fn try_parse_" src/parser.rs` to see all 45+ parser methods with line numbers.

**Debugging AST?** Use `serde_json::to_string_pretty(&ast_node)` to inspect parsed structure—all `Node` variants derive `Serialize`.

> **Note on line numbers**: This file references specific line numbers (e.g., "line 256", "lines 163-236"). If code changes, use the grep commands provided to relocate the relevant code sections.

## Project Philosophy

**Non-blocking tests**: All tests pass in CI regardless of spec coverage. The test harness (`tests/spec_tests.rs`) reports statistics to stderr but never fails—this enables incremental development while tracking progress toward 100% compliance. See line 62 for the pattern. This design allows continuous integration while building toward full spec compliance.

## Architecture Overview

**5-file core** (`src/{ast,parser,renderer,lib,main}.rs`):
- `ast.rs` (52 lines): Single `Node` enum with 18 variants (all `serde` serializable for tooling/debugging—use `serde_json::to_string_pretty()` to inspect AST structure)
- `parser.rs` (4,419 lines): Two-phase architecture with 45 methods (use `grep -n "fn is_\|fn parse_\|fn try_parse_" src/parser.rs`)
- `renderer.rs` (241 lines): Pattern-matching HTML renderer
- `lib.rs` (64 lines): Public API `markdown_to_html(&str) -> String`
- `main.rs` (11 lines): CLI stdin→HTML converter (`echo "text" | cargo run`)

**Dependencies** (minimal by design):
- `serde` (v1.0, derive feature): AST serialization/deserialization for debugging
- `serde_json` (v1.0): Test data loading from `tests/data/tests.json` + optional AST inspection
- `unicode-casefold` (v0.2.0): Unicode case folding for link reference normalization (e.g., `[ẞ]` matches `[SS]`)
- `test-fuzz` (dev): Fuzz testing infrastructure (not yet actively used)

**Why two-phase parsing?** Link references `[label]: destination` can appear anywhere but must be resolved during inline parsing. Phase 1 (lines 32-162 in `src/parser.rs`) scans entire input to collect all references into `HashMap<String, (String, Option<String>)>`, skipping contexts where they don't apply (code blocks, already-parsed HTML blocks, blockquotes processed recursively). Phase 2 (lines 163-236) parses blocks using these pre-collected references for single-pass inline link resolution. This prevents backtracking when encountering `[text][ref]` syntax.

**Delimiter stack pattern**: Emphasis/strong parsing uses a two-pass algorithm (lines 2086-2850). Pass 1 collects delimiter runs (`*`, `_`) with flanking information into a stack. Pass 2 processes the stack using `process_emphasis()` (line 2303) to match openers with closers, handling precedence rules (strong before emphasis, left-to-right matching). This implements CommonMark's complex emphasis nesting rules without backtracking. The `DelimiterRun` struct (lines 7-15) tracks position, count, flanking rules, and active status for each delimiter run.

## Critical Block Parsing Order

**`src/parser.rs` lines 163-236** defines block precedence. **Reordering breaks tests.** The `parse()` method checks in this EXACT sequence:

1. Link reference definitions (skip, already collected in phase 1)
2. ATX headings (e.g., `##`, before thematic breaks to avoid ambiguity with `###`)
3. Thematic breaks (`---`, before lists since `---` could be list marker)
4. Blockquotes (`>`)
5. HTML blocks (7 types, before lists since tags like `<ul>` look like markers)
6. Lists (unordered/ordered)
7. **Fenced code** (MUST precede indented - can have 0-3 space indent)
8. Indented code (4+ spaces)
9. Blank lines (skip)
10. Setext headings (lookahead for underline)
11. Paragraphs (fallback)

**Why this order matters**: `###` could be ATX heading OR thematic break. HTML tags like `<ul>` could be block OR list marker. Fenced code with 3 space indent must be checked before 4-space indented code.

## Adding Features (3-Step Pattern)

**Example**: Adding strikethrough support

1. **AST** (`src/ast.rs`): Add variant to `Node` enum
   ```rust
   Strikethrough(Vec<Node>),
   ```

2. **Parser** (`src/parser.rs`): Add parsing logic
   - Block elements: Add `is_*` predicate + `parse_*` method returning `(Node, usize)`
   - Inline elements: Add `try_parse_*` method returning `Option<(Node, usize)>`, insert check in `parse_inline_with_delimiter_stack()`
   
3. **Renderer** (`src/renderer.rs`): Add match arm in `render_node()`
   ```rust
   Node::Strikethrough(children) => {
       format!("<del>{}</del>", children.iter().map(render_node).collect::<String>())
   }
   ```

## Essential Workflows

**Run all tests**: `cargo test -- --nocapture` (shows first 5 failures with diffs + coverage stats)

**Fast iteration on specific sections** (10 example runners in `examples/`):
```bash
cargo run --example test_emphasis      # 132 emphasis tests (100% passing)
cargo run --example test_list_items    # 48 list item tests
cargo run --example test_blockquotes   # 25 blockquote tests
cargo run --example test_hard_breaks   # Test hard line breaks
cargo run --example test_html_blocks   # Test HTML block parsing
cargo run --example test_link_refs     # Test link reference definitions
cargo run --example check_failures     # Analyze any failing tests with diffs
cargo run --example test_169           # Single-test runner (example pattern)
cargo run --example test_618           # Single-test runner for test 618
cargo run --example test_618_detailed  # Detailed output for test 618
```

**Example runner pattern** (copy to create new test runners):
```rust
// Filter tests by section name
let emphasis_tests: Vec<_> = tests
    .iter()
    .filter(|t| t.section == "Emphasis and strong emphasis")
    .collect();

// Or by specific example numbers
let failing_examples = [294, 570, 618];
for &example in &failing_examples {
    if let Some(test) = tests.iter().find(|t| t.example == example) {
        let result = markdown_to_html(&test.markdown);
        println!("Example {}: {}", example, result == test.html);
    }
}
```

**Query test data** (requires `jq`):
```bash
jq '.[] | select(.example == 281)' tests/data/tests.json              # Get test #281
jq -r '.[].section' tests/data/tests.json | sort | uniq -c | sort -rn # Count by section
jq '[.[] | select(.section == "Lists")] | length' tests/data/tests.json # Section size
```

**Manual testing**: `echo "**bold**" | cargo run` (stdin → HTML)

**CI requirements** (3 toolchains: stable/beta/nightly, see `.github/workflows/ci.yml`):
- `cargo fmt --all -- --check` - Enforce formatting
- `cargo clippy --all-targets --all-features -- -D warnings` - No warnings allowed
- `cargo doc --no-deps` - Documentation must build
- `cargo test --verbose` - Tests always pass (non-blocking harness)

## Implementation Patterns

**Parser method naming** (find with `grep -n "fn is_\|fn parse_\|fn try_parse_" src/parser.rs`):
- `is_*`: Predicates returning `bool` or `Option<T>` (e.g., `is_thematic_break()` line 569, `is_fenced_code_start()` line 375)
- `parse_*`: Block parsers returning `(Node, usize)` where `usize` = lines consumed (e.g., `parse_blockquote()` line 631, `parse_list()` line 1268)
- `try_parse_*`: Inline parsers returning `Option<(Node, usize)>` where `usize` = chars consumed (e.g., `try_parse_link()` line 2805, `try_parse_code_span()` line 2642)

**Lookahead pattern** for indented code + setext headings (prevents premature paragraph commits):
```rust
let mut j = i + 1;
while j < lines.len() && lines[j].trim().is_empty() { j += 1; }
if j < lines.len() && self.is_indented_code_line(lines[j]) {
    // Include blank lines in code block
}
```

**Renderer output conventions** (all block elements end with `\n`):
- Void tags: `<hr />\n`, `<br />\n`
- Block tags: `<p>...</p>\n`, `<blockquote>\n...\n</blockquote>\n`
- Conditional attributes: `<ol start="5">` only if start ≠ 1 (line 62), `<code class="language-rust">` only if info string present (line 38)

**Tab handling**: Tabs advance to **next multiple of 4 columns** (NOT fixed 4 spaces). The `count_indent_columns()` method (line 256 in `src/parser.rs`) implements spec-compliant column counting. Critical for indented code detection and list item continuation.

## Current Test Coverage (655/655 - 100%)

**Status**: ✅ **100% CommonMark v0.31.2 spec compliance achieved!**

All 655 test cases passing. The parser correctly handles all CommonMark features including complex edge cases like lazy continuation in nested blockquotes within list items.

**Recent progress** (Oct 2025): Achieved 100% spec compliance by fixing lazy continuation logic in blockquotes. The fix ensures that:
1. List items within blockquotes can have lazy continuation lines
2. But list markers themselves cannot lazy-continue (preventing unintended list item additions)
3. This allows complex nesting like blockquotes→lists→blockquotes with proper lazy paragraph continuation

## Troubleshooting

**"Cannot find function `is_xyz` or `parse_xyz`"**: Use `grep -n "fn is_\|fn parse_\|fn try_parse_" src/parser.rs` to see all parser methods with current line numbers. The codebase has 45+ methods following strict naming conventions.

**Line numbers don't match**: Code has changed—use grep patterns from this file to locate the relevant sections. For example: `grep -n "FIRST PASS" src/parser.rs` or `grep -n "struct DelimiterRun" src/parser.rs`.

**Tests passing but output seems wrong**: Remember tests are non-blocking (line 62 in `tests/spec_tests.rs`). Check stderr for actual pass/fail statistics: `cargo test -- --nocapture 2>&1 | grep -A5 "CommonMark Spec"`.

**Clippy warnings after changes**: CI requires zero warnings (`-D warnings` flag). Common issues: unused variables in match arms, missing `#[allow(dead_code)]` on test structs, or non-idiomatic patterns. Run `cargo clippy --all-targets --all-features` locally.

**Tab vs space issues**: Never treat tabs as 4 spaces—use `count_indent_columns()` (line 256) which advances to next multiple of 4. Example: tab at column 2 → column 4, at column 5 → column 8.

## Debugging Workflow

When tests fail after changes:
1. `cargo test -- --nocapture` → see example numbers (e.g., "Test 281 failed")
2. `jq '.[] | select(.example == 281)' tests/data/tests.json` → get test details
3. Search `assets/spec.txt` for "Example 281" → read spec rationale
4. `echo "<markdown>" | cargo run` → verify actual output
5. `cargo run --example test_list_items` → faster iteration on section

## Common Pitfalls

1. **Block order violations**: Adding block type in wrong position in `parse()` method (lines 163-236) breaks existing tests. The order is load-bearing.
2. **Tab expansion**: Tabs are NOT 4 spaces - use `count_indent_columns()` which advances to next multiple of 4 (e.g., tab at column 2 → column 4, at column 5 → column 8)
3. **Link refs**: Case-insensitive using Unicode case folding (`[FOO]` matches `[foo]`, `[ẞ]` matches `[SS]`), whitespace-collapsed, stored in phase 1. Normalize with `unicode_casefold::UnicodeCaseFold` trait's `case_fold()` method plus `.split_whitespace().collect::<Vec<_>>().join(" ")` pattern.
4. **Setext headings**: Must lookahead (line 212) before committing to paragraph parse, otherwise underline becomes separate paragraph
5. **HTML blocks**: 7 distinct start conditions (line 814) with different end conditions. Type 1 (`<script>`) ends with `</script>`, Type 6 (normal tags) ends with blank line.
6. **List compatibility**: Compatible markers (same type/delimiter) continue list, incompatible start new list. See `ListType::is_compatible()` line 1945.
7. **Delimiter flanking**: `*` and `_` have asymmetric rules. `_` requires punctuation before/after for certain positions (lines 2744-2805). Don't simplify this logic.

## Key Resources

- `assets/spec.txt`: Full CommonMark v0.31.2 spec (9,811 lines)
- `tests/data/tests.json`: All 655 test cases with example numbers
- `examples/test_*.rs`: Section-focused test runners
- `IMPLEMENTATION_NOTES.md`: Historical context on test harness setup
