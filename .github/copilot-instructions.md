# Copilot Instructions for Conformark

## Project Overview

Conformark is a **CommonMark-compliant Markdown engine** (parser + renderer) written in Rust. The project aims to provide:
- Fast, memory-safe parsing with a stable AST
- Streaming APIs for efficient processing
- Pluggable backends (HTML/Plaintext initially)
- Optional GFM (GitHub Flavored Markdown) extensions
- Full CommonMark spec-test coverage

**Current Status**: Early development - project has only the initial commit with scaffolding.

## Architecture & Design Goals

### Core Components (To Be Implemented)
1. **Parser**: Converts Markdown text → AST (two-phase parsing per CommonMark spec)
   - Phase 1: Block structure (paragraphs, lists, blockquotes, etc.)
   - Phase 2: Inline elements (emphasis, links, code spans, etc.)
2. **AST**: Stable, serializable tree representation
3. **Renderer**: AST → output format (HTML, plaintext, etc.)
4. **Streaming API**: Process documents without loading entire content into memory

### Test-Driven Development
- **Golden standard**: `tests/data/tests.json` contains 655 CommonMark spec examples (v0.31.2)
- Each test case has: `markdown`, `html`, `example` number, line ranges, and `section`
- The spec text is in `assets/spec.txt` (~9,812 lines)
- **Critical**: All code changes must pass spec conformance tests

## Development Workflow

### Building & Testing
```bash
# Build with all toolchains (stable, beta, nightly)
cargo build --verbose

# Run tests (must maintain 100% spec conformance)
cargo test --verbose

# Format check (required for CI)
cargo fmt --all -- --check

# Clippy (zero warnings policy)
cargo clippy --all-targets --all-features -- -D warnings

# Generate docs
cargo doc --no-deps --verbose
```

### CI Pipeline (`.github/workflows/ci.yml`)
- Runs on push and pull requests
- Tests across stable, beta, and nightly Rust toolchains
- Enforces: builds, tests, formatting, clippy, and docs
- Uploads documentation artifacts

### Dependencies
- **serde**: For AST serialization
- **test-fuzz**: For fuzzing-based testing (property testing)
- Edition: Rust 2024

## Coding Conventions

### Rust Edition 2024
Use latest stable features. Check for breaking changes when they arise.

### Testing Strategy
1. **Spec compliance first**: Every parser/renderer feature must pass corresponding CommonMark tests
2. **Fuzz testing**: Use `test-fuzz` for property-based testing of parser robustness
3. **Reference `tests/data/tests.json`** structure when writing tests:
   ```json
   {
     "markdown": "input text",
     "html": "expected output",
     "example": 123,
     "start_line": 456,
     "end_line": 789,
     "section": "Section Name"
   }
   ```

### CommonMark Parsing Strategy (from spec)
**Two-phase parsing** is mandatory:
1. **Block parsing**: Consume lines, build document structure
2. **Inline parsing**: Parse raw text in blocks into inline elements

**Critical parsing rules**:
- Tabs expand to spaces (multiples of 4)
- Indented code blocks: 4+ spaces
- Fenced code blocks: ``` or ~~~ with matching fence
- List items track indentation for nesting
- HTML blocks follow 7 different start conditions
- Emphasis uses delimiter run algorithm

### Error Handling
- Parser should never panic on invalid input
- Follow CommonMark philosophy: **no syntax errors**, only different interpretations
- Unexpected input should produce valid output (often literal text)

## File Organization (Planned)

```
src/
  lib.rs           # Public API
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
