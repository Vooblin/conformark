# Copilot Instructions for Conformark

> **Note**: This file is intentionally comprehensive (~460 lines) due to the complexity of CommonMark parsing. The 60-second quick start and essential workflows provide immediate productivity; detailed sections support deep work.

## 60-Second Quick Start

**What**: CommonMark v0.31.2 parser in Rust (edition 2024). Three-file core: `ast.rs` (49 lines), `parser.rs` (3,710 lines), `renderer.rs` (206 lines).

**Add features**: (1) Add `Node` variant to `src/ast.rs`, (2) Add `is_*` predicate + `parse_*` method to `src/parser.rs` returning `(Node, usize)`, (3) Add pattern match to `src/renderer.rs`, (4) Run `cargo test -- --nocapture` to see coverage increase.

**Critical**: Parser order matters! `src/parser.rs` checks block types in specific sequence to avoid false positives (link refs → ATX headings → thematic breaks → blockquotes → HTML → lists → fenced code → indented code → setext headings → paragraphs).

**Test-driven**: 655 spec tests in `tests/data/tests.json`. Current: 81.1% coverage (531/655 passing). Non-failing tests track progress intentionally.

**Main gaps (124 failing tests)**: Link reference definitions in special contexts (multiline labels, inside blockquotes, paragraph interruption), complex link scenarios, nested list indentation tracking. Top test sections: Emphasis (132 tests), Links (90), List items (48).

**Key resources**: `assets/spec.txt` (9,811 line authoritative spec), `tests/data/tests.json` (all 655 test cases), `examples/test_*.rs` (focused test runners).

## Essential Workflows

**Run tests with details**: `cargo test -- --nocapture` shows first 5 failures with diffs + coverage %. Currently: 531/655 passing (81.1%).

**Fast iteration**: `cargo test --lib -- --nocapture` runs just library tests (skips doc tests).

**Debug specific sections**: `cargo run --example test_emphasis` runs just emphasis tests (132 cases) - faster iteration. Available: `test_html_blocks`, `test_link_refs`, `test_169`.

**Query tests**: `jq '.[] | select(.section == "Links")' tests/data/tests.json` finds specific test cases. `jq -r '.[].section' tests/data/tests.json | sort | uniq -c | sort -rn` shows sections by test count.

**Find failing tests**: Look at test output from `cargo test -- --nocapture` to see which examples fail (e.g., "Test 203 failed").

**CI requirements**: `cargo fmt --check && cargo clippy && cargo doc` - all must pass before commit.

**CLI usage**: `echo "# Hello" | cargo run` parses stdin to HTML stdout.

## Adding a Feature (3-Step Pattern)

**Example: Adding a new inline element**

1. **AST** (`src/ast.rs`): Add enum variant
   ```rust
   pub enum Node {
       // ... existing
       Strikethrough(Vec<Node>),  // ~~text~~
   }
   ```

2. **Parser** (`src/parser.rs`): Add to inline parsing logic
   ```rust
   fn is_strikethrough_delimiter(&self, chars: &[char], pos: usize) -> bool { ... }
   fn try_parse_strikethrough(&self, chars: &[char], start: usize) -> Option<(Node, usize)> { ... }
   // Insert check in parse_inline() in the correct order
   ```

3. **Renderer** (`src/renderer.rs`): Add pattern match
   ```rust
   Node::Strikethrough(children) => format!("<del>{}</del>", render_children(children))
   ```

**Verify**: `cargo test -- --nocapture` shows increased coverage. Check specific section with `cargo run --example test_your_section`.

## Architecture & Key Files

**Three-file core**:
- `src/ast.rs` (49 lines): 18 `Node` enum variants - Document, Paragraph, Heading, CodeBlock, ThematicBreak, BlockQuote, Lists (Unordered/Ordered/ListItem), Inline nodes (Text, Code, Emphasis, Strong, Link, Image, HardBreak, HtmlBlock, HtmlInline)
- `src/parser.rs` (3,710 lines): Stateful parser with `HashMap` for link references, **two-phase parsing**: (1) collect link reference definitions, (2) parse blocks with inline content
- `src/renderer.rs` (206 lines): Recursive pattern matching on `Node`, HTML escaping, tight/loose list logic

