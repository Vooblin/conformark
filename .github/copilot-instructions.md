# Copilot Instructions for Conformark

## Quick Start for AI Agents

**Before writing code:**
1. Run `cargo test -- --nocapture` to see current coverage (32.4% baseline - 212/655 tests)
2. Search `tests/data/tests.json` for test cases: `jq '.[] | select(.section == "Your Topic")' tests/data/tests.json`
3. Count tests in a section: `jq '[.[] | select(.section == "Your Topic")] | length' tests/data/tests.json`
4. Read relevant sections in `assets/spec.txt` for authoritative CommonMark v0.31.2 rules

**To add a feature:**
1. Add `Node` variant to `src/ast.rs` (e.g., `BlockQuote(Vec<Node>)`)
2. Implement parsing in `src/parser.rs` (use `is_*` predicates, `parse_*` methods returning `(Node, usize)`)
3. Add rendering in `src/renderer.rs` (pattern match on new node variant)
4. Verify: `cargo test -- --nocapture` shows increased pass rate
5. CI checks: `cargo fmt --check && cargo clippy && cargo doc`

## Project Overview

Conformark is a **CommonMark-compliant Markdown engine** (parser + renderer) written in Rust. The project aims to provide:
- Fast, memory-safe parsing with a stable AST
- Streaming APIs for efficient processing
- Pluggable backends (HTML/Plaintext initially)
- Optional GFM (GitHub Flavored Markdown) extensions
- Full CommonMark spec-test coverage (655 tests from v0.31.2)

**Current Status**: Test harness complete (**32.4% coverage** - 212/655 tests passing). Core architecture in place with working implementations: `Parser` (ATX headings, Setext headings, thematic breaks, fenced code blocks, indented code blocks, blockquotes, basic lists, basic paragraphs), `HtmlRenderer` (proper HTML escaping), and `Node` enum AST. Implementation proceeding **incrementally via TDD**.

## Architecture & Design Goals

### Core Components (Implementation Status)
1. **Parser** (`src/parser.rs`): Line-by-line parser with implemented features:
   - ✅ ATX headings (1-6 `#` levels with space requirement)
   - ✅ Setext headings (`===` for h1, `---` for h2, with ≤3 spaces indent)
   - ✅ Thematic breaks (`---`, `___`, `***` with proper spacing rules)
   - ✅ Fenced code blocks (``` or ~~~ with 3+ chars, info strings, proper closing)
   - ✅ Indented code blocks (4+ space indentation, blank line handling)
   - ✅ Block quotes (`>` prefix, recursive parsing)
   - ✅ Basic lists (unordered `-`, `+`, `*` and ordered `1.`, `2)` markers, flat structure only)
   - ✅ Basic paragraphs (non-empty, non-heading lines)
   - ⏳ Advanced list features needed:
     - Multi-line list items with continuation
     - Nested lists
     - List item containing multiple block elements (paragraphs, code blocks, etc.)
     - Tight vs loose list detection
   - ⏳ Inline parsing (Phase 2):
     - Emphasis, links, code spans, images
2. **AST** (`src/ast.rs`): `Node` enum with 10 variants: `Document`, `Paragraph`, `Heading`, `CodeBlock`, `ThematicBreak`, `BlockQuote`, `UnorderedList`, `OrderedList`, `ListItem`, `Text`, `Code` (inline code span). Expand incrementally as features are added.
3. **Renderer** (`src/renderer.rs`): `HtmlRenderer` implemented with proper HTML escaping (`<>&"`). Pattern-match on `Node` enum, recursively render children. **Special handling for `ListItem`**: detects nested block elements (checks for `</p>`, `</blockquote>`, etc.) and adjusts formatting accordingly.
4. **Public API** (`src/lib.rs`): `markdown_to_html(&str) -> String` chains parser → renderer.
5. **Streaming API**: Not yet implemented.

### Test Infrastructure (Fully Operational)
- **`tests/spec_tests.rs`**: Loads 655 CommonMark v0.31.2 examples from `tests/data/tests.json`
- Reports pass/fail stats with detailed first 5 failures
- Currently non-failing test (tracking only) - will enforce once implementation starts
- Test format: `{ markdown, html, example, start_line, end_line, section }`
- Example test data: tabs expand to spaces, lists, blockquotes, code blocks, etc.

## Development Workflow

