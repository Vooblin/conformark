# Copilot Instructions for Conformark

A CommonMark v0.31.2 parser in Rust (edition 2024) with 99.2% spec compliance (650/655 tests passing).

## Quick Start (First 60 Seconds)

```bash
cargo test -- --nocapture                  # See test results + coverage (99.2%)
echo "**bold**" | cargo run                # Test CLI parser
cargo run --example test_emphasis         # Run 132 emphasis tests (100% passing!)
cargo run --example check_failures        # Analyze 5 currently failing tests
```

**Making changes?** Follow the 3-step pattern: AST enum variant → parser method → renderer match arm. Tests track progress but never fail (non-blocking).

**Finding methods?** Use `grep -n "fn is_\|fn parse_\|fn try_parse_" src/parser.rs` to see all parser methods with line numbers.

**Debugging AST?** Use `serde_json::to_string_pretty(&ast_node)` to inspect parsed structure—all `Node` variants derive `Serialize`.

## Project Philosophy

**Non-blocking tests**: All tests pass in CI regardless of spec coverage. The test harness (`tests/spec_tests.rs`) reports statistics to stderr but never fails—this enables incremental development while tracking progress toward 100% compliance. See line 62 for the pattern.

## Architecture Overview

**5-file core** (`src/{ast,parser,renderer,lib,main}.rs`):
- `ast.rs` (52 lines): Single `Node` enum with 18 variants (all `serde` serializable for tooling/debugging—use `serde_json::to_string_pretty()` to inspect AST structure)
- `parser.rs` (4,395 lines): Two-phase architecture with 45 methods (use `grep -n "fn is_\|fn parse_\|fn try_parse_" src/parser.rs`)
- `renderer.rs` (241 lines): Pattern-matching HTML renderer
- `lib.rs` (64 lines): Public API `markdown_to_html(&str) -> String`
- `main.rs` (11 lines): CLI stdin→HTML converter (`echo "text" | cargo run`)

**Dependencies** (minimal by design):
- `serde` (v1.0, derive feature): AST serialization/deserialization
- `serde_json` (v1.0): Test data loading and optional AST inspection
- `unicode-casefold` (v0.2.0): Unicode case folding for link reference normalization (`[ẞ]` matches `[SS]`)
- `test-fuzz` (dev): Fuzz testing infrastructure

**Why two-phase parsing?** Link references `[label]: destination` can appear anywhere but must be resolved during inline parsing. Phase 1 (lines 31-153 in `src/parser.rs`) scans entire input to collect all references into `HashMap<String, (String, Option<String>)>`, skipping contexts where they don't apply (code blocks, already-parsed HTML blocks). Phase 2 (lines 154-236) parses blocks using these pre-collected references for single-pass inline link resolution. This prevents backtracking when encountering `[text][ref]` syntax.

**Delimiter stack pattern**: Emphasis/strong parsing uses a two-pass algorithm (lines 2139-2850). Pass 1 collects delimiter runs (`*`, `_`) with flanking information into a stack. Pass 2 processes the stack using `process_emphasis()` to match openers with closers, handling precedence rules (strong before emphasis, left-to-right matching). This implements CommonMark's complex emphasis nesting rules without backtracking. The `DelimiterRun` struct (lines 7-15) tracks position, count, flanking rules, and active status for each delimiter.

## Critical Block Parsing Order

**`src/parser.rs` lines 154-236** defines block precedence. **Reordering breaks tests.** The `parse()` method checks in this EXACT sequence:

1. Link reference definitions (skip, already collected)
2. ATX headings (`##`, before `###` thematic breaks)
3. Thematic breaks (`---`, before lists)
4. Blockquotes (`>`)
5. HTML blocks (7 types, before lists since tags look like markers)
6. Lists (unordered/ordered)
7. **Fenced code** (MUST precede indented - can have 0-3 space indent)
8. Indented code (4+ spaces)
9. Blank lines (skip)
10. Setext headings (lookahead for underline)
11. Paragraphs (fallback)

