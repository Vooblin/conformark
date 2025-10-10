# Copilot Instructions for Conformark

## Project Overview

Conformark is a **CommonMark-compliant Markdown engine** (parser + renderer) written in Rust. The project aims to provide:
- Fast, memory-safe parsing with a stable AST
- Streaming APIs for efficient processing
- Pluggable backends (HTML/Plaintext initially)
- Optional GFM (GitHub Flavored Markdown) extensions
- Full CommonMark spec-test coverage (655 tests from v0.31.2)

**Current Status**: Test harness complete (0.2% coverage - 1/655 tests passing). Core architecture in place: `Parser` (stub), `HtmlRenderer` (basic escaping), and `Node` enum AST. Ready for incremental TDD implementation.

## Architecture & Design Goals

### Core Components (Implementation Status)
1. **Parser** (`src/parser.rs`): Currently a stub returning `Node::Document(vec![])`. Needs two-phase parsing:
   - Phase 1: Block structure (paragraphs, lists, blockquotes, code blocks)
   - Phase 2: Inline elements (emphasis, links, code spans)
2. **AST** (`src/ast.rs`): `Node` enum with 4 variants: `Document`, `Paragraph`, `Heading`, `CodeBlock`, `Text`. Expand incrementally as features are added.
3. **Renderer** (`src/renderer.rs`): `HtmlRenderer` implemented with proper HTML escaping (`<>&"`). Pattern-match on `Node` enum, recursively render children.
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

# Run all tests including 655 spec tests (currently 1 passing)
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
2. **Implement incrementally**: Add `Node` variants to `src/ast.rs`, then parser logic in `src/parser.rs`
3. **Run spec tests**: `cargo test -- --nocapture` shows which examples pass/fail
4. **Track progress**: Coverage % increases as features are added (currently 0.2%)
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
2. **Non-failing tests**: `tests/spec_tests.rs` currently doesn't fail CI (tracking mode). Will become strict once implementation progresses.
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

### Current Implementation Patterns
1. **AST Nodes** (`src/ast.rs`): Enum variants with serde derives
   ```rust
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub enum Node {
       Document(Vec<Node>),
       Paragraph(Vec<Node>),
       Heading { level: u8, children: Vec<Node> },
       CodeBlock { info: String, literal: String },
       Text(String),
   }
   ```

2. **HTML Escaping** (`src/renderer.rs`): Character-by-character escaping for `<`, `>`, `&`, `"`
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

3. **Renderer Pattern**: Recursive pattern matching on `Node` enum
   - `Document` → concatenate children
   - `Paragraph` → wrap in `<p>` tags with newline
   - `Heading` → `<h{level}>` tags with level from 1-6
   - `CodeBlock` → `<pre><code>` with optional `language-{info}` class

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
  ast.rs           # Node enum (4 variants currently)
  parser.rs        # Parser struct (stub implementation)
  renderer.rs      # HtmlRenderer with escape_html()
  main.rs          # Binary entry point (if needed)

tests/
  spec_tests.rs    # CommonMark v0.31.2 test runner
  data/
    tests.json     # 655 spec examples (JSON array)

assets/
  spec.txt         # Official CommonMark v0.31.2 spec (9,811 lines)
```

**Planned expansion**:
```
src/
  parser/
    mod.rs         # Parser entry point
    block.rs       # Block-level parsing
    inline.rs      # Inline parsing
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

### HTML Entities
- Recognized entities: 2125+ HTML5 named entities
- Numeric entities: `&#123;` (decimal), `&#xAB;` (hex)
- Invalid entities render literally with `&` escaped

### Link Processing
- Normalize reference labels (case-insensitive, collapse whitespace)
- Percent-encode URLs per spec
- Support angle-bracket destinations: `<url>`
- Handle balanced parentheses in URLs

### Emphasis Algorithm
- Use **delimiter run** algorithm (not greedy matching)
- Track left/right flanking delimiters
- `*` can open/close emphasis when flanking
- `_` has word boundary restrictions

### Performance Considerations
- Streaming API for large documents
- Avoid backtracking in parser
- Lazy evaluation where possible
- Benchmark against reference implementations (cmark, pulldown-cmark)

## Common Pitfalls

1. **Tab handling**: Tabs expand to next multiple of 4 (not fixed 4 spaces)
2. **List item continuation**: Must maintain indentation relative to list marker
3. **HTML block types**: 7 distinct start conditions, different end conditions
4. **Link reference matching**: Case-insensitive, normalize whitespace
5. **Emphasis precedence**: Code spans and HTML tags take precedence over emphasis

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

## Quick Start for AI Agents

### Before Writing Code
1. Run `cargo test -- --nocapture` to see current coverage (0.2% baseline)
2. Search `tests/data/tests.json` for test cases related to your feature (e.g., `grep "Tabs" tests/data/tests.json`)
3. Read relevant sections in `assets/spec.txt` for precise CommonMark rules

### Implementation Pattern
1. **Add AST node** in `src/ast.rs` (new `Node` variant)
2. **Update parser** in `src/parser.rs` (currently stub - needs block/inline logic)
3. **Update renderer** in `src/renderer.rs` (add pattern match for new node)
4. **Run tests**: `cargo test -- --nocapture` to see pass rate increase
5. **Check CI requirements**: `cargo fmt --check`, `cargo clippy`, `cargo doc`

### Example: Adding Blockquote Support
1. Find blockquote tests: `jq '.[] | select(.section == "Block quotes")' tests/data/tests.json | head -20`
2. Add `Node::BlockQuote(Vec<Node>)` to `src/ast.rs`
3. Parse `>` prefix in `src/parser.rs`
4. Render as `<blockquote>` in `src/renderer.rs`
5. Verify: `cargo test -- --nocapture` shows more passing tests
