# Copilot Instructions for Conformark

A CommonMark v0.31.2 parser in Rust (edition 2024) with 90.5% spec compliance (593/655 tests passing).

## Quick Start (First 60 Seconds)

```bash
cargo test -- --nocapture     # See test results + coverage (90.5%)
echo "**bold**" | cargo run   # Test CLI parser
cargo run --example test_emphasis  # Run 132 emphasis tests only
```

**Making changes?** Follow the 3-step pattern: AST enum variant → parser method → renderer match arm. Tests track progress but never fail (non-blocking).

**Project goal**: Implement a fast, memory-safe CommonMark parser with stable AST, streaming APIs, and optional GFM extensions. Currently achieving 90.5% spec compliance through incremental test-driven development.

## Architecture (5-File Core)

- `src/ast.rs` (49 lines): Single `Node` enum with 18 variants representing the AST. All nodes are serializable via serde.
- `src/parser.rs` (4,013 lines): **Two-phase parsing architecture** with 44 methods
  - **Phase 1 (lines 17-146)**: Scan entire input to collect link reference definitions into `HashMap<String, (String, Option<String>)>`. Skips fenced/indented code blocks and recursively checks blockquotes where link refs can/can't appear.
  - **Phase 2 (lines 147+)**: Parse blocks in critical order (see below), using collected references for inline link resolution.
  - `Parser` struct holds only `reference_definitions` HashMap - stateless for each `parse()` call.
  - **Naming convention**: `is_*` predicates, `parse_*` block parsers (return `(Node, usize)`), `try_parse_*` inline parsers (return `Option<(Node, usize)>`)
  - **Why two phases?** Link references can be defined anywhere in the document but must be resolved during inline parsing. The first pass finds all definitions without parsing inline content, enabling single-pass inline parsing in phase 2.
- `src/renderer.rs` (206 lines): Recursive pattern matching on `Node` → HTML with proper escaping. All block elements output trailing `\n`.
- `src/lib.rs` (64 lines): Public API `markdown_to_html(&str) -> String` + 6 unit tests for edge cases (entities, images, autolinks).
- `src/main.rs` (11 lines): CLI tool that reads stdin → outputs HTML.

**Note**: Uses Rust edition 2024 (cutting edge). Line counts approximate and may drift with development.

**Quick Start**: `echo "**bold**" | cargo run` or `cargo test -- --nocapture` to see test results with diffs and coverage (currently 90.5%).

**Key insight**: The parser is a two-pass scanner. Pass 1 collects all `[label]: destination` definitions (skipping code blocks where they don't apply). Pass 2 parses blocks in strict precedence order - changing this order breaks tests. This enables single-pass inline parsing with all link refs already known.

## Critical Parser Order (src/parser.rs lines 147-400)

After phase 1 collects link references, the `parse()` method checks blocks in this EXACT sequence to prevent misidentification:
1. Link reference definitions (lines 149-153, silent - don't produce blocks)
2. ATX headings (lines 155-158, before thematic breaks since `###` is ambiguous)
3. Thematic breaks (lines 160-163, before lists)
4. Blockquotes (lines 165-168)
5. HTML blocks (lines 170-173, 7 types, before lists since tags look like markers)
6. Lists (lines 175-178, before code blocks)
7. **Fenced code** (lines 180-183, MUST precede indented - can have 0-3 space indent)
8. Indented code blocks (lines 185-188, 4+ spaces)
9. Blank lines (lines 190-193, skip)
10. Setext headings (lines 195-198, lookahead to next line for underline)
11. Paragraphs (lines 200+, fallback)

**Reordering these causes cascading test failures.** Each block type has precedence rules defined in the CommonMark spec.

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

**Find parser methods**: `grep -n "fn is_\|fn parse_\|fn try_parse_" src/parser.rs` (shows 44 methods: ~17 `is_*` predicates, ~13 `parse_*` block parsers, ~14 `try_parse_*` inline parsers)

## Essential Workflows

**Run tests with diagnostics**: `cargo test -- --nocapture` (shows first 10 failures with example numbers + diffs + 90.5% coverage)

**Fast iteration on specific sections** (bypass full test suite): 
```bash
cargo run --example test_emphasis      # 132 emphasis tests only
cargo run --example test_html_blocks   # 46 HTML block tests
cargo run --example test_link_refs     # 27 link reference tests
cargo run --example test_blockquotes   # 25 blockquote tests
cargo run --example test_list_items    # 48 list item tests
cargo run --example test_hard_breaks   # 15 hard line break tests
# Pattern: Copy examples/test_emphasis.rs, change .section filter to target specific section
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

**Parser method naming convention** (44 methods total - use `grep -n "fn is_\|fn parse_\|fn try_parse_" src/parser.rs` to see all):
- `is_*` (17 methods): Predicate methods check if line/position matches pattern, return `bool` or `Option<T>`
  - Examples: `is_indented_code_line()`, `is_thematic_break()`, `is_blockquote_start()`
- `parse_*` (13 methods): Consume input, return `(Node, usize)` where `usize` = lines/chars consumed
  - Examples: `parse_paragraph()`, `parse_blockquote()`, `parse_list()`
- `try_parse_*` (14 methods): Optional parsing for inline elements, return `Option<(Node, usize)>`
  - Examples: `try_parse_link()`, `try_parse_code_span()`, `try_parse_autolink()`

**Lookahead pattern** (used in indented code blocks + setext headings):
```rust
// Look ahead through blank lines to check if pattern continues
let mut j = i + 1;
while j < lines.len() && lines[j].trim().is_empty() { j += 1; }
if j < lines.len() && self.is_indented_code_line(lines[j]) {
    // Include blank lines in code block
}
```

**Renderer output patterns** (all block elements include trailing `\n`):
- Void elements: `<hr />\n`, `<br />\n`, `<img ... />\n`
- Block elements: `<p>...</p>\n`, `<h1>...</h1>\n`, `<blockquote>\n...\n</blockquote>\n`
- Lists: `<ul>\n...\n</ul>\n` (items render their own `\n`)
- Conditional attributes: `<ol start="5">` only if start ≠ 1, `<code class="language-rust">` only if info string present

**Tab handling**: Tabs expand to **next multiple of 4** (NOT fixed 4 spaces). Partial tab removal in `remove_code_indent()` adds padding spaces.

## Current Test Coverage (593/655 passing - 90.5%)

**Top failing sections** (62 tests, find with `jq -r '.[].section' tests/data/tests.json | sort | uniq -c | sort -rn`):
- ~~Link reference definitions in special contexts (multiline titles, inside blockquotes, paragraph interruption)~~ ✅ **100% complete (27/27)**
- ~~Block quotes (lazy continuation, empty lines, fenced code interaction)~~ ✅ **100% complete (25/25)**
- ~~Emphasis and strong emphasis~~ ✅ **97.0% complete (128/132)** - only 4 complex nesting edge cases remain
- List items: 43/48 complete (89.6%) - remaining issues: empty items, blockquote interactions
- Complex link scenarios (nested brackets, reference link edge cases)
- Tab handling in nested contexts (blockquotes, lists, code blocks)

**Largest test sections**:
- Emphasis and strong emphasis: 132 tests
- Links: 90 tests
- List items: 48 tests
- HTML blocks: 46 tests

**Test format** (`tests/data/tests.json`): 655 objects with `{markdown, html, example, start_line, end_line, section}`. 

**Test Philosophy**: Tests are **non-blocking tracking tests** - they report progress but never fail CI. This allows incremental development while maintaining visibility into spec compliance. See `tests/spec_tests.rs` line 63: tests always pass but output detailed statistics.

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
