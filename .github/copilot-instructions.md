# Copilot Instructions for Conformark

## 60-Second Quick Start

**TL;DR**: CommonMark parser in Rust. Add features by: (1) Add `Node` variant to `src/ast.rs`, (2) Add `is_*` predicate + `parse_*` method to `src/parser.rs` returning `(Node, usize)`, (3) Add pattern match to `src/renderer.rs`, (4) Run `cargo test -- --nocapture` to see coverage increase.

**Critical files**: `tests/data/tests.json` (655 spec tests across 26 sections), `assets/spec.txt` (9,811 line spec), `src/parser.rs` (3,545 lines - order matters!).

**Current status**: 75.7% coverage (496/655 tests passing). Main gaps: nested lists, full emphasis delimiter algorithm, remaining tab/indentation edge cases.

## Quick Start for AI Agents

**Before writing code:**
1. Run `cargo test -- --nocapture` to see current coverage and failure examples
2. Search test cases: `jq '.[] | select(.section == "Your Topic")' tests/data/tests.json`
3. Count section tests: `jq '[.[] | select(.section == "Your Topic")] | length' tests/data/tests.json`
4. Read `assets/spec.txt` for authoritative CommonMark v0.31.2 rules

**To add a feature:**
1. Add `Node` variant to `src/ast.rs` (e.g., `Emphasis(Vec<Node>)`)
2. Implement parsing in `src/parser.rs`:
   - Add `is_*` predicate method (returns bool or Option)
   - Add `parse_*` method returning `(Node, usize)` where usize = lines consumed
   - Insert check in main `parse()` loop **in correct order** (see "Critical: Parser Order")
3. Add rendering in `src/renderer.rs` (pattern match on new node variant)
4. Verify: `cargo test -- --nocapture` shows increased pass count
5. CI checks: `cargo fmt --check && cargo clippy && cargo doc`

## Project Architecture

**Three-file core** (`src/ast.rs`, `src/parser.rs`, `src/renderer.rs`):
- `ast.rs`: 18 `Node` enum variants with serde derives - Document, Paragraph, Heading, CodeBlock, ThematicBreak, BlockQuote, Lists, Inline nodes (Text, Code, Emphasis, Strong, Link, Image, HardBreak, HtmlBlock, HtmlInline)
- `parser.rs`: 3,545 lines, stateful parser with `HashMap` for link references, two-phase parsing (blocks → inline)
- `renderer.rs`: 205 lines, recursive pattern matching on `Node`, HTML escaping, special ListItem logic for block elements

**Public API** (`src/lib.rs`, 64 lines): Single function `markdown_to_html(&str) -> String`

**Binary CLI** (`src/main.rs`): Reads markdown from stdin, outputs HTML to stdout
```bash
echo "# Hello" | cargo run
cat README.md | cargo run > output.html
```

**Test Infrastructure** (`tests/spec_tests.rs`):
- Loads 655 JSON test cases from `tests/data/tests.json` (CommonMark v0.31.2)
- Non-failing test - tracks progress, prints first 5 failures with diffs
- Each test: `{markdown, html, example, start_line, end_line, section}`
- Run with `--nocapture` to see detailed output

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
cargo test --verbose                     # Run all 655 spec tests
cargo test -- --nocapture                # See detailed output with first 5 failures
cargo fmt --all -- --check               # Format check (required for CI)
cargo clippy --all-targets --all-features -- -D warnings  # Linting (zero warnings)
cargo doc --no-deps --verbose            # Generate docs
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

**Rust Edition 2024:** Minimum Rust 1.85+. Latest stable features. Current tested version: 1.90.0 (Sept 14, 2025).

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
  parser.rs        # Parser struct (3,525 lines)
  renderer.rs      # HtmlRenderer with escape_html() (205 lines)
  main.rs          # Binary entry point (11 lines)

tests/
  spec_tests.rs    # CommonMark v0.31.2 test runner
  data/
    tests.json     # 655 spec examples (JSON array)

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
The majority of current failures (159/655 tests) fall into these categories:
1. **Emphasis and strong emphasis**: Full delimiter run algorithm needed (132 tests in section)
2. **Link edge cases**: Complex link scenarios, reference definitions, URL encoding (90 tests)
3. **List items**: Nested lists with proper indentation tracking (48 tests)
4. **HTML blocks edge cases**: While 7 types are implemented, some edge cases remain (46 tests total)
5. **Tab handling in nested contexts**: Tabs within blockquotes, lists, code blocks - partial tab expansion logic

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