**Why this order matters**: `###` could be ATX heading OR thematic break. HTML `<ul>` could be block OR list marker. Fenced code with 3 space indent must be checked before 4-space indented code.

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

**Fast iteration on specific sections** (8 example runners in `examples/`):
```bash
cargo run --example test_emphasis      # 132 emphasis tests (100% passing)
cargo run --example test_list_items    # 48 list item tests
cargo run --example test_blockquotes   # 25 blockquote tests
cargo run --example test_hard_breaks   # Test hard line breaks
cargo run --example test_html_blocks   # Test HTML block parsing
cargo run --example test_link_refs     # Test link reference definitions
cargo run --example check_failures     # Analyze 6 currently failing tests with diffs
cargo run --example test_169           # Single-test runner (example pattern)
# Pattern: Each example filters tests.json by .section or .example field
# Create new examples by copying the pattern from existing ones
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
- `parse_*`: Block parsers returning `(Node, usize)` where `usize` = lines consumed (e.g., `parse_blockquote()` line 631, `parse_list()` line 1328)
- `try_parse_*`: Inline parsers returning `Option<(Node, usize)>` where `usize` = chars consumed (e.g., `try_parse_link()` line 2855, `try_parse_code_span()` line 2621)

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

## Current Test Coverage (650/655 - 99.2%)

**Remaining failures** (5 tests across 2 categories):
- **Lists** (1 test): Complex blockquote continuation in nested list structures (test 294)
- **Raw HTML** (4 tests): Complex edge cases involving multi-line tags and comments (tests 618, 627, 628, 631)

**Recent progress** (Oct 2025): Improved from 99.1% to 99.2% (649→650 passing). Fixed link reference fallback: when inline link parsing fails (e.g., `[foo](invalid dest)`), parser now correctly falls back to trying shortcut reference link pattern, allowing `[foo]` to match a reference definition.

## Debugging Workflow

When tests fail after changes:
1. `cargo test -- --nocapture` → see example numbers (e.g., "Test 281 failed")
2. `jq '.[] | select(.example == 281)' tests/data/tests.json` → get test details
3. Search `assets/spec.txt` for "Example 281" → read spec rationale
4. `echo "<markdown>" | cargo run` → verify actual output
5. `cargo run --example test_list_items` → faster iteration on section

## Common Pitfalls

1. **Block order violations**: Adding block type in wrong position in `parse()` method (lines 154-236) breaks existing tests. The order is load-bearing.
2. **Tab expansion**: Tabs are NOT 4 spaces - use `count_indent_columns()` which advances to next multiple of 4 (e.g., tab at column 2 → column 4, at column 5 → column 8)
3. **Link refs**: Case-insensitive using Unicode case folding (`[FOO]` matches `[foo]`, `[ẞ]` matches `[SS]`), whitespace-collapsed, stored in phase 1. Normalize with `unicode_casefold::UnicodeCaseFold` trait's `case_fold()` method plus `.split_whitespace().collect::<Vec<_>>().join(" ")` pattern.
4. **Setext headings**: Must lookahead (line 212) before committing to paragraph parse, otherwise underline becomes separate paragraph
5. **HTML blocks**: 7 distinct start conditions (line 814) with different end conditions. Type 1 (`<script>`) ends with `</script>`, Type 6 (normal tags) ends with blank line.
6. **List compatibility**: Compatible markers (same type/delimiter) continue list, incompatible start new list. See `ListType::is_compatible()` line 2005.
7. **Delimiter flanking**: `*` and `_` have asymmetric rules. `_` requires punctuation before/after for certain positions (lines 2723-2783). Don't simplify this logic.

## Key Resources

- `assets/spec.txt`: Full CommonMark v0.31.2 spec (9,811 lines)
- `tests/data/tests.json`: All 655 test cases with example numbers
- `examples/test_*.rs`: Section-focused test runners
- `IMPLEMENTATION_NOTES.md`: Historical context on test harness setup
