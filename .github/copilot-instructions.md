# Copilot Instructions for Conformark

A CommonMark v0.31.2 parser in Rust (edition 2024) with 96.5% spec compliance (632/655 tests passing).

## Quick Start (First 60 Seconds)

```bash
cargo test -- --nocapture                  # See test results + coverage (96.5%)
echo "**bold**" | cargo run                # Test CLI parser
cargo run --example test_emphasis         # Run 132 emphasis tests (100% passing!)
cargo run --example check_failures        # Analyze specific failing tests
```

**Making changes?** Follow the 3-step pattern: AST enum variant → parser method → renderer match arm. Tests track progress but never fail (non-blocking).

**Finding methods?** Use `grep -n "fn is_\|fn parse_\|fn try_parse_" src/parser.rs` to see all parser methods with line numbers.

## Project Philosophy

**Non-blocking tests**: All tests pass in CI regardless of spec coverage. The test harness (`tests/spec_tests.rs`) reports statistics to stderr but never fails—this enables incremental development while tracking progress toward 100% compliance. See line 62 for the pattern.

## Architecture Overview

**5-file core** (`src/{ast,parser,renderer,lib,main}.rs`):
- `ast.rs` (52 lines): Single `Node` enum with 18 variants (all `serde` serializable)
- `parser.rs` (4,296 lines): Two-phase architecture with 44+ methods
- `renderer.rs` (241 lines): Pattern-matching HTML renderer
- `lib.rs` (64 lines): Public API `markdown_to_html(&str) -> String`
- `main.rs` (11 lines): CLI stdin→HTML converter

**Why two-phase parsing?** Link references `[label]: destination` can appear anywhere but must be resolved during inline parsing. Phase 1 (lines 31-153 in `src/parser.rs`) scans entire input to collect all references into `HashMap<String, (String, Option<String>)>`, skipping contexts where they don't apply (code blocks, already-parsed HTML blocks). Phase 2 (lines 154-240) parses blocks using these pre-collected references for single-pass inline link resolution. This prevents backtracking when encountering `[text][ref]` syntax.

**Delimiter stack pattern**: Emphasis/strong parsing uses a two-pass algorithm (lines 2102-2498). Pass 1 collects delimiter runs (`*`, `_`) with flanking information into a stack. Pass 2 processes the stack using `process_emphasis()` to match openers with closers, handling precedence rules (strong before emphasis, left-to-right matching). This implements CommonMark's complex emphasis nesting rules without backtracking.

## Critical Block Parsing Order

**`src/parser.rs` lines 154-240** defines block precedence. **Reordering breaks tests.** The `parse()` method checks in this EXACT sequence:

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
cargo run --example check_failures     # Analyze currently failing tests
# Pattern: Each example filters tests.json by .section field
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
- `is_*`: Predicates returning `bool` or `Option<T>` (e.g., `is_thematic_break()` line 560, `is_fenced_code_start()` line 366)
- `parse_*`: Block parsers returning `(Node, usize)` where `usize` = lines consumed (e.g., `parse_blockquote()` line 622, `parse_list()` line 1295)
- `try_parse_*`: Inline parsers returning `Option<(Node, usize)>` where `usize` = chars consumed (e.g., `try_parse_link()` line 2784, `try_parse_code_span()` line 2621)

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

**Tab handling**: Tabs advance to **next multiple of 4 columns** (NOT fixed 4 spaces). The `count_indent_columns()` method (line 247 in `src/parser.rs`) implements spec-compliant column counting. Critical for indented code detection and list item continuation.

## Current Test Coverage (632/655 - 96.5%)

**Remaining failures** (23 tests across 3 categories):
- **Links**: Multi-line destinations, HTML tag interference in link text, Unicode case folding
- **Lists**: Complex blockquote interactions, setext heading rendering edge cases
- **Code spans**: Backtick edge cases with unusual spacing

**Test Philosophy**: Tests are **non-blocking tracking tests** - they never fail CI but report detailed progress. See `tests/spec_tests.rs` line 62: test always passes, outputs statistics to stderr. Use `cargo run --example check_failures` to see current failures.

**Recent progress** (Oct 2025): Improved from 96.3% to 96.5% (631→632 passing). Fixed tight/loose list detection to properly handle items containing multiple block-level elements with blank lines between them (CommonMark spec §5.3).

## Debugging Workflow

When tests fail after changes:
1. `cargo test -- --nocapture` → see example numbers (e.g., "Test 281 failed")
2. `jq '.[] | select(.example == 281)' tests/data/tests.json` → get test details
3. Search `assets/spec.txt` for "Example 281" → read spec rationale
4. `echo "<markdown>" | cargo run` → verify actual output
5. `cargo run --example test_list_items` → faster iteration on section

## Common Pitfalls

1. **Block order violations**: Adding block type in wrong position in `parse()` method (lines 154-240) breaks existing tests. The order is load-bearing.
2. **Tab expansion**: Tabs are NOT 4 spaces - use `count_indent_columns()` which advances to next multiple of 4 (e.g., tab at column 2 → column 4, at column 5 → column 8)
3. **Link refs**: Case-insensitive (`[FOO]` matches `[foo]`), whitespace-collapsed, stored in phase 1. Normalize with `.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")` pattern.
4. **Setext headings**: Must lookahead (line 212) before committing to paragraph parse, otherwise underline becomes separate paragraph
5. **HTML blocks**: 7 distinct start conditions (line 805) with different end conditions. Type 1 (`<script>`) ends with `</script>`, Type 6 (normal tags) ends with blank line.
6. **List compatibility**: Compatible markers (same type/delimiter) continue list, incompatible start new list. See `ListType::is_compatible()` line 1976.
7. **Delimiter flanking**: `*` and `_` have asymmetric rules. `_` requires punctuation before/after for certain positions (lines 2723-2783). Don't simplify this logic.

## Key Resources

- `assets/spec.txt`: Full CommonMark v0.31.2 spec (9,811 lines)
- `tests/data/tests.json`: All 655 test cases with example numbers
- `examples/test_*.rs`: Section-focused test runners
- `IMPLEMENTATION_NOTES.md`: Historical context on test harness setup