**API & CLI**:
- `src/lib.rs` (64 lines): Single public function `markdown_to_html(&str) -> String`
- `src/main.rs` (11 lines): CLI reads stdin, outputs HTML to stdout

**Test infrastructure**:
- `tests/spec_tests.rs`: Loads 655 JSON test cases, reports first 5 failures with diffs, non-failing (tracks progress)
- `tests/data/tests.json`: 655 CommonMark v0.31.2 examples with `{markdown, html, example, section}`
- `examples/test_*.rs`: Focused test runners for rapid iteration (test_emphasis, test_html_blocks, test_link_refs, test_169)

**Critical: Parser Order** (`src/parser.rs` main loop):
The order of checks in `parse()` prevents false positives:
1. Link reference definitions (silent, don't produce blocks)
2. ATX headings (before thematic breaks - `###` could be either)
3. Thematic breaks (before lists)
4. Blockquotes
5. HTML blocks (7 types, before lists since tags can look like markers)
6. Lists (before code blocks)
7. Fenced code blocks (MUST precede indented - can have 0-3 space indent)
8. Indented code blocks (4+ spaces)
9. Blank lines (skip)
10. Setext headings (requires lookahead to next line for underline)
11. Paragraphs (fallback, continues until interrupted)

## Development Workflow

**Building & Testing:**
```bash
cargo build --verbose                    # Rust 2024 edition
cargo test --verbose                     # Run all 655 spec tests (non-failing)
cargo test -- --nocapture                # See detailed output with first 5 failures + coverage %
cargo fmt --all -- --check               # Format check (required for CI)
cargo clippy --all-targets --all-features -- -D warnings  # Linting (zero warnings)
cargo doc --no-deps --verbose            # Generate docs
```

**Running the CLI:**
```bash
echo "# Hello" | cargo run               # Parse from stdin
cat README.md | cargo run > output.html  # Convert file to HTML
```

**Debugging Specific Sections** (use `examples/` for focused testing):
```bash
# Run section-specific test programs (faster than full suite)
cargo run --example test_emphasis        # Just emphasis tests (132 tests)
cargo run --example test_html_blocks     # Just HTML blocks (46 tests)
cargo run --example test_link_refs       # Just link references (27 tests)
cargo run --example test_169             # Single test case

# Pattern: Copy examples/test_emphasis.rs, change .section filter
# Useful for rapid iteration on specific features
```

**Debugging Individual Test Cases**:
```bash
# Find a specific failing test by example number
jq '.[] | select(.example == 203)' tests/data/tests.json

# Quick test of your changes on specific input
echo "**bold**" | cargo run

# Run just one test case with a custom example program
cargo run --example test_169  # Edit this file to test any example
```

**Test exploration (requires `jq`):**
```bash
# List all 26 sections
jq -r '.[].section' tests/data/tests.json | sort -u

# Get tests for specific section
jq '.[] | select(.section == "ATX headings")' tests/data/tests.json

# Count tests by section (find high-impact areas)
jq -r '.[].section' tests/data/tests.json | sort | uniq -c | sort -rn
# Top sections: Emphasis (132), Links (90), List items (48)

# Find specific test by example number
jq '.[] | select(.example == 123)' tests/data/tests.json
```

**CI Pipeline** (`.github/workflows/ci.yml`):
- Tests on stable, beta, nightly Rust toolchains
- Enforces: build, test, fmt, clippy (zero warnings), docs
- Uploads doc artifacts for each toolchain

## Coding Conventions

**Rust Edition 2024:** Minimum Rust 1.85+. Latest stable features. Uses edition="2024" in `Cargo.toml`.

**Testing Strategy:**
1. **Spec compliance first** - every feature must pass CommonMark tests
2. **Non-failing tests** - `tests/spec_tests.rs` tracks progress, doesn't block CI (intentional during development)
3. **Test format:** `{markdown, html, example, start_line, end_line, section}`
4. **Output:** First 5 failures show detailed diffs, then summary with coverage %

**Current Implementation Patterns:**

1. **AST Nodes** (`src/ast.rs`): Enum variants with serde derives
   ```rust
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub enum Node {
       Document(Vec<Node>),
       Paragraph(Vec<Node>),
       Heading { level: u8, children: Vec<Node> },
       CodeBlock { info: String, literal: String },
       ThematicBreak,
       Text(String),
   }
   ```

2. **Parser Methods** (`src/parser.rs`): Consistent naming and return patterns
   ```rust
   // Predicate methods check if a line matches a pattern
   fn is_thematic_break(&self, line: &str) -> bool { ... }
   fn is_indented_code_line(&self, line: &str) -> bool { ... }
   fn is_fenced_code_start(&self, line: &str) -> Option<(char, usize, usize)> { ... }
   
   // Parse methods consume lines and return (Node, lines_consumed)
   fn parse_atx_heading(&self, line: &str) -> Option<Node> { ... }
   fn parse_fenced_code_block(&self, lines: &[&str], ...) -> (Node, usize) { ... }
   fn parse_indented_code_block(&self, lines: &[&str]) -> (Node, usize) { ... }
   
   // Quick way to find all parsing methods:
   // grep -n "fn is_\|fn parse_\|fn try_parse_" src/parser.rs
   ```

3. **Lookahead for Blank Lines**: Indented code blocks handle blank lines with lookahead
   ```rust
   // Look ahead to check if blank lines are followed by more code
   let mut j = i + 1;
   while j < lines.len() && lines[j].trim().is_empty() {
       j += 1;
   }
   if j < lines.len() && self.is_indented_code_line(lines[j]) {
       // Include blank lines in code block
   }
   ```
   **Pattern used elsewhere**: Setext heading detection also uses lookahead (checks next line for underline before committing to paragraph parsing).

4. **Tab Handling**: Complex partial tab removal (tabs = 4 spaces to next multiple of 4)
   ```rust
   fn remove_code_indent(&self, line: &str) -> String {
       // Removes up to 4 spaces of indentation
       // Handles partial tab removal with space padding
   }
   ```

5. **HTML Escaping** (`src/renderer.rs`): Character-by-character escaping
   ```rust
   fn escape_html(text: &str) -> String {
       text.chars().map(|c| match c {
           '<' => "&lt;".to_string(),
           '>' => "&gt;".to_string(),
           '&' => "&amp;".to_string(),
           '"' => "&quot;".to_string(),
           _ => c.to_string(),
       }).collect()
   }
   ```

6. **Renderer Pattern**: Recursive pattern matching on `Node` enum
   - `Document` → concatenate children
   - `Paragraph` → `<p>...</p>\n`
   - `Heading` → `<h{level}>...</h{level}>\n`
   - `CodeBlock` → `<pre><code class="language-{info}">...</code></pre>\n` (class only if info is non-empty)
   - `ThematicBreak` → `<hr />\n`
   - `BlockQuote` → `<blockquote>\n...\n</blockquote>\n`
   - `UnorderedList` → `<ul>\n...\n</ul>\n`
   - `OrderedList` → `<ol start="{start}">\n...\n</ol>\n` (start attribute only if not 1)
   - `ListItem` → `<li>...</li>\n` (simple) or `<li>\n...\n</li>\n` (with block elements)

7. **List Parsing Pattern** (`src/parser.rs`): Uses helper enum and methods
   ```rust
   enum ListType {
       Unordered(char),    // -, +, *
       Ordered(u32, char), // start number and delimiter (. or ))
   }
   
   fn is_list_start(&self, line: &str) -> Option<ListType> {
       // Detects list markers with ≤3 spaces indent
       // Must have space after marker
   }
   
   fn parse_list(&self, lines: &[&str], list_type: ListType) -> (Node, usize) {
       // Collects consecutive items with compatible markers
       // Different markers split into separate lists
   }
   ```
   - Currently implements flat lists only (single-line items)
   - Future: multi-line items, nesting, tight/loose detection

**CommonMark Parsing Strategy** (from spec):
- **Two-phase parsing** is mandatory: (1) Block parsing, (2) Inline parsing
- Tabs expand to spaces (multiples of 4)
- Indented code blocks: 4+ spaces
- Fenced code blocks: ``` or ~~~ with matching fence
- List items track indentation for nesting
- HTML blocks follow 7 different start conditions
- Emphasis uses delimiter run algorithm

**Error Handling:**
- Parser should never panic on invalid input
- Follow CommonMark philosophy: **no syntax errors**, only different interpretations
- Unexpected input should produce valid output (often literal text)

**File Organization:**

**Current structure** (keep files in `src/` root until refactoring is needed):
```
src/
  lib.rs           # Public API: markdown_to_html() (64 lines)
  ast.rs           # Node enum (18 variants, 49 lines)
  parser.rs        # Parser struct (3,710 lines)
  renderer.rs      # HtmlRenderer with escape_html() (206 lines)
  main.rs          # Binary entry point (11 lines)

tests/
  spec_tests.rs    # CommonMark v0.31.2 test runner
  data/
    tests.json     # 655 spec examples (JSON array)

examples/
  test_emphasis.rs    # Focused test runner for emphasis (132 tests)
  test_html_blocks.rs # Focused test runner for HTML blocks (46 tests)
  test_link_refs.rs   # Focused test runner for link references (27 tests)
  test_169.rs         # Single test case runner (useful for debugging)

assets/
  spec.txt         # Official CommonMark v0.31.2 spec (9,811 lines)

IMPLEMENTATION_NOTES.md  # Historical notes on test harness setup
```

**Planned expansion** (not yet implemented):
```
src/
  parser/
    mod.rs         # Parser entry point
    block.rs       # Block-level parsing
    inline.rs      # Inline parsing (emphasis, links, code spans)
    delimiter.rs   # Emphasis/strong delimiter runs
  ast/
    mod.rs         # AST node definitions
  render/
    mod.rs         # Renderer trait
    html.rs        # HTML backend
    plaintext.rs   # Plaintext backend
  extensions/      # GFM extensions (future)
    tables.rs
    strikethrough.rs
```

## Key Implementation Notes

**Critical Parsing Details (Currently Implemented):**

1. **Parser Ordering Matters**: The `parse()` method checks block types in a specific order to avoid false positives (see "Critical: Parser Order" section above).

2. **Fence Detection**: Backticks or tildes, 3+ characters, max 3 spaces indentation
   ```rust
   fn is_fenced_code_start(&self, line: &str) -> Option<(char, usize, usize)> {
       // Returns (fence_char, fence_length, indent)
       // 4+ spaces = indented code block, not fenced
   }
   ```

3. **Closing Fence Requirements**: Same character, >= opening length, only whitespace after
   ```rust
   fn is_closing_fence(&self, line: &str, fence_char: char, min_fence_len: usize) -> bool
   ```

4. **Thematic Break Rules**: 3+ matching `-`, `_`, or `*` with optional spaces between
   - Must have 0-3 leading spaces (4+ = code block)
   - Can have spaces between characters
   - All non-space chars must match

5. **Setext Heading Detection**: Two-line pattern with content + underline
   ```rust
   fn parse_setext_heading(&self, lines: &[&str]) -> Option<(u8, usize)> {
       // First line: ≤3 spaces indent, would be paragraph otherwise
       // Second line: only '=' (h1) or '-' (h2) chars, ≤3 spaces indent
       // Returns (level, lines_consumed)
   }
   ```

6. **Link Parsing Pattern**: Inline link syntax with destination and optional title
   ```rust
   fn try_parse_link(&self, chars: &[char], start: usize) -> Option<(Node, usize)> {
       // 1. Find closing ']' for link text (track bracket depth)
       // 2. Expect '(' after ']'
       // 3. Parse destination: <...> or raw (no spaces unless in parens)
       // 4. Optional title in quotes (", ', or parens)
       // 5. Expect closing ')'
       // Returns (Node::Link{destination, title, children}, position)
   }
   ```

**Future Features (Not Yet Implemented):**
- Nested lists with proper indentation tracking
- Full emphasis delimiter run algorithm per spec
- Streaming API for large documents
- Performance optimizations (avoid backtracking, benchmarks)

## Common Pitfalls & Troubleshooting

**Common Test Failure Patterns:**
The majority of current failures (133/655 tests) fall into these categories:
1. **Link reference definitions**: Multiline titles, angle bracket handling, whitespace normalization (27 tests total)
2. **Link edge cases**: Complex link scenarios, reference definitions, URL encoding (90 tests total)
3. **List items**: Nested lists with proper indentation tracking (48 tests total)
4. **Tab handling in nested contexts**: Tabs within blockquotes, lists, code blocks - partial tab expansion logic
5. **Emphasis edge cases**: Remaining delimiter run algorithm nuances

**Test Section Breakdown (by test count):**
Top 10 sections with most tests (use `jq` to explore):
- Emphasis and strong emphasis: 132 tests
- Links: 90 tests  
- List items: 48 tests
- HTML blocks: 46 tests
- Fenced code blocks: 29 tests
- Setext headings: 27 tests
- Link reference definitions: 27 tests
- Lists: 26 tests
- Block quotes: 25 tests
- Images: 22 tests

**When tests fail after changes:**
```bash
# Run with output to see which tests are failing
cargo test -- --nocapture

# Check specific section's tests
jq '.[] | select(.section == "ATX headings")' tests/data/tests.json | jq -s '.[0:3]'

# Verify formatting and linting
cargo fmt && cargo clippy --all-targets --all-features
```

**Understanding test failures:**
Test output shows:
- Example number (cross-reference with `tests/data/tests.json`)
- Section name (e.g., "Tabs", "Block quotes")
- Input markdown, expected HTML, actual HTML
- First 5 failures are detailed, then summary

**Rust 2024 Edition Issues:**
Edition 2024 is new (2024 stable release). If you encounter edition-related errors:
- Check Rust version: `rustc --version` (should be 1.85+)
- Update toolchain: `rustup update stable`
- Reference: https://doc.rust-lang.org/edition-guide/rust-2024/

**Common pitfalls:**
1. **Tab handling**: Tabs expand to next multiple of 4 (not fixed 4 spaces)
2. **List item continuation**: Must maintain indentation relative to list marker
3. **HTML block types**: 7 distinct start conditions, different end conditions
4. **Link reference matching**: Case-insensitive, normalize whitespace
5. **Emphasis precedence**: Code spans and HTML tags take precedence over emphasis

## When Making Changes

1. **Start with tests**: Find relevant examples in `tests/data/tests.json`
2. **Reference the spec**: Check `assets/spec.txt` for authoritative rules
3. **Run full test suite**: All 655 examples must pass eventually
4. **Update docs**: Keep API docs in sync with implementation
5. **Check CI**: Ensure all toolchains pass (stable, beta, nightly)

## Concrete Example: Adding Blockquote Support

**Note**: Blockquotes are already implemented. This example shows the pattern that was followed:

1. **Find relevant tests**:
   ```bash
   jq '.[] | select(.section == "Block quotes")' tests/data/tests.json | head -20
   ```

2. **Add AST node** to `src/ast.rs`:
   ```rust
   pub enum Node {
       // ... existing variants
       BlockQuote(Vec<Node>),
   }
   ```

3. **Parse in `src/parser.rs`** (in the main parse loop):
   ```rust
   // After other block checks
   else if self.is_blockquote_start(line) {
       let (blockquote, lines_consumed) = self.parse_blockquote(&lines[i..]);
       blocks.push(blockquote);
       i += lines_consumed;
   }
   ```
   
   Then implement the method:
   ```rust
   fn parse_blockquote(&self, lines: &[&str]) -> (Node, usize) {
       // Extract lines starting with '>', parse recursively
       // Return (Node::BlockQuote(children), lines_consumed)
   }
   ```

4. **Render in `src/renderer.rs`**:
   ```rust
   Node::BlockQuote(children) => {
       let content: String = children.iter().map(render_node).collect();
       format!("<blockquote>\n{}</blockquote>\n", content)
   }
   ```

5. **Verify**:
   ```bash
   cargo test -- --nocapture  # Watch coverage increase
   cargo fmt && cargo clippy  # Ensure code quality
   ```
