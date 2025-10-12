# Copilot Instructions for Conformark

A CommonMark v0.31.2 parser in Rust (edition 2024) with 83.2% spec compliance (545/655 tests passing).

## Architecture (3-File Core)

- `src/ast.rs` (49 lines): 18 `Node` enum variants (Document, Paragraph, Heading, CodeBlock, ThematicBreak, BlockQuote, Lists, Text, Code, Emphasis, Strong, Link, Image, HardBreak, HtmlBlock, HtmlInline)
- `src/parser.rs` (3,879 lines): **Two-phase parsing architecture**
  - **Phase 1 (lines 17-110)**: Scan entire input to collect link reference definitions into `HashMap<String, (String, Option<String>)>`, skipping fenced/indented code blocks and recursively checking blockquotes where link refs can't/can appear
  - **Phase 2 (lines 111+)**: Parse blocks in critical order, using collected references for inline link resolution
  - Contains 43 helper methods following `is_*`/`parse_*`/`try_parse_*` naming convention
  - `Parser` struct (line 5) holds only the `reference_definitions` HashMap - stateless for each parse call
- `src/renderer.rs` (206 lines): Recursive pattern matching on `Node` → HTML with proper escaping
- `src/lib.rs` (64 lines): Public API `markdown_to_html(&str) -> String` + 6 unit tests for edge cases
- `src/main.rs` (11 lines): CLI tool that reads stdin → outputs HTML

**Quick Start**: `echo "**bold**" | cargo run` or `cargo test -- --nocapture` to see test results with diffs.

## Critical Parser Order (src/parser.rs lines 111-400)