### Building & Testing
```bash
# Build project (Rust 2024 edition)
cargo build --verbose

# Run all tests including 655 spec tests (currently 212 passing, 32.4% coverage)
cargo test --verbose

# See detailed test output with pass/fail stats
cargo test -- --nocapture

# Format check (required for CI)
cargo fmt --all -- --check

# Clippy (zero warnings policy)
cargo clippy --all-targets --all-features -- -D warnings

# Generate docs
cargo doc --no-deps --verbose
```

### CI Pipeline (`.github/workflows/ci.yml`)
- Runs on push and pull requests
- Tests across **stable, beta, and nightly** Rust toolchains
- Enforces: builds, tests, formatting, clippy, and docs
- Uploads documentation artifacts for each toolchain
- All checks must pass before merge

### TDD Workflow (Critical)
1. **Find relevant tests**: Search `tests/data/tests.json` by section (e.g., "Tabs", "Headings", "Lists")
   ```bash
   # List all sections
   jq -r '.[].section' tests/data/tests.json | sort -u
   
   # Get all tests for a section
   jq '.[] | select(.section == "ATX headings")' tests/data/tests.json
   
   # Count tests per section
   jq -r '.[].section' tests/data/tests.json | sort | uniq -c | sort -rn
   
   # Find failing test by example number
   jq '.[] | select(.example == 123)' tests/data/tests.json
   ```
2. **Implement incrementally**: Add `Node` variants to `src/ast.rs`, then parser logic in `src/parser.rs`
3. **Run spec tests**: `cargo test -- --nocapture` shows which examples pass/fail
4. **Track progress**: Coverage % increases as features are added (currently 32.4%)
5. **Reference spec**: `assets/spec.txt` (9,811 lines) has authoritative CommonMark v0.31.2 rules

### Dependencies
- **serde** (with derive): For AST serialization (`#[derive(Serialize, Deserialize)]` on `Node`)
- **serde_json**: Parse `tests/data/tests.json`
- **test-fuzz** (dev): For property-based fuzzing tests (not yet used)
- Edition: Rust 2024

## Coding Conventions

### Rust Edition 2024
Use latest stable features. Check for breaking changes when they arise.

### Testing Strategy
1. **Spec compliance first**: Every parser/renderer feature must pass corresponding CommonMark tests
2. **Non-failing tests**: `tests/spec_tests.rs` currently doesn't fail CI (tracking mode). This is intentional during incremental development - the test reports pass/fail statistics but doesn't block the build. Will become strict once core features are complete.
3. **Incremental validation**: After each feature, run `cargo test -- --nocapture` to see new passing tests
4. **Reference test structure** (`tests/data/tests.json`):
   ```json
   {
     "markdown": "\tfoo\tbaz\t\tbim\n",
     "html": "<pre><code>foo\tbaz\t\tbim\n</code></pre>\n",
     "example": 1,
     "start_line": 355,
     "end_line": 360,
     "section": "Tabs"
   }
   ```
5. **Test output format**: First 5 failures show detailed diffs (input, expected, actual), then summary statistics

### Current Implementation Patterns

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

### CommonMark Parsing Strategy (from spec)
**Two-phase parsing** is mandatory:
1. **Block parsing**: Consume lines, build document structure
2. **Inline parsing**: Parse raw text in blocks into inline elements

**Critical parsing rules**:
- Tabs expand to spaces (multiples of 4)
- Indented code blocks: 4+ spaces
- Fenced code blocks: \`\`\` or ~~~ with matching fence
- List items track indentation for nesting
- HTML blocks follow 7 different start conditions
- Emphasis uses delimiter run algorithm

### Error Handling
- Parser should never panic on invalid input
- Follow CommonMark philosophy: **no syntax errors**, only different interpretations
- Unexpected input should produce valid output (often literal text)

## File Organization (Current & Planned)

**Current structure**:
```
src/
  lib.rs           # Public API: markdown_to_html()
  ast.rs           # Node enum (10 variants currently)
  parser.rs        # Parser struct (ATX headings, Setext headings, thematic breaks, fenced code blocks, indented code blocks, blockquotes, lists, paragraphs)
  renderer.rs      # HtmlRenderer with escape_html()
  main.rs          # Binary entry point (if needed)

tests/
  spec_tests.rs    # CommonMark v0.31.2 test runner
  data/
    tests.json     # 655 spec examples (JSON array)

assets/
  spec.txt         # Official CommonMark v0.31.2 spec (9,811 lines)
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

**Note**: Keep all files in `src/` root until code organization becomes necessary. Don't create subdirectories prematurely.

## Key Implementation Notes

### Critical Parsing Details (Currently Implemented)