After phase 1 collects link references, the `parse()` method checks blocks in this EXACT sequence to prevent misidentification:
1. Link reference definitions (silent, don't produce blocks)
2. ATX headings (before thematic breaks - `###` ambiguous)
3. Thematic breaks (before lists)
4. Blockquotes
5. HTML blocks (7 types, before lists since tags look like markers)
6. Lists (before code blocks)
7. **Fenced code** (MUST precede indented - can have 0-3 space indent)
8. Indented code blocks (4+ spaces)
9. Blank lines (skip)
10. Setext headings (lookahead to next line for underline)
11. Paragraphs (fallback)

Reordering these causes cascading test failures.

## Adding Features (3-Step Pattern)

**Example workflow**: Adding strikethrough support

1. **AST** (`src/ast.rs`): Add enum variant
   ```rust
   pub enum Node {
       // ... existing variants
       Strikethrough(Vec<Node>),
   }
   ```

2. **Parser** (`src/parser.rs`): Add predicate + parse method (use existing patterns as templates)
   ```rust
   fn is_strikethrough_delimiter(&self, chars: &[char], pos: usize) -> bool { ... }
   fn try_parse_strikethrough(&self, chars: &[char], start: usize) -> Option<(Node, usize)> { ... }
   // Insert check in parse_inline_with_delimiters() at correct position
   ```

3. **Renderer** (`src/renderer.rs`): Add match arm in `render_node()`
   ```rust
   Node::Strikethrough(children) => {
       let content: String = children.iter().map(render_node).collect();
       format!("<del>{}</del>", content)
   }
   ```

**Find parser methods**: `grep -n "fn is_\|fn parse_\|fn try_parse_" src/parser.rs` (shows 43 methods: ~20 `is_*` predicates, ~15 `parse_*` block parsers, ~8 `try_parse_*` inline parsers)

## Essential Workflows

**Run tests with diagnostics**: `cargo test -- --nocapture` (shows first 10 failures with example numbers + diffs + 83.2% coverage)

**Fast iteration on specific sections** (bypass full test suite): 
```bash
cargo run --example test_emphasis      # 132 emphasis tests only
cargo run --example test_html_blocks   # 46 HTML block tests
cargo run --example test_link_refs     # 27 link reference tests
# Pattern: Copy examples/test_emphasis.rs, change .section filter
```

**Query test data** (requires `jq`):
```bash
jq '.[] | select(.section == "Links")' tests/data/tests.json  # Find section tests
jq '.[] | select(.example == 203)' tests/data/tests.json      # Find failing test by number
jq -r '.[].section' tests/data/tests.json | sort | uniq -c | sort -rn  # Count by section
```

**Quick manual test**: `echo "**bold**" | cargo run`

**CI requirements** (3 toolchains: stable/beta/nightly, defined in `.github/workflows/ci.yml`):
- `cargo fmt --all -- --check` (formatting)
- `cargo clippy --all-targets --all-features -- -D warnings` (zero warnings)
- `cargo doc --no-deps` (doc generation)
- `cargo test --verbose` (all tests must pass)

**Dependencies**:
- Runtime: `serde` (with derive), `serde_json` (for test data loading)
- Dev: `test-fuzz = "*"` (fuzzing support, not currently used in CI)

## Implementation Patterns

**Parser method naming convention** (consistently used):
- `is_*`: Predicate methods check if line/position matches pattern, return `bool` or `Option<T>`
- `parse_*`: Consume input, return `(Node, usize)` where `usize` = lines/chars consumed
- `try_parse_*`: Optional parsing for inline elements, return `Option<(Node, usize)>`

**Lookahead pattern** (used in indented code blocks + setext headings):
```rust
// Look ahead through blank lines to check if pattern continues
let mut j = i + 1;
while j < lines.len() && lines[j].trim().is_empty() { j += 1; }
if j < lines.len() && self.is_indented_code_line(lines[j]) {
    // Include blank lines in code block
}
```

**Renderer output patterns** (all include trailing `\n`):
- Void elements: `<hr />\n`, `<br />\n`, `<img ... />\n`
- Block elements: `<p>...</p>\n`, `<h1>...</h1>\n`, `<blockquote>\n...\n</blockquote>\n`
- Lists: `<ul>\n...\n</ul>\n` (items render their own `\n`)
- Conditional attributes: `<ol start="5">` only if start ≠ 1, `<code class="language-rust">` only if info string present

**Tab handling**: Tabs expand to next multiple of 4 (NOT fixed 4 spaces). Partial tab removal in `remove_code_indent()` adds padding spaces.

## Current Test Coverage (545/655 passing - 83.2%)

**Top failing sections** (110 tests, find with `jq -r '.[].section' tests/data/tests.json | sort | uniq -c | sort -rn`):
- ~~Link reference definitions in special contexts (multiline titles, inside blockquotes, paragraph interruption)~~ ✅ **100% complete (27/27)**
- ~~Block quotes (lazy continuation, empty lines, fenced code interaction)~~ ✅ **100% complete (25/25)**
- Complex link scenarios (nested brackets, reference link edge cases)
- Nested list indentation tracking
- Tab handling in nested contexts (blockquotes, lists, code blocks)

**Largest test sections**:
- Emphasis and strong emphasis: 132 tests
- Links: 90 tests
- List items: 48 tests
- HTML blocks: 46 tests

**Test format** (`tests/data/tests.json`): 655 objects with `{markdown, html, example, start_line, end_line, section}`. Tests are non-failing (track progress, don't block CI).

## Key Resources

- `assets/spec.txt` (9,811 lines): Authoritative CommonMark v0.31.2 spec - reference for ambiguous cases
- `tests/data/tests.json`: All 655 spec test cases
- `examples/test_*.rs`: Section-focused test runners for rapid iteration
- `IMPLEMENTATION_NOTES.md`: Historical context on test harness setup

## Debugging Failures

When tests fail after changes:
1. Run `cargo test -- --nocapture` to see which examples fail (shows example numbers like "Test 203 failed")
2. Look up failing test: `jq '.[] | select(.example == 203)' tests/data/tests.json`
3. Check spec: Search `assets/spec.txt` for the example number (format: "Example 203")
4. Verify with CLI: `echo "<input>" | cargo run` to see actual output
5. Run section test: `cargo run --example test_emphasis` for faster iteration

## Common Pitfalls

1. **Block order violations**: Adding new block type in wrong position breaks existing tests
2. **Tab expansion**: Tabs are NOT 4 spaces - they expand to next multiple of 4
3. **Link reference matching**: Case-insensitive, whitespace normalized, stored in first pass
4. **Setext vs paragraph**: Must lookahead before committing to paragraph parse
5. **HTML blocks**: 7 distinct start conditions with different end conditions (see `is_html_block_start`)
6. **List markers**: Compatible markers continue list, incompatible markers start new list

## Error Philosophy

Follow CommonMark: **no syntax errors**, only different interpretations. Parser should never panic - invalid input produces valid output (often literal text).