1. **Parser Ordering Matters**: The `parse()` method checks block types in a specific order to avoid false positives. This order is critical:
   ```rust
   // Order in Parser::parse() main loop:
   1. ATX headings (before thematic breaks, since ### could be heading or break)
   2. Thematic breaks (before lists/paragraphs)
   3. Blockquote start
   4. List start (before code blocks)
   5. Fenced code blocks (MUST come before indented code)
   6. Indented code blocks
   7. Blank lines (skip)
   8. Setext headings (requires lookahead to next line)
   9. Paragraphs (fallback)
   ```
   **Why this matters**: Fenced code blocks can have up to 3 spaces indentation, so they must be checked before indented code blocks (which require 4+ spaces). ATX headings like `### foo` must be checked before thematic breaks to avoid misinterpretation.

2. **Fence Detection**: Backticks or tildes, 3+ characters, max 3 spaces indentation
   ```rust
   fn is_fenced_code_start(&self, line: &str) -> Option<(char, usize, usize)> {
       // Returns (fence_char, fence_length, indent)
       // 4+ spaces = indented code block, not fenced
   }
   ```

2. **Closing Fence Requirements**: Same character, >= opening length, only whitespace after
   ```rust
   fn is_closing_fence(&self, line: &str, fence_char: char, min_fence_len: usize) -> bool
   ```

3. **Thematic Break Rules**: 3+ matching `-`, `_`, or `*` with optional spaces between
   - Must have 0-3 leading spaces (4+ = code block)
   - Can have spaces between characters
   - All non-space chars must match

4. **Setext Heading Detection**: Two-line pattern with content + underline
   ```rust
   fn parse_setext_heading(&self, lines: &[&str]) -> Option<(u8, usize)> {
       // First line: ≤3 spaces indent, would be paragraph otherwise
       // Second line: only '=' (h1) or '-' (h2) chars, ≤3 spaces indent
       // Returns (level, lines_consumed)
   }
   ```
   - Underline must be all `=` (level 1) or all `-` (level 2)
   - Trailing spaces allowed on underline
   - Cannot have spaces between underline characters
   - Most tests require inline parsing for full compatibility

### Future Features (Not Yet Implemented)

- **HTML Entities**: 2125+ HTML5 named entities, numeric entities (`&#123;`, `&#xAB;`)
- **Link Processing**: Reference label normalization, percent-encoding, balanced parens
- **Emphasis Algorithm**: Delimiter run algorithm (not greedy), left/right flanking rules
- **Streaming API**: For large documents
- **Performance**: Avoid backtracking, lazy evaluation, benchmark vs cmark/pulldown-cmark

## Common Pitfalls

1. **Tab handling**: Tabs expand to next multiple of 4 (not fixed 4 spaces)
2. **List item continuation**: Must maintain indentation relative to list marker
3. **HTML block types**: 7 distinct start conditions, different end conditions
4. **Link reference matching**: Case-insensitive, normalize whitespace
5. **Emphasis precedence**: Code spans and HTML tags take precedence over emphasis

## Troubleshooting

### Tests failing after changes
```bash
# Run with output to see which tests are failing
cargo test -- --nocapture

# Check specific section's tests
jq '.[] | select(.section == "ATX headings")' tests/data/tests.json | jq -s '.[0:3]'

# Verify formatting and linting
cargo fmt && cargo clippy --all-targets --all-features
```

### Understanding test failures
Test output shows:
- Example number (cross-reference with `tests/data/tests.json`)
- Section name (e.g., "Tabs", "Block quotes")
- Input markdown, expected HTML, actual HTML
- First 5 failures are detailed, then summary

### Rust 2024 Edition Issues
Edition 2024 is new (2024 stable release). If you encounter edition-related errors:
- Check Rust version: `rustc --version` (should be 1.85+)
- Update toolchain: `rustup update stable`
- Reference: https://doc.rust-lang.org/edition-guide/rust-2024/

## When Making Changes

1. **Start with tests**: Find relevant examples in `tests/data/tests.json`
2. **Reference the spec**: Check `assets/spec.txt` for authoritative rules
3. **Run full test suite**: All 655 examples must pass
4. **Add fuzz tests**: For new parsing logic
5. **Update docs**: Keep API docs in sync with implementation
6. **Check CI**: Ensure all toolchains pass

## Resources

- CommonMark Spec: https://spec.commonmark.org/0.31.2/
- Reference Implementation: https://github.com/commonmark/cmark
- Rust Markdown Parsers: pulldown-cmark, comrak (for comparison)
- Test Suite: `tests/data/tests.json` (authoritative for this project)

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
