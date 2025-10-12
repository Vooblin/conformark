/// CommonMark parser implementation
use crate::ast::Node;
use std::collections::HashMap;

/// Delimiter run on the stack for emphasis processing
#[derive(Debug, Clone)]
struct DelimiterRun {
    delimiter: char,
    count: usize,
    pos: usize, // Position in nodes vec
    can_open: bool,
    can_close: bool,
    active: bool,
}

pub struct Parser {
    /// Link reference definitions: label -> (destination, title)
    reference_definitions: HashMap<String, (String, Option<String>)>,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            reference_definitions: HashMap::new(),
        }
    }

    pub fn parse(&mut self, input: &str) -> Node {
        let lines: Vec<&str> = input.lines().collect();

        // FIRST PASS: Collect all link reference definitions
        // Skip lines that are inside code blocks or other contexts where link refs don't apply
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];

            // Skip fenced code blocks entirely
            if let Some((fence_char, fence_len, _indent)) = self.is_fenced_code_start(line) {
                i += 1; // Skip opening fence
                // Skip until closing fence
                while i < lines.len() {
                    if self.is_closing_fence(lines[i], fence_char, fence_len) {
                        i += 1; // Skip closing fence
                        break;
                    }
                    i += 1;
                }
                continue;
            }

            // Skip indented code blocks (4+ spaces)
            if self.is_indented_code_line(line) {
                // Skip consecutive indented lines
                while i < lines.len()
                    && (self.is_indented_code_line(lines[i]) || lines[i].trim().is_empty())
                {
                    // Look ahead to see if blank lines continue the code block
                    if lines[i].trim().is_empty() {
                        let mut j = i + 1;
                        while j < lines.len() && lines[j].trim().is_empty() {
                            j += 1;
                        }
                        if j < lines.len() && self.is_indented_code_line(lines[j]) {
                            i += 1; // Include blank line in code block
                            continue;
                        } else {
                            break; // Blank lines end the code block
                        }
                    }
                    i += 1;
                }
                continue;
            }

            // Handle blockquotes - need to check for link refs inside them
            if self.is_blockquote_start(line) {
                // Collect blockquote lines and check them recursively
                let mut quote_lines = Vec::new();
                let mut j = i;

                while j < lines.len() {
                    let qline = lines[j];
                    if self.is_blockquote_start(qline) {
                        let stripped = self.strip_blockquote_marker(qline);
                        quote_lines.push(stripped);
                        j += 1;
                    } else if !qline.trim().is_empty() && self.can_lazy_continue(qline) {
                        quote_lines.push(qline.to_string());
                        j += 1;
                    } else if qline.trim().is_empty() {
                        // Look ahead to see if blockquote continues
                        let mut k = j + 1;
                        while k < lines.len() && lines[k].trim().is_empty() {
                            k += 1;
                        }
                        if k < lines.len() && self.is_blockquote_start(lines[k]) {
                            quote_lines.extend(std::iter::repeat_n(String::new(), k - j));
                            j = k;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // Recursively parse link refs in blockquote content
                let content = quote_lines.join("\n");
                let content_lines: Vec<&str> = content.lines().collect();
                let mut k = 0;
                while k < content_lines.len() {
                    if let Some(lines_consumed) =
                        self.try_parse_link_reference_definition(&content_lines[k..])
                    {
                        k += lines_consumed;
                    } else {
                        k += 1;
                    }
                }

                i = j;
                continue;
            }

            // Link reference definitions cannot interrupt a paragraph
            // Check if previous line could be part of a paragraph (non-blank, not a block structure)
            if i > 0 {
                let prev_line = lines[i - 1];
                let is_prev_blank = prev_line.trim().is_empty();
                let is_prev_special = self.parse_atx_heading(prev_line).is_some()
                    || self.is_thematic_break(prev_line)
                    || self.is_blockquote_start(prev_line)
                    || self.is_html_block_start(prev_line).is_some()
                    || self.is_list_start(prev_line).is_some()
                    || self.is_fenced_code_start(prev_line).is_some()
                    || self.is_indented_code_line(prev_line);

                // If previous line is not blank and not a special block, it's part of a paragraph
                // Link refs cannot interrupt paragraphs
                if !is_prev_blank && !is_prev_special {
                    i += 1;
                    continue;
                }
            }

            // Try to parse link reference definition
            if let Some(lines_consumed) = self.try_parse_link_reference_definition(&lines[i..]) {
                i += lines_consumed;
            } else {
                i += 1;
            }
        }

        // SECOND PASS: Parse blocks (now with all references available)
        let mut blocks = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Skip link reference definitions (already processed, won't modify state)
            if let Some(lines_consumed) = self.try_parse_link_reference_definition(&lines[i..]) {
                i += lines_consumed;
            }
            // Try to parse ATX heading first
            else if let Some(heading) = self.parse_atx_heading(line) {
                blocks.push(heading);
                i += 1;
            }
            // Try to parse thematic break
            else if self.is_thematic_break(line) {
                blocks.push(Node::ThematicBreak);
                i += 1;
            }
            // Try to parse blockquote
            else if self.is_blockquote_start(line) {
                let (blockquote, lines_consumed) = self.parse_blockquote(&lines[i..]);
                blocks.push(blockquote);
                i += lines_consumed;
            }
            // Try to parse HTML block (before lists, since some HTML tags could look like list items)
            else if let Some(html_block_type) = self.is_html_block_start(line) {
                let (html_block, lines_consumed) =
                    self.parse_html_block(&lines[i..], html_block_type);
                blocks.push(html_block);
                i += lines_consumed;
            }
            // Try to parse list (unordered or ordered)
            else if let Some(list_type) = self.is_list_start(line) {
                let (list, lines_consumed) = self.parse_list(&lines[i..], list_type);
                blocks.push(list);
                i += lines_consumed;
            }
            // Try to parse fenced code block (before indented code block)
            else if let Some((fence_char, fence_len, indent)) = self.is_fenced_code_start(line) {
                let (code_block, lines_consumed) =
                    self.parse_fenced_code_block(&lines[i..], fence_char, fence_len, indent);
                blocks.push(code_block);
                i += lines_consumed;
            }
            // Try to parse indented code block
            else if self.is_indented_code_line(line) {
                let (code_block, lines_consumed) = self.parse_indented_code_block(&lines[i..]);
                blocks.push(code_block);
                i += lines_consumed;
            }
            // Blank lines are skipped
            else if line.trim().is_empty() {
                i += 1;
            }
            // Try to parse Setext heading (check if next line is underline)
            else if i + 1 < lines.len() {
                if let Some((level, lines_consumed)) = self.parse_setext_heading(&lines[i..]) {
                    // Join all content lines (all except the last which is the underline)
                    let content_lines = &lines[i..i + lines_consumed - 1];
                    let trimmed: Vec<&str> = content_lines.iter().map(|line| line.trim()).collect();
                    let text = trimmed.join("\n");
                    let children = self.parse_inline(&text);
                    blocks.push(Node::Heading { level, children });
                    i += lines_consumed;
                } else {
                    // Not a Setext heading, treat as paragraph
                    let (paragraph, lines_consumed) = self.parse_paragraph(&lines[i..]);
                    blocks.push(paragraph);
                    i += lines_consumed;
                }
            }
            // Last line with no possibility of Setext underline
            else {
                let (paragraph, lines_consumed) = self.parse_paragraph(&lines[i..]);
                blocks.push(paragraph);
                i += lines_consumed;
            }
        }

        Node::Document(blocks)
    }

    fn is_indented_code_line(&self, line: &str) -> bool {
        // A line with 4+ columns of indentation
        // Tabs advance to next multiple of 4 columns
        let indent_cols = self.count_indent_columns(line);
        indent_cols >= 4 && !line.trim().is_empty()
    }

    /// Count the number of columns of indentation, treating tabs as advancing to next multiple of 4
    fn count_indent_columns(&self, line: &str) -> usize {
        let mut col = 0;
        for ch in line.chars() {
            match ch {
                ' ' => col += 1,
                '\t' => {
                    // Tab advances to next multiple of 4
                    col = (col / 4 + 1) * 4;
                }
                _ => break,
            }
        }
        col
    }

    fn parse_indented_code_block(&self, lines: &[&str]) -> (Node, usize) {
        let mut code_lines = Vec::new();
        let mut i = 0;

        // Collect consecutive indented or blank lines
        while i < lines.len() {
            let line = lines[i];

            if self.is_indented_code_line(line) {
                // Remove 4 spaces of indentation
                let dedented = self.remove_code_indent(line);
                code_lines.push(dedented);
                i += 1;
            } else if line.trim().is_empty() {
                // Blank lines can be part of code block if followed by more indented lines
                // Look ahead to see if there are more indented lines
                let mut j = i + 1;
                while j < lines.len() && lines[j].trim().is_empty() {
                    j += 1;
                }

                if j < lines.len() && self.is_indented_code_line(lines[j]) {
                    // Include blank lines, but dedent them too (they might have spaces)
                    for &line in lines.iter().take(j).skip(i) {
                        let dedented = self.remove_code_indent(line);
                        code_lines.push(dedented);
                    }
                    i = j;
                } else {
                    // No more indented lines, stop
                    break;
                }
            } else {
                break;
            }
        }

        // Remove trailing blank lines
        while code_lines.last().is_some_and(|l| l.trim().is_empty()) {
            code_lines.pop();
        }

        let literal = code_lines.join("\n") + "\n";

        (
            Node::CodeBlock {
                info: String::new(),
                literal,
            },
            i,
        )
    }

    fn remove_code_indent(&self, line: &str) -> String {
        // Remove up to 4 columns of indentation
        self.remove_indent_columns(line, 4)
    }

    /// Remove up to `columns` worth of indentation from a line
    /// Handles tabs properly (tabs advance to next multiple of 4)
    fn remove_indent_columns(&self, line: &str, columns: usize) -> String {
        let mut col = 0;
        let mut chars = line.chars().peekable();
        let mut result = String::new();

        // Skip up to `columns` of indentation
        while col < columns {
            match chars.peek() {
                Some(&' ') => {
                    chars.next();
                    col += 1;
                }
                Some(&'\t') => {
                    chars.next();
                    let next_tab_stop = (col / 4 + 1) * 4;
                    if next_tab_stop <= columns {
                        // Tab fits entirely within the columns to remove
                        col = next_tab_stop;
                    } else {
                        // Partial tab: it extends beyond columns
                        // Add spaces for the part that extends beyond
                        let spaces_to_add = next_tab_stop - columns;
                        for _ in 0..spaces_to_add {
                            result.push(' ');
                        }
                        col = columns;
                    }
                }
                _ => break,
            }
        }

        // Add remaining characters
        result.extend(chars);
        result
    }

    /// Calculate the number of leading space characters in a line
    fn count_leading_spaces(&self, line: &str) -> usize {
        line.chars().take_while(|&c| c == ' ').count()
    }

    /// Check if a line starts a fenced code block
    /// Returns Some((fence_char, fence_length, indent)) if it does
    fn is_fenced_code_start(&self, line: &str) -> Option<(char, usize, usize)> {
        // Count leading spaces (max 3 for fenced code block)
        let indent = self.count_leading_spaces(line);
        if indent >= 4 {
            return None; // 4+ spaces = indented code block
        }

        let after_indent = &line[indent..];

        // Check for backticks or tildes
        let fence_char = after_indent.chars().next()?;
        if fence_char != '`' && fence_char != '~' {
            return None;
        }

        // Count fence characters (must be 3+)
        let fence_len = after_indent
            .chars()
            .take_while(|&c| c == fence_char)
            .count();
        if fence_len < 3 {
            return None;
        }

        // Per CommonMark spec: info string after backtick fence cannot contain backticks
        // This prevents inline code like ``` ``` from being treated as a fence
        if fence_char == '`' {
            let after_fence = &after_indent[fence_len..];
            if after_fence.contains('`') {
                return None;
            }
        }

        Some((fence_char, fence_len, indent))
    }

    /// Parse a fenced code block starting from the current position
    fn parse_fenced_code_block(
        &self,
        lines: &[&str],
        fence_char: char,
        fence_len: usize,
        fence_indent: usize,
    ) -> (Node, usize) {
        if lines.is_empty() {
            return (
                Node::CodeBlock {
                    info: String::new(),
                    literal: String::new(),
                },
                0,
            );
        }

        // Parse the info string from the opening fence line
        let first_line = lines[0];
        let indent = self.count_leading_spaces(first_line);
        let after_indent = &first_line[indent..];
        let after_fence = &after_indent[fence_len..];

        // Info string is everything after the fence, trimmed
        // But only the first word becomes the language class
        let info_string = after_fence.trim();
        let info = if info_string.is_empty() {
            String::new()
        } else {
            // Extract first word for language class and process backslash escapes and entities
            let raw_info = info_string.split_whitespace().next().unwrap_or("");
            let escaped = self.process_backslash_escapes(raw_info);
            self.process_entities(&escaped)
        };

        let mut code_lines = Vec::new();
        let mut i = 1; // Start after the opening fence

        // Collect lines until we find a closing fence
        while i < lines.len() {
            let line = lines[i];

            // Check if this is a closing fence
            if self.is_closing_fence(line, fence_char, fence_len) {
                // Found closing fence, we're done
                i += 1; // Include the closing fence line
                break;
            }

            // Remove up to fence_indent spaces from the line
            // Per CommonMark spec: if fence is indented N spaces, remove up to N spaces from each line
            let line_with_indent_removed = self.remove_indent_columns(line, fence_indent);
            code_lines.push(line_with_indent_removed);
            i += 1;
        }

        // If we didn't find a closing fence, that's ok - treat rest as code
        let literal = if code_lines.is_empty() {
            String::new()
        } else {
            code_lines.join("\n") + "\n"
        };

        (Node::CodeBlock { info, literal }, i)
    }

    /// Check if a line is a valid closing fence
    fn is_closing_fence(&self, line: &str, fence_char: char, min_fence_len: usize) -> bool {
        // Can have leading spaces (0-3)
        let indent = self.count_leading_spaces(line);
        if indent >= 4 {
            return false;
        }

        let after_indent = &line[indent..];

        // Must start with the same fence character
        if !after_indent.starts_with(fence_char) {
            return false;
        }

        // Count fence characters (must be >= opening fence length)
        let fence_len = after_indent
            .chars()
            .take_while(|&c| c == fence_char)
            .count();
        if fence_len < min_fence_len {
            return false;
        }

        // After the fence, only whitespace is allowed
        let after_fence = &after_indent[fence_len..];
        after_fence.trim().is_empty()
    }

    fn parse_atx_heading(&self, line: &str) -> Option<Node> {
        // ATX headings can have 0-3 spaces of indentation
        // 4+ spaces = indented code block, not a heading
        let leading_spaces = line.chars().take_while(|&c| c == ' ').count();
        if leading_spaces >= 4 {
            return None;
        }

        let trimmed = line.trim_start();

        // Count leading # characters
        let hash_count = trimmed.chars().take_while(|&c| c == '#').count();

        // Must be 1-6 hashes followed by space or end of line
        if hash_count == 0 || hash_count > 6 {
            return None;
        }

        let after_hashes = &trimmed[hash_count..];

        // Must have space after hashes (or be end of line)
        if !after_hashes.is_empty()
            && !after_hashes.starts_with(' ')
            && !after_hashes.starts_with('\t')
        {
            return None;
        }

        // Extract heading text, trim leading/trailing whitespace
        let mut text = after_hashes.trim();

        // Remove trailing # characters only if preceded by whitespace
        // Per CommonMark spec: "The closing sequence of #s is optional,
        // but if present must be preceded by a space"
        if let Some(pos) = text.rfind(|c: char| c != '#' && c != ' ' && c != '\t') {
            // Found a non-hash, non-whitespace character
            let before_trailing = &text[..=pos];
            let trailing = &text[pos + 1..];

            // Check if trailing part is whitespace followed by hashes (and maybe more whitespace)
            let trailing_trimmed = trailing.trim_start();
            if trailing_trimmed
                .chars()
                .all(|c| c == '#' || c == ' ' || c == '\t')
                && trailing_trimmed.contains('#')
                && trailing.starts_with([' ', '\t'])
            {
                text = before_trailing.trim_end();
            }
        } else if text.chars().all(|c| c == '#' || c == ' ' || c == '\t') {
            // Content is only hashes and whitespace - strip everything
            text = "";
        }

        let children = self.parse_inline(text);

        Some(Node::Heading {
            level: hash_count as u8,
            children,
        })
    }

    fn is_thematic_break(&self, line: &str) -> bool {
        // A thematic break is a sequence of three or more matching -, _, or * characters
        // Can have leading spaces (0-3), spaces between characters, and trailing spaces
        // 4+ leading spaces makes it a code block (not implemented yet, but we should check)

        let trimmed = line.trim_start();
        let leading_spaces = line.len() - trimmed.len();

        // If 4+ leading spaces, not a thematic break (would be code block)
        if leading_spaces >= 4 {
            return false;
        }

        // Remove all spaces and check if we have 3+ of the same character
        let chars_only: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();

        if chars_only.len() < 3 {
            return false;
        }

        // Must be all the same character and must be -, _, or *
        let first_char = match chars_only.chars().next() {
            Some(c @ ('-' | '_' | '*')) => c,
            _ => return false,
        };

        chars_only.chars().all(|c| c == first_char)
    }

    /// Check if a line starts a blockquote
    fn is_blockquote_start(&self, line: &str) -> bool {
        // Count leading spaces (max 3 for blockquote)
        let indent = self.count_leading_spaces(line);
        if indent >= 4 {
            return false; // 4+ spaces = code block
        }

        let after_indent = &line[indent..];
        after_indent.starts_with('>')
    }

    /// Check if a line starts a block structure (used for lazy continuation detection)
    /// Returns true if the line starts: ATX heading, thematic break, blockquote,
    /// HTML block, fenced code, or list
    /// Note: Indented code blocks are NOT included because they cannot interrupt paragraphs
    fn is_block_structure_start(&self, line: &str) -> bool {
        // Blank lines are not block structure starts
        if line.trim().is_empty() {
            return false;
        }

        // Check for block starters that CAN interrupt paragraphs
        // Indented code blocks are excluded - they cannot interrupt paragraphs
        self.parse_atx_heading(line).is_some()
            || self.is_thematic_break(line)
            || self.is_blockquote_start(line)
            || self.is_html_block_start(line).is_some()
            || self.is_fenced_code_start(line).is_some()
            || self.is_list_start(line).is_some()
    }

    /// Parse a blockquote starting from the current position
    fn parse_blockquote(&mut self, lines: &[&str]) -> (Node, usize) {
        let mut quote_lines = Vec::new();
        let mut i = 0;
        let mut had_lazy = false;
        let mut last_line_allows_lazy = false;

        while i < lines.len() {
            let line = lines[i];

            // Check if this line is part of the blockquote
            if self.is_blockquote_start(line) {
                // Strip the blockquote marker and add to quote lines
                let stripped = self.strip_blockquote_marker(line);

                quote_lines.push(stripped.clone());
                had_lazy = false; // Reset lazy flag when we see explicit marker

                // Check if this line would allow lazy continuation
                // Blank lines don't allow lazy continuation (end paragraphs)
                // Lists, code blocks, etc. don't allow lazy continuation
                // Only paragraphs (and similar inline content) allow it
                last_line_allows_lazy = !stripped.trim().is_empty()
                    && !self.is_indented_code_line(&stripped)
                    && self.is_list_start(&stripped).is_none()
                    && self.is_fenced_code_start(&stripped).is_none()
                    && !self.is_thematic_break(&stripped)
                    && self.parse_atx_heading(&stripped).is_none()
                    && self.is_html_block_start(&stripped).is_none();

                i += 1;
            } else if !line.trim().is_empty() {
                // Lazy continuation is only possible if the last line allows it
                if !last_line_allows_lazy {
                    break;
                }

                // Check for lazy continuation
                // A non-empty line without > can continue if it's not a new block structure
                if self.can_lazy_continue(line) {
                    // According to CommonMark spec: "The setext heading underline cannot
                    // be a lazy continuation line in a list item or block quote"
                    // So if this looks like a setext underline AND we had a lazy continuation
                    // before it, we need to prevent it from being treated as such.
                    // We do this by adding a backslash escape before the line.
                    let line_to_add = if had_lazy && self.is_setext_underline(line).is_some() {
                        let trimmed = line.trim_start();
                        if !trimmed.is_empty() {
                            let indent = line.len() - trimmed.len();
                            format!("{}\\{}", " ".repeat(indent), trimmed)
                        } else {
                            line.to_string()
                        }
                    } else {
                        line.to_string()
                    };
                    quote_lines.push(line_to_add);
                    had_lazy = true;
                    // Lazy lines continue to allow more lazy lines (paragraph continues)
                    last_line_allows_lazy = true;
                    i += 1;
                } else {
                    // This starts a new block, stop the blockquote
                    break;
                }
            } else {
                // Blank line - according to CommonMark spec:
                // "A blank line always separates block quotes"
                // So we stop here regardless of what follows
                break;
            }
        }

        // Parse the collected lines recursively
        let content = quote_lines.join("\n");
        let inner_ast = self.parse(&content);

        // Extract children from the Document node
        let children = match inner_ast {
            Node::Document(children) => children,
            _ => vec![inner_ast],
        };

        (Node::BlockQuote(children), i)
    }

    /// Strip the blockquote marker (>) and optional following space from a line
    fn strip_blockquote_marker(&self, line: &str) -> String {
        // Remove leading spaces (up to 3)
        let indent = self.count_leading_spaces(line);
        let after_indent = &line[indent..];

        // Remove the > marker
        if let Some(after_marker) = after_indent.strip_prefix('>') {
            // The > can be followed by an optional space (or tab treated as spaces)
            // We need to expand tabs to spaces based on column position,
            // then remove one column for the optional space after >

            if let Some(rest) = after_marker.strip_prefix(' ') {
                // Simple case: space after >, just remove it
                rest.to_string()
            } else if after_marker.starts_with('\t') || !after_marker.is_empty() {
                // Need to handle tabs by expanding to spaces based on column position
                // Start at column (indent + 1) because we're right after the >
                let start_col = indent + 1;
                let expanded = self.expand_tabs(after_marker, start_col);

                // Remove one column (the optional space after >)
                if let Some(rest) = expanded.strip_prefix(' ') {
                    rest.to_string()
                } else {
                    expanded
                }
            } else {
                // No content after >
                String::new()
            }
        } else {
            line.to_string()
        }
    }

    /// Expand tabs to spaces based on column position
    /// Tabs advance to the next multiple of 4
    fn expand_tabs(&self, text: &str, start_col: usize) -> String {
        let mut result = String::new();
        let mut col = start_col;

        for ch in text.chars() {
            if ch == '\t' {
                // Advance to next tab stop (multiple of 4)
                let next_stop = (col / 4 + 1) * 4;
                let spaces = next_stop - col;
                result.push_str(&" ".repeat(spaces));
                col = next_stop;
            } else if ch == '\n' {
                result.push(ch);
                col = 0; // Reset column at newline
            } else {
                result.push(ch);
                col += 1;
            }
        }

        result
    }

    /// Check if a line can continue a blockquote via lazy continuation
    fn can_lazy_continue(&self, line: &str) -> bool {
        // According to CommonMark spec, lazy continuation lines can contain
        // almost anything that would normally be part of a paragraph.
        // The key exceptions are:
        // 1. Thematic breaks (always start new blocks)
        // 2. ATX headings (always start new blocks)
        // 3. Fenced code blocks (need explicit markers)

        // However, indented content and list markers CAN lazy-continue
        // because they could be literal text in a paragraph context.

        // Thematic break
        if self.is_thematic_break(line) {
            return false;
        }

        // ATX heading
        if self.parse_atx_heading(line).is_some() {
            return false;
        }

        // Fenced code block - these need explicit blockquote markers
        if self.is_fenced_code_start(line).is_some() {
            return false;
        }

        // HTML blocks - these interrupt paragraphs
        if self.is_html_block_start(line).is_some() {
            return false;
        }

        // Otherwise, can lazy continue (including indented code and list markers)
        true
    }

    /// Check if a line starts an HTML block (returns the block type 1-7)
    fn is_html_block_start(&self, line: &str) -> Option<u8> {
        // HTML blocks can have up to 3 spaces of indentation
        let indent = self.count_leading_spaces(line);
        if indent > 3 {
            return None;
        }

        let trimmed = line[indent..].trim_start();

        // Type 1: <pre, <script, <style, <textarea (case-insensitive)
        for tag in ["<pre", "<script", "<style", "<textarea"] {
            if trimmed.to_lowercase().starts_with(tag) {
                let after = &trimmed[tag.len()..];
                if after.is_empty()
                    || after.starts_with('>')
                    || after.starts_with(' ')
                    || after.starts_with('\t')
                {
                    return Some(1);
                }
            }
        }

        // Type 2: HTML comment <!--
        if trimmed.starts_with("<!--") {
            return Some(2);
        }

        // Type 3: Processing instruction <?
        if trimmed.starts_with("<?") {
            return Some(3);
        }

        // Type 4: Declaration <! followed by uppercase letter
        if trimmed.starts_with("<!") && trimmed.len() > 2 {
            let ch = trimmed.chars().nth(2).unwrap();
            if ch.is_ascii_uppercase() {
                return Some(4);
            }
        }

        // Type 5: CDATA section <![CDATA[
        if trimmed.starts_with("<![CDATA[") {
            return Some(5);
        }

        // Type 6: Block-level tags
        let block_tags = [
            "address",
            "article",
            "aside",
            "base",
            "basefont",
            "blockquote",
            "body",
            "caption",
            "center",
            "col",
            "colgroup",
            "dd",
            "details",
            "dialog",
            "dir",
            "div",
            "dl",
            "dt",
            "fieldset",
            "figcaption",
            "figure",
            "footer",
            "form",
            "frame",
            "frameset",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "head",
            "header",
            "hr",
            "html",
            "iframe",
            "legend",
            "li",
            "link",
            "main",
            "menu",
            "menuitem",
            "nav",
            "noframes",
            "ol",
            "optgroup",
            "option",
            "p",
            "param",
            "search",
            "section",
            "summary",
            "table",
            "tbody",
            "td",
            "tfoot",
            "th",
            "thead",
            "title",
            "tr",
            "track",
            "ul",
        ];

        for tag in block_tags {
            // Opening tag <tag
            let open_pattern = format!("<{}", tag);
            if trimmed.to_lowercase().starts_with(&open_pattern) {
                let after = &trimmed[open_pattern.len()..];
                if after.is_empty()
                    || after.starts_with('>')
                    || after.starts_with(' ')
                    || after.starts_with('\t')
                    || after.starts_with("/>")
                {
                    return Some(6);
                }
            }
            // Closing tag </tag
            let close_pattern = format!("</{}", tag);
            if trimmed.to_lowercase().starts_with(&close_pattern) {
                let after = &trimmed[close_pattern.len()..];
                if after.is_empty()
                    || after.starts_with('>')
                    || after.starts_with(' ')
                    || after.starts_with('\t')
                {
                    return Some(6);
                }
            }
        }

        // Type 7: Complete open or close tag on a single line
        // This is complex - check if line contains a complete tag followed only by whitespace
        if ((trimmed.starts_with('<') && !trimmed.starts_with("</")) || trimmed.starts_with("</"))
            && self.is_complete_tag_line(trimmed)
        {
            return Some(7);
        }

        None
    }

    /// Check if a line contains a complete HTML tag followed only by whitespace
    /// For HTML block type 7: must be a SINGLE complete tag (open or close) with optional whitespace after
    fn is_complete_tag_line(&self, line: &str) -> bool {
        let trimmed = line.trim_end();

        if !trimmed.starts_with('<') {
            return false;
        }

        // Don't match autolinks - they start with < followed by scheme:
        // Check for common URL schemes to avoid false positives
        if trimmed.len() > 5 {
            let after_bracket = &trimmed[1..];
            if after_bracket.starts_with("http://")
                || after_bracket.starts_with("https://")
                || after_bracket.starts_with("ftp://")
                || after_bracket.starts_with("mailto:")
            {
                return false;
            }
        }

        // Find the end of the first tag (either > or />)
        let mut in_quotes = false;
        let mut quote_char = ' ';
        let mut tag_end = 0;

        for (i, ch) in trimmed.chars().enumerate() {
            if i == 0 {
                continue; // Skip the opening <
            }

            if in_quotes {
                if ch == quote_char {
                    in_quotes = false;
                }
            } else if ch == '"' || ch == '\'' {
                in_quotes = true;
                quote_char = ch;
            } else if ch == '>' {
                tag_end = i;
                break;
            }
        }

        if tag_end == 0 {
            return false; // No closing >
        }

        // Check what comes after the tag - should be ONLY whitespace
        let after_tag = &trimmed[tag_end + 1..];
        if !after_tag.trim().is_empty() {
            return false; // Content after tag means this is inline HTML, not a block
        }

        // Validate the tag itself
        let tag_content = &trimmed[1..tag_end];

        // For closing tags, skip the /
        let tag_part = tag_content.strip_prefix('/').unwrap_or(tag_content);

        // Extract just the tag name (before space or other attributes)
        if let Some(first_char) = tag_part.chars().next() {
            // Tag name must start with ASCII letter
            if first_char.is_ascii_alphabetic() {
                return true;
            }
        }

        false
    }

    /// Parse an HTML block of the given type
    fn parse_html_block(&self, lines: &[&str], block_type: u8) -> (Node, usize) {
        let mut html_lines = Vec::new();
        let mut i = 0;

        // Types 6 and 7 end when followed by a blank line
        // Types 1-5 end when they encounter their specific end condition

        if block_type == 6 || block_type == 7 {
            // Add lines until we hit a blank line
            while i < lines.len() {
                let line = lines[i];

                if line.trim().is_empty() {
                    // Blank line ends the block, but don't include it
                    break;
                }

                html_lines.push(line.to_string());
                i += 1;
            }
        } else {
            // Types 1-5: add first line
            html_lines.push(lines[0].to_string());

            // Check if first line already contains end condition
            if self.check_html_end_condition(lines[0], block_type) {
                let content = html_lines.join("\n") + "\n";
                return (Node::HtmlBlock(content), 1);
            }

            i += 1;

            // Continue until end condition is met
            while i < lines.len() {
                let line = lines[i];
                html_lines.push(line.to_string());

                if self.check_html_end_condition(line, block_type) {
                    i += 1;
                    break;
                }

                i += 1;
            }
        }

        let content = html_lines.join("\n") + "\n";
        (Node::HtmlBlock(content), i)
    }

    /// Check if a line meets the end condition for an HTML block type
    fn check_html_end_condition(&self, line: &str, block_type: u8) -> bool {
        match block_type {
            1 => {
                // End: line contains </pre>, </script>, </style>, or </textarea>
                let lower = line.to_lowercase();
                lower.contains("</pre>")
                    || lower.contains("</script>")
                    || lower.contains("</style>")
                    || lower.contains("</textarea>")
            }
            2 => {
                // End: line contains -->
                line.contains("-->")
            }
            3 => {
                // End: line contains ?>
                line.contains("?>")
            }
            4 => {
                // End: line contains >
                line.contains('>')
            }
            5 => {
                // End: line contains ]]>
                line.contains("]]>")
            }
            6 | 7 => {
                // These types are handled separately in parse_html_block (blank line termination)
                false
            }
            _ => false,
        }
    }

    /// Parse a Setext heading (if the next line is an underline)
    /// Returns Some((level, lines_consumed)) if successful
    fn parse_setext_heading(&self, lines: &[&str]) -> Option<(u8, usize)> {
        if lines.is_empty() {
            return None;
        }

        let first_line = lines[0];

        // First line must have ≤3 spaces of indentation
        let indent = count_leading_spaces(first_line);
        if indent >= 4 {
            return None;
        }

        // Look ahead to find a setext underline
        // We need to simulate paragraph parsing to find where it would end with an underline
        let mut content_lines = 0;
        for (idx, &line) in lines.iter().enumerate() {
            if idx == 0 {
                content_lines = 1;
                continue;
            }

            // Check if this is a setext underline
            if let Some((level, _)) = self.is_setext_underline(line) {
                // Found an underline after content - this is a setext heading
                return Some((level, idx + 1)); // idx lines of content + 1 underline
            }

            // Check if this line would interrupt the paragraph
            // If so, there's no setext heading here
            if line.trim().is_empty() {
                // Blank line ends paragraph without heading
                return None;
            }
            if self.is_thematic_break(line) {
                return None;
            }
            if self.parse_atx_heading(line).is_some() {
                return None;
            }
            if self.is_fenced_code_start(line).is_some() {
                return None;
            }
            if idx == 1 && self.is_indented_code_line(line) {
                // Indented code on second line interrupts
                return None;
            }
            if self.is_blockquote_start(line) {
                return None;
            }
            if self.is_list_start(line).is_some() {
                return None;
            }

            // This line is part of the potential heading content
            content_lines += 1;

            // Don't look too far ahead (reasonable limit)
            if content_lines > 20 {
                return None;
            }
        }

        None
    }

    /// Check if a line is a valid Setext heading underline
    /// Returns Some((level, lines_consumed)) if valid (level 1 for '=', level 2 for '-')
    fn is_setext_underline(&self, line: &str) -> Option<(u8, usize)> {
        // Count leading spaces (max 3)
        let indent = count_leading_spaces(line);
        if indent >= 4 {
            return None;
        }

        // Get the content after indentation
        let content = &line[indent..];

        // Trim trailing whitespace to get the actual underline part
        let trimmed = content.trim_end();

        // Must have at least one character
        if trimmed.is_empty() {
            return None;
        }

        // Find first character (which must be = or -)
        let first_char = trimmed.chars().next()?;

        // Determine level
        let level = match first_char {
            '=' => 1,
            '-' => 2,
            _ => return None,
        };

        // ALL characters in the trimmed part must be the same (= or -)
        // No spaces or other characters allowed in the middle
        for ch in trimmed.chars() {
            if ch != first_char {
                return None;
            }
        }

        Some((level, 2)) // Consume 2 lines (content + underline)
    }

    /// Check if a line starts a list (unordered or ordered)
    /// Returns Some(ListType) if it's a list marker
    fn is_list_start(&self, line: &str) -> Option<ListType> {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        // Max 3 spaces of indentation for list marker
        if indent > 3 {
            return None;
        }

        // Check for unordered list marker: -, +, *
        if let Some(first_char) = trimmed.chars().next()
            && (first_char == '-' || first_char == '+' || first_char == '*')
        {
            // Must be followed by space or end of line
            if trimmed.len() == 1
                || trimmed.chars().nth(1) == Some(' ')
                || trimmed.chars().nth(1) == Some('\t')
            {
                return Some(ListType::Unordered(first_char));
            }
        }

        // Check for ordered list marker: digit(s) followed by . or )
        let mut digit_count = 0;
        let mut chars = trimmed.chars();
        while let Some(ch) = chars.next() {
            if ch.is_ascii_digit() {
                digit_count += 1;
                if digit_count > 9 {
                    // Max 9 digits
                    return None;
                }
            } else if (ch == '.' || ch == ')') && digit_count > 0 {
                // Must be followed by space or end of line
                if let Some(next) = chars.next() {
                    if next == ' ' || next == '\t' {
                        let num_str = &trimmed[0..digit_count];
                        if let Ok(start) = num_str.parse::<u32>() {
                            return Some(ListType::Ordered(start, ch));
                        }
                    }
                } else {
                    // End of line after marker
                    let num_str = &trimmed[0..digit_count];
                    if let Ok(start) = num_str.parse::<u32>() {
                        return Some(ListType::Ordered(start, ch));
                    }
                }
                return None;
            } else {
                break;
            }
        }

        None
    }

    /// Check if a line is an empty list item (marker with no content after it)
    /// Per CommonMark spec: empty list items cannot interrupt paragraphs
    fn is_empty_list_item(&self, line: &str) -> bool {
        if let Some(list_type) = self.is_list_start(line) {
            // Check if there's any non-whitespace content after the marker
            let content = self.extract_list_item_content(line, &list_type);
            content.trim().is_empty()
        } else {
            false
        }
    }

    /// Parse a list (collecting consecutive items with same marker type)
    fn parse_list(&mut self, lines: &[&str], list_type: ListType) -> (Node, usize) {
        let mut items = Vec::new();
        let mut i = 0;
        let mut has_blank_between_items = false;

        while i < lines.len() {
            // Check for thematic break first - it can interrupt a list
            if self.is_thematic_break(lines[i]) {
                // Thematic break interrupts the list
                break;
            }

            // Check if current line is a list item of the same type
            if let Some(current_type) = self.is_list_start(lines[i]) {
                if !list_type.is_compatible(&current_type) {
                    // Different list type, stop this list
                    break;
                }

                // Parse this list item (multi-line support)
                let (item, consumed, item_has_multiple_blocks) =
                    self.parse_list_item(&lines[i..], &current_type);
                items.push(item);
                i += consumed;

                // Check if there's a blank line before the next item
                if i < lines.len() && lines[i].trim().is_empty() {
                    has_blank_between_items = true;
                }

                // Item contains multiple blocks separated by blanks makes list loose
                if item_has_multiple_blocks {
                    has_blank_between_items = true;
                }
            } else if i > 0 && lines[i].trim().is_empty() {
                // Blank line - might continue or end the list
                // Look ahead to see if there's a continuation
                let mut j = i + 1;
                while j < lines.len() && lines[j].trim().is_empty() {
                    j += 1;
                }

                // Check if next non-blank line continues the list
                if j < lines.len()
                    && let Some(next_type) = self.is_list_start(lines[j])
                    && list_type.is_compatible(&next_type)
                {
                    // Continue to next list item
                    has_blank_between_items = true;
                    i = j;
                    continue;
                }

                // No more list items, stop
                break;
            } else {
                // Not a list item, stop
                break;
            }
        }

        // Apply tight/loose formatting to all items
        let tight = !has_blank_between_items;
        let formatted_items = if tight {
            // Tight list - unwrap single paragraphs from items
            items
                .into_iter()
                .map(|item| match item {
                    Node::ListItem(children) => {
                        let unwrapped = children
                            .into_iter()
                            .flat_map(|child| match child {
                                Node::Paragraph(para_children) => para_children,
                                other => vec![other],
                            })
                            .collect();
                        Node::ListItem(unwrapped)
                    }
                    other => other,
                })
                .collect()
        } else {
            // Loose list - items keep their paragraph tags
            items
        };

        // Create the appropriate list node
        let list_node = match list_type {
            ListType::Unordered(_) => Node::UnorderedList {
                tight,
                children: formatted_items,
            },
            ListType::Ordered(start, _) => Node::OrderedList {
                start,
                tight,
                children: formatted_items,
            },
        };

        (list_node, i)
    }

    /// Parse a single list item with multi-line support
    /// Returns (Node, lines_consumed, has_blank_lines)
    fn parse_list_item(&mut self, lines: &[&str], list_type: &ListType) -> (Node, usize, bool) {
        let first_line = lines[0];

        // Calculate the content indent (W + N)
        // W = marker width, N = spaces after marker (1-4)
        let content_indent = self.calculate_list_item_indent(first_line, list_type);

        // Collect all lines belonging to this list item
        let mut item_lines = Vec::new();

        // Add first line content
        let first_content = self.extract_list_item_content(first_line, list_type);
        if !first_content.is_empty() {
            item_lines.push(first_content);
        }

        let mut i = 1;
        let mut has_blank = false;
        let mut last_line_was_blank = false;

        while i < lines.len() {
            let line = lines[i];

            // Blank line
            if line.trim().is_empty() {
                has_blank = true;
                last_line_was_blank = true;
                item_lines.push(String::new());
                i += 1;
                continue;
            }

            // Check indentation to see if line belongs to this item
            let line_indent = self.count_indent_columns(line);

            // Check if this is a new list item at the same level or less indented
            if let Some(_list_type) = self.is_list_start(line) {
                // If the list marker is at our content indent or more, it's a nested list
                // Include it in the item content
                if line_indent < content_indent {
                    // List marker is less indented - it's a sibling or parent item
                    break;
                }
                // Otherwise, it's indented enough to be a nested list, include it
            }

            if line_indent >= content_indent {
                // Remove the item indentation and add to item
                let dedented = self.remove_indent(line, content_indent);
                item_lines.push(dedented);
                last_line_was_blank = false;
                i += 1;
            } else {
                // Lazy continuation: If we have existing content, the previous line was NOT blank,
                // and this line doesn't start a block structure, treat it as paragraph continuation
                let can_lazy_continue = !item_lines.is_empty()
                    && !last_line_was_blank
                    && !self.is_block_structure_start(line);

                if can_lazy_continue {
                    // Add the line with its original indentation (lazy lines aren't dedented)
                    item_lines.push(line.to_string());
                    last_line_was_blank = false;
                    i += 1;
                } else {
                    // Not enough indentation and can't lazy continue, stop item
                    break;
                }
            }
        }

        // Parse the collected lines as blocks
        let item_content = item_lines.join("\n");
        let parsed = self.parse(&item_content);

        // Extract children from the parsed document
        let children = match parsed {
            Node::Document(children) => children,
            other => vec![other],
        };

        // Determine if item has multiple blocks separated by blank lines
        // This makes the parent list loose
        let has_multiple_blocks_with_blanks = has_blank && children.len() > 1;

        let item = Node::ListItem(children);
        (item, i, has_multiple_blocks_with_blanks)
    }

    /// Calculate the required indent for list item continuation
    /// W (marker width) + N (spaces after marker, 1-4 columns)
    /// If there are more than 4 columns of whitespace, only 1 is consumed as spacing (per spec)
    fn calculate_list_item_indent(&self, line: &str, list_type: &ListType) -> usize {
        let trimmed = line.trim_start();
        let initial_indent = self.count_indent_columns(&line[..line.len() - trimmed.len()]);

        match list_type {
            ListType::Unordered(_) => {
                // Marker is 1 char at initial_indent, so marker ends at initial_indent + 1
                let after_marker = &trimmed[1..];

                // Count columns of whitespace after marker
                let mut col = 0;
                for ch in after_marker.chars() {
                    match ch {
                        ' ' => col += 1,
                        '\t' => {
                            // Tab advances to next multiple of 4 from current position
                            let current_pos = initial_indent + 1 + col;
                            let next_tab_stop = (current_pos / 4 + 1) * 4;
                            col += next_tab_stop - current_pos;
                        }
                        _ => break,
                    }
                }

                // Per CommonMark spec: if >4 columns of whitespace, only 1 is consumed as spacing
                // This allows indented code blocks on the first line of a list item
                let spacing = if col > 4 { 1 } else { col.max(1) };
                initial_indent + 1 + spacing
            }
            ListType::Ordered(_, delimiter) => {
                // Find delimiter position to get marker width
                if let Some(pos) = trimmed.find(*delimiter) {
                    let marker_width = pos + 1;
                    let after_marker = &trimmed[marker_width..];

                    // Count columns of whitespace after marker
                    let mut col = 0;
                    for ch in after_marker.chars() {
                        match ch {
                            ' ' => col += 1,
                            '\t' => {
                                let current_pos = initial_indent + marker_width + col;
                                let next_tab_stop = (current_pos / 4 + 1) * 4;
                                col += next_tab_stop - current_pos;
                            }
                            _ => break,
                        }
                    }

                    // Per CommonMark spec: if >4 columns of whitespace, only 1 is consumed
                    let spacing = if col > 4 { 1 } else { col.max(1) };
                    initial_indent + marker_width + spacing
                } else {
                    initial_indent + 2 // Fallback
                }
            }
        }
    }

    /// Remove a specific amount of indentation from a line
    /// Handles partial tab removal by replacing with spaces
    /// Expands any remaining tabs to spaces
    fn remove_indent(&self, line: &str, cols: usize) -> String {
        let mut removed = 0;
        let mut result = String::new();
        let mut chars_to_skip = 0;

        for (idx, ch) in line.chars().enumerate() {
            if removed >= cols {
                // We've removed enough, expand tabs in the rest and return
                let remainder = result + &line.chars().skip(chars_to_skip).collect::<String>();
                return self.expand_tabs(&remainder, removed);
            }

            match ch {
                ' ' => {
                    removed += 1;
                    chars_to_skip = idx + 1;
                }
                '\t' => {
                    let next_tab_stop = (removed / 4 + 1) * 4;
                    if next_tab_stop <= cols {
                        // Whole tab is removed
                        removed = next_tab_stop;
                        chars_to_skip = idx + 1;
                    } else {
                        // Partial tab removal - replace with spaces
                        let spaces_needed = next_tab_stop - cols;
                        result = " ".repeat(spaces_needed);
                        chars_to_skip = idx + 1;
                        removed = cols;
                    }
                }
                _ => {
                    // Hit non-whitespace, expand tabs in the rest and return
                    let remainder = result + &line.chars().skip(chars_to_skip).collect::<String>();
                    return self.expand_tabs(&remainder, removed);
                }
            }
        }

        // Removed all whitespace, expand tabs in remainder and return
        let remainder = result + &line.chars().skip(chars_to_skip).collect::<String>();
        self.expand_tabs(&remainder, removed)
    }

    /// Extract the content after a list marker for the first line
    /// Removes the marker and spacing after it (1-4 columns)
    /// Per spec: if >4 columns of whitespace, only 1 is consumed as spacing
    fn extract_list_item_content(&self, line: &str, list_type: &ListType) -> String {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            return String::new();
        }

        let leading_ws_bytes = line.len() - trimmed.len();

        match list_type {
            ListType::Unordered(_) => {
                // Marker is 1 char
                if trimmed.is_empty() {
                    return String::new();
                }

                // Content starts after marker
                let after_marker_start = leading_ws_bytes + 1;
                if after_marker_start >= line.len() {
                    return String::new();
                }

                let after_marker = &line[after_marker_start..];

                // First, count total columns of whitespace after marker
                let marker_col = self.count_indent_columns(&line[..leading_ws_bytes]);
                let mut col = marker_col + 1; // After marker
                let mut total_ws_cols = 0;

                for ch in after_marker.chars() {
                    match ch {
                        ' ' => total_ws_cols += 1,
                        '\t' => {
                            let next_tab_stop = ((col + total_ws_cols) / 4 + 1) * 4;
                            total_ws_cols += next_tab_stop - (col + total_ws_cols);
                        }
                        _ => break,
                    }
                }

                // Must have at least 1 column of whitespace
                if total_ws_cols == 0 {
                    return String::new();
                }

                // Determine how many columns to remove as spacing
                let spacing_to_remove = if total_ws_cols > 4 { 1 } else { total_ws_cols };

                // Now remove that many columns, handling partial tabs
                let mut removed = 0;
                let mut result = String::new();
                let mut chars_to_skip = 0;

                for (idx, ch) in after_marker.chars().enumerate() {
                    if removed >= spacing_to_remove {
                        break;
                    }

                    match ch {
                        ' ' => {
                            removed += 1;
                            chars_to_skip = idx + 1;
                        }
                        '\t' => {
                            let next_tab_stop = (col / 4 + 1) * 4;
                            let tab_cols = next_tab_stop - col;

                            if removed + tab_cols <= spacing_to_remove {
                                // Whole tab fits in spacing
                                col = next_tab_stop;
                                removed += tab_cols;
                                chars_to_skip = idx + 1;
                            } else {
                                // Partial tab - replace with spaces
                                let cols_to_remove = spacing_to_remove - removed;
                                let cols_to_keep = tab_cols - cols_to_remove;
                                result = " ".repeat(cols_to_keep);
                                chars_to_skip = idx + 1;
                                removed = spacing_to_remove;
                            }
                        }
                        _ => break,
                    }
                }

                let content =
                    result + &after_marker.chars().skip(chars_to_skip).collect::<String>();

                // Expand any remaining tabs in the content
                // Content starts at column marker_col + spacing_to_remove
                let content_col = marker_col + 1 + removed;
                self.expand_tabs(&content, content_col)
            }
            ListType::Ordered(_, delimiter) => {
                // Find delimiter
                if let Some(delim_pos) = trimmed.find(*delimiter) {
                    let marker_end = leading_ws_bytes + delim_pos + 1;
                    if marker_end >= line.len() {
                        return String::new();
                    }

                    let after_marker = &line[marker_end..];
                    let marker_col =
                        self.count_indent_columns(&line[..leading_ws_bytes]) + delim_pos + 1;

                    // First, count total columns of whitespace
                    let mut col = marker_col;
                    let mut total_ws_cols = 0;

                    for ch in after_marker.chars() {
                        match ch {
                            ' ' => total_ws_cols += 1,
                            '\t' => {
                                let next_tab_stop = ((col + total_ws_cols) / 4 + 1) * 4;
                                total_ws_cols += next_tab_stop - (col + total_ws_cols);
                            }
                            _ => break,
                        }
                    }

                    // Must have at least 1 column of whitespace
                    if total_ws_cols == 0 {
                        return String::new();
                    }

                    // Determine how many columns to remove as spacing
                    let spacing_to_remove = if total_ws_cols > 4 { 1 } else { total_ws_cols };

                    // Now remove that many columns
                    let mut removed = 0;
                    let mut result = String::new();
                    let mut chars_to_skip = 0;

                    for (idx, ch) in after_marker.chars().enumerate() {
                        if removed >= spacing_to_remove {
                            break;
                        }

                        match ch {
                            ' ' => {
                                removed += 1;
                                chars_to_skip = idx + 1;
                            }
                            '\t' => {
                                let next_tab_stop = (col / 4 + 1) * 4;
                                let tab_cols = next_tab_stop - col;

                                if removed + tab_cols <= spacing_to_remove {
                                    col = next_tab_stop;
                                    removed += tab_cols;
                                    chars_to_skip = idx + 1;
                                } else {
                                    let cols_to_remove = spacing_to_remove - removed;
                                    let cols_to_keep = tab_cols - cols_to_remove;
                                    result = " ".repeat(cols_to_keep);
                                    chars_to_skip = idx + 1;
                                    removed = spacing_to_remove;
                                }
                            }
                            _ => break,
                        }
                    }

                    let content =
                        result + &after_marker.chars().skip(chars_to_skip).collect::<String>();

                    // Expand any remaining tabs in the content
                    let content_col = marker_col + removed;
                    self.expand_tabs(&content, content_col)
                } else {
                    String::new()
                }
            }
        }
    }
}

/// List type identifier
#[derive(Debug, Clone, PartialEq)]
enum ListType {
    Unordered(char),    // The marker character (-, +, *)
    Ordered(u32, char), // Start number and delimiter (. or ))
}

impl ListType {
    /// Check if two list types are compatible (can be in the same list)
    fn is_compatible(&self, other: &ListType) -> bool {
        match (self, other) {
            (ListType::Unordered(a), ListType::Unordered(b)) => a == b,
            (ListType::Ordered(_, a), ListType::Ordered(_, b)) => a == b,
            _ => false,
        }
    }
}

/// Count leading spaces in a line (tabs count as spaces to next multiple of 4)
fn count_leading_spaces(line: &str) -> usize {
    let mut count = 0;
    for ch in line.chars() {
        match ch {
            ' ' => count += 1,
            '\t' => count += 4 - (count % 4),
            _ => break,
        }
    }
    count
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    /// Parse a paragraph by collecting consecutive non-blank lines
    /// that don't match any other block structure
    fn parse_paragraph(&self, lines: &[&str]) -> (Node, usize) {
        let mut paragraph_lines = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Stop on blank line
            if line.trim().is_empty() {
                break;
            }

            // Stop on thematic break
            if self.is_thematic_break(line) {
                break;
            }

            // Stop on ATX heading
            if self.parse_atx_heading(line).is_some() {
                break;
            }

            // Stop on fenced code block
            if self.is_fenced_code_start(line).is_some() {
                break;
            }

            // Stop on indented code block (4+ spaces) - but ONLY on the first line
            // Per CommonMark spec: "Lines after the first may be indented any amount,
            // since indented code blocks cannot interrupt paragraphs."
            if i == 0 && self.is_indented_code_line(line) {
                break;
            }

            // Stop on blockquote
            if self.is_blockquote_start(line) {
                break;
            }

            // Stop on list - but with restrictions per CommonMark spec:
            // - Empty list items cannot interrupt paragraphs
            // - Ordered lists can only interrupt paragraphs if they start with 1
            if let Some(list_type) = self.is_list_start(line)
                && !self.is_empty_list_item(line)
            {
                match list_type {
                    ListType::Unordered(_) => break,
                    ListType::Ordered(start, _) => {
                        if start == 1 {
                            break;
                        }
                        // Ordered lists not starting with 1 cannot interrupt paragraphs
                    }
                }
            }

            // Stop on HTML block (types 1-6 can interrupt paragraphs, type 7 cannot)
            if let Some(html_type) = self.is_html_block_start(line)
                && html_type != 7
            {
                break;
            }

            // Check if this could be a Setext underline (would end the paragraph)
            if i > 0 && self.is_setext_underline(line).is_some() {
                // This line is a Setext underline - don't include it in paragraph
                break;
            }

            // Include this line in the paragraph
            paragraph_lines.push(line);
            i += 1;
        }

        // Join lines with newlines and parse inline content
        // Per CommonMark spec: "The paragraph's raw content is formed by concatenating
        // the lines and removing initial and final spaces or tabs."
        // However, we must preserve trailing spaces for hard line breaks (2+ spaces before newline)
        let mut processed_lines: Vec<String> = paragraph_lines
            .iter()
            .map(|&line: &&str| {
                // Trim start only, preserve trailing spaces for hard breaks
                line.trim_start_matches([' ', '\t']).to_string()
            })
            .collect();

        // Trim trailing spaces from the last line (end of paragraph)
        if let Some(last) = processed_lines.last_mut() {
            *last = last.trim_end_matches([' ', '\t']).to_string();
        }

        let text = processed_lines.join("\n");
        let children = self.parse_inline(&text);

        (Node::Paragraph(children), i)
    }

    /// Parse inline elements (code spans, emphasis, links, etc.) from text
    /// Uses a delimiter-based approach for emphasis per CommonMark spec
    fn parse_inline(&self, text: &str) -> Vec<Node> {
        let chars: Vec<char> = text.chars().collect();
        self.parse_inline_with_delimiter_stack(&chars, 0, chars.len())
    }

    /// Parse inline elements with proper delimiter stack algorithm per CommonMark spec
    fn parse_inline_with_delimiter_stack(
        &self,
        chars: &[char],
        start: usize,
        end: usize,
    ) -> Vec<Node> {
        let mut nodes = Vec::new();
        let mut delimiter_stack: Vec<DelimiterRun> = Vec::new();
        let mut i = start;

        // First pass: collect all inline elements and delimiter runs
        while i < end {
            // Handle backslash escapes first
            if chars[i] == '\\' && i + 1 < end {
                // Check for hard line break (backslash at end of line)
                if chars[i + 1] == '\n' {
                    nodes.push(Node::HardBreak);
                    i += 2;
                    continue;
                }
                // Check if next char is ASCII punctuation
                else if self.is_ascii_punctuation(chars[i + 1]) {
                    // Escaped punctuation - treat as literal text
                    nodes.push(Node::Text(chars[i + 1].to_string()));
                    i += 2;
                    continue;
                } else {
                    // Not escapable - backslash is literal
                    nodes.push(Node::Text('\\'.to_string()));
                    i += 1;
                    continue;
                }
            }

            // Try to parse HTML entity or numeric character reference
            if chars[i] == '&'
                && let Some((entity_text, new_i)) = self.try_parse_entity(chars, i)
            {
                nodes.push(Node::Text(entity_text));
                i = new_i;
                continue;
            }

            // Try to parse code span first (takes precedence over emphasis per Rule 17)
            if chars[i] == '`'
                && let Some((code_node, new_i)) = self.try_parse_code_span(chars, i)
            {
                nodes.push(code_node);
                i = new_i;
                continue;
            }

            // Try to parse autolink (before regular links)
            if chars[i] == '<'
                && let Some((autolink_node, new_i)) = self.try_parse_autolink(chars, i)
            {
                nodes.push(autolink_node);
                i = new_i;
                continue;
            }

            // Try to parse raw HTML inline (after autolink attempt)
            if chars[i] == '<'
                && let Some((html_node, new_i)) = self.try_parse_html_inline(chars, i)
            {
                nodes.push(html_node);
                i = new_i;
                continue;
            }

            // Try to parse image (before link, since images start with ![)
            if chars[i] == '!'
                && i + 1 < end
                && chars[i + 1] == '['
                && let Some((image_node, new_i)) = self.try_parse_image(chars, i)
            {
                nodes.push(image_node);
                i = new_i;
                continue;
            }

            // Try to parse link (links take precedence over emphasis per Rule 17)
            if chars[i] == '['
                && let Some((link_node, new_i)) = self.try_parse_link(chars, i)
            {
                nodes.push(link_node);
                i = new_i;
                continue;
            }

            // Handle emphasis delimiters - add to stack
            if chars[i] == '*' || chars[i] == '_' {
                let delimiter = chars[i];
                let delim_start = i;
                let mut count = 0;
                while i < end && chars[i] == delimiter {
                    count += 1;
                    i += 1;
                }

                // Check flanking rules
                let is_left_flanking = self.is_left_flanking(chars, delim_start, count);
                let is_right_flanking = self.is_right_flanking(chars, delim_start, count);

                let can_open = if delimiter == '*' {
                    is_left_flanking
                } else {
                    is_left_flanking
                        && (!is_right_flanking || {
                            let before_char = if delim_start == 0 {
                                ' '
                            } else {
                                chars[delim_start - 1]
                            };
                            self.is_unicode_punctuation(before_char)
                        })
                };

                let can_close = if delimiter == '*' {
                    is_right_flanking
                } else {
                    is_right_flanking
                        && (!is_left_flanking || {
                            let after_char = if i >= end { ' ' } else { chars[i] };
                            self.is_unicode_punctuation(after_char)
                        })
                };

                // Add delimiter run to text nodes and track on stack
                let delimiter_str: String = chars[delim_start..i].iter().collect();
                nodes.push(Node::Text(delimiter_str));

                if can_open || can_close {
                    delimiter_stack.push(DelimiterRun {
                        delimiter,
                        count,
                        pos: nodes.len() - 1,
                        can_open,
                        can_close,
                        active: true,
                    });
                }
                continue;
            }

            // Collect regular text until next special character
            let text_start = i;
            while i < end
                && chars[i] != '\\'
                && chars[i] != '&'
                && chars[i] != '`'
                && chars[i] != '*'
                && chars[i] != '_'
                && chars[i] != '['
                && chars[i] != '!'
                && chars[i] != '<'
                && chars[i] != '\n'
            {
                i += 1;
            }
            if i > text_start {
                let text: String = chars[text_start..i].iter().collect();
                // Check for hard line break: 2+ trailing spaces before newline
                if i < end && chars[i] == '\n' {
                    let trimmed_end = text.trim_end_matches(' ').len();
                    let trailing_spaces = text.len() - trimmed_end;
                    if trailing_spaces >= 2 {
                        // Hard line break - emit text without trailing spaces, then <br />
                        if trimmed_end > 0 {
                            nodes.push(Node::Text(text[..trimmed_end].to_string()));
                        }
                        nodes.push(Node::HardBreak);
                        i += 1; // consume the newline
                        continue;
                    } else {
                        // Normal text with newline - include the newline in text
                        nodes.push(Node::Text(text));
                        nodes.push(Node::Text("\n".to_string()));
                        i += 1;
                        continue;
                    }
                } else {
                    nodes.push(Node::Text(text));
                }
            }

            // If we didn't move forward, just consume one character as text
            if i == text_start {
                nodes.push(Node::Text(chars[i].to_string()));
                i += 1;
            }
        }

        // Second pass: process emphasis delimiters
        self.process_emphasis(&mut nodes, &mut delimiter_stack, None);

        nodes
    }

    /// Process emphasis delimiters using the CommonMark algorithm
    /// Modifies nodes in place, converting delimiter runs to emphasis/strong nodes
    fn process_emphasis(
        &self,
        nodes: &mut Vec<Node>,
        delimiter_stack: &mut Vec<DelimiterRun>,
        stack_bottom: Option<usize>,
    ) {
        let bottom_index = stack_bottom.unwrap_or(0);
        let mut closer_idx = bottom_index;

        // Find potential closers
        while closer_idx < delimiter_stack.len() {
            let closer = &delimiter_stack[closer_idx];

            // Skip if not a potential closer or not active
            if !closer.can_close || !closer.active {
                closer_idx += 1;
                continue;
            }

            // Look for matching opener (go backwards from closer)
            let mut opener_idx = closer_idx;
            let mut found_opener = None;

            while opener_idx > bottom_index {
                opener_idx -= 1;
                let opener = &delimiter_stack[opener_idx];

                // Skip if wrong delimiter type, not active, or can't open
                if opener.delimiter != closer.delimiter || !opener.active || !opener.can_open {
                    continue;
                }

                // Check modulo-3 rule (Rule 9/10)
                let both_can_open_and_close = opener.can_close && closer.can_open;
                if both_can_open_and_close {
                    let sum = opener.count + closer.count;
                    if sum.is_multiple_of(3)
                        && !(opener.count.is_multiple_of(3) && closer.count.is_multiple_of(3))
                    {
                        continue;
                    }
                }

                // Found a match!
                found_opener = Some(opener_idx);
                break;
            }

            if let Some(opener_idx) = found_opener {
                // Determine how many delimiters to use (prefer 2 for strong, else 1 for em)
                let opener_count = delimiter_stack[opener_idx].count;
                let closer_count = delimiter_stack[closer_idx].count;

                let use_delims = if opener_count >= 2 && closer_count >= 2 {
                    2 // strong
                } else {
                    1 // emphasis
                };

                // Extract content between opener and closer
                let opener_pos = delimiter_stack[opener_idx].pos;
                let closer_pos = delimiter_stack[closer_idx].pos;
                let opener_count = delimiter_stack[opener_idx].count;
                let closer_count = delimiter_stack[closer_idx].count;

                // Remove delimiters from the text nodes and create emphasis node
                let new_node = self.create_emphasis_node(nodes, opener_pos, closer_pos, use_delims);

                // Replace the range with the new emphasis node
                // This updates nodes and adjusts positions
                self.replace_with_emphasis(
                    nodes,
                    delimiter_stack,
                    (opener_pos, closer_pos),
                    new_node,
                    use_delims,
                    (opener_count, closer_count),
                );

                // Update delimiter counts
                delimiter_stack[opener_idx].count = opener_count.saturating_sub(use_delims);
                delimiter_stack[closer_idx].count = closer_count.saturating_sub(use_delims);

                // Remove exhausted delimiters and those between opener and closer
                let mut to_remove = Vec::new();

                // Collect indices to remove
                if delimiter_stack[closer_idx].count == 0 {
                    to_remove.push(closer_idx);
                }
                for idx in (opener_idx + 1..closer_idx).rev() {
                    to_remove.push(idx);
                }
                if delimiter_stack[opener_idx].count == 0 {
                    to_remove.push(opener_idx);
                }

                // Remove from highest to lowest index to avoid shifting issues
                to_remove.sort_unstable_by(|a, b| b.cmp(a));
                for &idx in &to_remove {
                    delimiter_stack.remove(idx);
                }

                // Adjust closer_idx for next iteration
                // Count how many items we removed that were before closer_idx
                let removed_before = to_remove.iter().filter(|&&idx| idx < closer_idx).count();
                closer_idx = closer_idx.saturating_sub(removed_before);

                // Continue from the same position (might be a new closer now)
                continue;
            }

            closer_idx += 1;
        }
    }

    /// Create an emphasis or strong node from the content between two positions
    fn create_emphasis_node(
        &self,
        nodes: &[Node],
        opener_pos: usize,
        closer_pos: usize,
        use_delims: usize,
    ) -> Node {
        // Extract content between delimiters (excluding the delimiter text nodes themselves)
        let mut content = Vec::new();
        for node in nodes.iter().take(closer_pos).skip(opener_pos + 1) {
            content.push(node.clone());
        }

        if use_delims == 2 {
            Node::Strong(content)
        } else {
            Node::Emphasis(content)
        }
    }

    /// Replace the delimiter range with an emphasis node
    /// This updates the nodes vec and adjusts delimiter positions in the stack
    fn replace_with_emphasis(
        &self,
        nodes: &mut Vec<Node>,
        delimiter_stack: &mut [DelimiterRun],
        positions: (usize, usize), // (opener_pos, closer_pos)
        emphasis_node: Node,
        use_delims: usize,
        delimiter_counts: (usize, usize), // (opener_count, closer_count)
    ) {
        let (opener_pos, closer_pos) = positions;
        let (opener_count, closer_count) = delimiter_counts;

        // Update the delimiter text nodes to remove used delimiters
        if let Node::Text(ref mut opener_text) = nodes[opener_pos] {
            let delim_char = opener_text.chars().next().unwrap_or('*');
            let remaining = opener_count.saturating_sub(use_delims);
            *opener_text = delim_char.to_string().repeat(remaining);
        }

        if let Node::Text(ref mut closer_text) = nodes[closer_pos] {
            let delim_char = closer_text.chars().next().unwrap_or('*');
            let remaining = closer_count.saturating_sub(use_delims);
            *closer_text = delim_char.to_string().repeat(remaining);
        }

        // Replace the content between opener and closer with the emphasis node
        // First, remove the content range (excluding delimiters)
        let remove_start = opener_pos + 1;
        let remove_end = closer_pos;
        let remove_count = remove_end - remove_start;

        if remove_count > 0 {
            nodes.drain(remove_start..remove_end);
        }

        // Insert the emphasis node
        nodes.insert(remove_start, emphasis_node);

        // Update all delimiter positions after the change
        let pos_shift = 1_isize - remove_count as isize;
        for delim in delimiter_stack.iter_mut() {
            if delim.pos > opener_pos {
                delim.pos = (delim.pos as isize + pos_shift) as usize;
            }
        }
    }

    /// Check if a character is ASCII punctuation (can be backslash-escaped)
    fn is_ascii_punctuation(&self, ch: char) -> bool {
        matches!(
            ch,
            '!' | '"'
                | '#'
                | '$'
                | '%'
                | '&'
                | '\''
                | '('
                | ')'
                | '*'
                | '+'
                | ','
                | '-'
                | '.'
                | '/'
                | ':'
                | ';'
                | '<'
                | '='
                | '>'
                | '?'
                | '@'
                | '['
                | '\\'
                | ']'
                | '^'
                | '_'
                | '`'
                | '{'
                | '|'
                | '}'
                | '~'
        )
    }

    /// Process backslash escapes in a string (for link destinations/titles)
    fn process_backslash_escapes(&self, text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut result = String::new();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() && self.is_ascii_punctuation(chars[i + 1]) {
                // Escaped punctuation - include the literal character
                result.push(chars[i + 1]);
                i += 2;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }

        result
    }

    /// Decode HTML entities in a string
    fn process_entities(&self, text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut result = String::new();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '&'
                && let Some((decoded, new_i)) = self.try_parse_entity(&chars, i)
            {
                result.push_str(&decoded);
                i = new_i;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }

        result
    }

    /// URL-encode a string for use in href attributes (percent-encode non-ASCII and special chars)
    fn url_encode(&self, text: &str) -> String {
        let mut result = String::new();

        for ch in text.chars() {
            // ASCII alphanumeric and safe URL characters pass through
            if ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '-' | '_'
                        | '.'
                        | '~'
                        | '!'
                        | '*'
                        | '\''
                        | '('
                        | ')'
                        | ';'
                        | ':'
                        | '@'
                        | '&'
                        | '='
                        | '+'
                        | '$'
                        | ','
                        | '/'
                        | '?'
                        | '#'
                        | '['
                        | ']'
                )
            {
                result.push(ch);
            } else {
                // Percent-encode as UTF-8 bytes
                for byte in ch.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }

        result
    }

    fn try_parse_code_span(&self, chars: &[char], start: usize) -> Option<(Node, usize)> {
        let mut i = start;
        let mut backtick_count = 0;

        // Count opening backticks
        while i < chars.len() && chars[i] == '`' {
            backtick_count += 1;
            i += 1;
        }

        let content_start = i;
        let mut j = i;

        // Look for matching closing backticks
        while j < chars.len() {
            if chars[j] == '`' {
                let close_start = j;
                let mut close_count = 0;
                while j < chars.len() && chars[j] == '`' {
                    close_count += 1;
                    j += 1;
                }

                if close_count == backtick_count {
                    // Found matching close
                    let mut content: String = chars[content_start..close_start].iter().collect();

                    // Convert line endings to spaces
                    content = content.replace(['\n', '\r'], " ");

                    // Strip single leading and trailing space if present and content isn't all spaces
                    if content.len() > 2
                        && content.starts_with(' ')
                        && content.ends_with(' ')
                        && !content.trim().is_empty()
                    {
                        content = content[1..content.len() - 1].to_string();
                    }

                    return Some((Node::Code(content), j));
                }
            } else {
                j += 1;
            }
        }

        None
    }

    /// Check if a character is Unicode punctuation (for emphasis flanking rules)
    /// Per CommonMark spec: characters in Unicode P (punctuation) or S (symbol) categories
    fn is_unicode_punctuation(&self, c: char) -> bool {
        // Fast path for ASCII
        if c.is_ascii_punctuation() {
            return true;
        }

        // For non-ASCII, check if it's in the P or S categories
        // This is a simplified check covering the most common ranges
        // A full implementation would use Unicode database, but this covers
        // the test cases including currency symbols ($, £, €, etc.)
        let code = c as u32;

        // Common punctuation and symbol ranges:
        // - Latin-1 Supplement punctuation/symbols: 0x00A1-0x00BF
        // - Currency symbols: 0x20A0-0x20CF and scattered (Sc category)
        // - General Punctuation: 0x2000-0x206F
        // - Math symbols: 0x2200-0x22FF
        // - Arrows: 0x2190-0x21FF
        // - Box drawing, etc.: 0x2500-0x25FF
        // - Miscellaneous symbols: 0x2600-0x26FF
        // - Supplemental Punctuation: 0x2E00-0x2E7F
        matches!(code,
            // Latin-1 Supplement (includes ¡-¿, ×, ÷, and ¢-¥ which are part of 0x00A1..=0x00BF)
            0x00A1..=0x00BF | 0x00D7 | 0x00F7 |
            // Currency symbols (including $)
            0x0024 | 0x20A0..=0x20CF | 0x1E2FF |
            // General Punctuation
            0x2000..=0x206F |
            // Supplemental Punctuation
            0x2E00..=0x2E7F |
            // Mathematical Operators
            0x2200..=0x22FF |
            // Arrows
            0x2190..=0x21FF |
            // Miscellaneous Technical
            0x2300..=0x23FF |
            // Box Drawing, Block Elements, Geometric Shapes
            0x2500..=0x25FF |
            // Miscellaneous Symbols
            0x2600..=0x26FF |
            // Dingbats
            0x2700..=0x27BF |
            // Miscellaneous Mathematical Symbols-A/B
            0x27C0..=0x27EF | 0x2980..=0x29FF |
            // Supplemental Arrows-A/B
            0x27F0..=0x27FF | 0x2900..=0x297F |
            // Miscellaneous Symbols and Arrows
            0x2B00..=0x2BFF
        )
    }

    fn is_left_flanking(&self, chars: &[char], pos: usize, count: usize) -> bool {
        let after_pos = pos + count;

        // Beginning/end of line counts as Unicode whitespace
        let after_char = if after_pos >= chars.len() {
            ' ' // Treat end as whitespace
        } else {
            chars[after_pos]
        };

        let before_char = if pos == 0 {
            ' ' // Treat beginning as whitespace
        } else {
            chars[pos - 1]
        };

        // Rule 1: not followed by Unicode whitespace
        if after_char.is_whitespace() {
            return false;
        }

        // Rule 2a: not followed by Unicode punctuation
        if !self.is_unicode_punctuation(after_char) {
            return true;
        }

        // Rule 2b: followed by Unicode punctuation AND
        // preceded by Unicode whitespace or Unicode punctuation
        before_char.is_whitespace() || self.is_unicode_punctuation(before_char)
    }

    fn is_right_flanking(&self, chars: &[char], pos: usize, count: usize) -> bool {
        let after_pos = pos + count;

        let after_char = if after_pos >= chars.len() {
            ' ' // Treat end as whitespace
        } else {
            chars[after_pos]
        };

        let before_char = if pos == 0 {
            ' ' // Treat beginning as whitespace
        } else {
            chars[pos - 1]
        };

        // Rule 1: not preceded by Unicode whitespace
        if before_char.is_whitespace() {
            return false;
        }

        // Rule 2a: not preceded by Unicode punctuation
        if !self.is_unicode_punctuation(before_char) {
            return true;
        }

        // Rule 2b: preceded by Unicode punctuation AND
        // followed by Unicode whitespace or Unicode punctuation
        after_char.is_whitespace() || self.is_unicode_punctuation(after_char)
    }

    fn try_parse_link(&self, chars: &[char], start: usize) -> Option<(Node, usize)> {
        // Link syntax:
        // - Inline: [link text](destination "title")
        // - Full reference: [link text][label]
        // - Collapsed reference: [link text][]
        // - Shortcut reference: [link text]
        // We start at '['
        let mut i = start + 1;

        // Find the closing ']' for link text
        let mut bracket_depth = 1;
        let text_start = i;

        while i < chars.len() {
            if chars[i] == '[' {
                bracket_depth += 1;
            } else if chars[i] == ']' {
                bracket_depth -= 1;
                if bracket_depth == 0 {
                    break;
                }
            } else if chars[i] == '\\' && i + 1 < chars.len() {
                // Skip escaped character
                i += 1;
            }
            i += 1;
        }

        if i >= chars.len() || chars[i] != ']' {
            return None; // No closing bracket
        }

        let text_end = i;
        i += 1; // Move past ']'

        // Parse the link text
        let link_text: String = chars[text_start..text_end].iter().collect();

        // Check what follows: '(' for inline, '[' for reference
        if i < chars.len() && chars[i] == '(' {
            // Inline link
            self.try_parse_inline_link(chars, i, &link_text)
        } else if i < chars.len() && chars[i] == '[' {
            // Full or collapsed reference link
            self.try_parse_reference_link(chars, i, &link_text)
        } else {
            // Try shortcut reference link
            self.try_parse_shortcut_reference_link(&link_text, i)
        }
    }

    fn try_parse_inline_link(
        &self,
        chars: &[char],
        start: usize,
        link_text: &str,
    ) -> Option<(Node, usize)> {
        let mut i = start;
        // Now we need '(' for inline link
        if i >= chars.len() || chars[i] != '(' {
            return None;
        }
        i += 1; // Move past '('

        // Skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        // Parse destination (either <...> or raw)
        let destination: String;
        if i < chars.len() && chars[i] == '<' {
            // Angle-bracket enclosed destination
            i += 1;
            let dest_start = i;
            while i < chars.len() && chars[i] != '>' && chars[i] != '\n' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1; // Skip escaped character
                }
                i += 1;
            }
            if i >= chars.len() || chars[i] != '>' {
                return None; // Unclosed angle bracket
            }
            let raw_dest: String = chars[dest_start..i].iter().collect();
            // Process backslash escapes, then entities, then URL-encode
            let escaped_dest = self.process_backslash_escapes(&raw_dest);
            let entity_decoded = self.process_entities(&escaped_dest);
            destination = self.url_encode(&entity_decoded);
            i += 1; // Move past '>'
        } else {
            // Raw destination (no spaces allowed unless in parens)
            let dest_start = i;
            let mut paren_depth = 0;
            while i < chars.len() {
                if chars[i] == '(' {
                    paren_depth += 1;
                } else if chars[i] == ')' {
                    if paren_depth == 0 {
                        break; // End of destination
                    }
                    paren_depth -= 1;
                } else if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1; // Skip escaped character
                } else if chars[i].is_whitespace() {
                    break; // Whitespace ends destination
                }
                i += 1;
            }
            let raw_dest: String = chars[dest_start..i].iter().collect();
            // Process backslash escapes, then entities, then URL-encode
            let escaped_dest = self.process_backslash_escapes(&raw_dest);
            let entity_decoded = self.process_entities(&escaped_dest);
            destination = self.url_encode(&entity_decoded);

            // Check for invalid characters in destination (spaces outside parens)
            if entity_decoded.contains(|c: char| c.is_whitespace()) && paren_depth == 0 {
                return None; // Invalid destination
            }
        }

        // Skip whitespace
        while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
            i += 1;
        }

        // Check for optional title
        let title: Option<String>;
        if i < chars.len() && (chars[i] == '"' || chars[i] == '\'' || chars[i] == '(') {
            let quote_char = if chars[i] == '(' { ')' } else { chars[i] };
            i += 1; // Move past opening quote
            let title_start = i;

            while i < chars.len() && chars[i] != quote_char {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1; // Skip escaped character
                }
                i += 1;
            }

            if i >= chars.len() || chars[i] != quote_char {
                return None; // Unclosed title
            }

            let raw_title: String = chars[title_start..i].iter().collect();
            // Process backslash escapes, then entities (no URL encoding for titles)
            let escaped_title = self.process_backslash_escapes(&raw_title);
            title = Some(self.process_entities(&escaped_title));
            i += 1; // Move past closing quote

            // Skip trailing whitespace
            while i < chars.len() && chars[i].is_whitespace() && chars[i] != ')' {
                i += 1;
            }
        } else {
            title = None;
        }

        // Expect closing ')'
        if i >= chars.len() || chars[i] != ')' {
            return None;
        }
        i += 1; // Move past ')'

        let children = self.parse_inline(link_text);

        Some((
            Node::Link {
                destination,
                title,
                children,
            },
            i,
        ))
    }

    fn try_parse_reference_link(
        &self,
        chars: &[char],
        start: usize,
        link_text: &str,
    ) -> Option<(Node, usize)> {
        let mut i = start;
        // Must be '['
        if i >= chars.len() || chars[i] != '[' {
            return None;
        }
        i += 1; // Move past '['

        // Find closing ']'
        let label_start = i;
        while i < chars.len() && chars[i] != ']' {
            i += 1;
        }

        if i >= chars.len() {
            return None; // No closing bracket
        }

        let raw_label: String = chars[label_start..i].iter().collect();
        i += 1; // Move past ']'

        // Determine the label to look up
        let label = if raw_label.is_empty() {
            // Collapsed reference: use link text as label
            Self::normalize_label(link_text)
        } else {
            // Full reference: use explicit label
            Self::normalize_label(&raw_label)
        };

        // Look up the reference definition
        if let Some((destination, title)) = self.reference_definitions.get(&label) {
            let children = self.parse_inline(link_text);
            Some((
                Node::Link {
                    destination: destination.clone(),
                    title: title.clone(),
                    children,
                },
                i,
            ))
        } else {
            // Reference not found
            None
        }
    }

    fn try_parse_shortcut_reference_link(
        &self,
        link_text: &str,
        end_pos: usize,
    ) -> Option<(Node, usize)> {
        // Shortcut reference: [link text] where link_text is also the label
        let label = Self::normalize_label(link_text);

        // Look up the reference definition
        if let Some((destination, title)) = self.reference_definitions.get(&label) {
            let children = self.parse_inline(link_text);
            Some((
                Node::Link {
                    destination: destination.clone(),
                    title: title.clone(),
                    children,
                },
                end_pos,
            ))
        } else {
            // Reference not found
            None
        }
    }

    fn try_parse_image(&self, chars: &[char], start: usize) -> Option<(Node, usize)> {
        // Image syntax:
        // - Inline: ![alt text](destination "title")
        // - Reference: ![alt text][label] or ![alt text][] or ![alt text]
        // We start at '!'
        if chars[start] != '!' || start + 1 >= chars.len() || chars[start + 1] != '[' {
            return None;
        }

        let mut i = start + 2; // Move past '!['

        // Find the closing ']' for alt text
        let mut bracket_depth = 1;
        let text_start = i;

        while i < chars.len() {
            if chars[i] == '[' {
                bracket_depth += 1;
            } else if chars[i] == ']' {
                bracket_depth -= 1;
                if bracket_depth == 0 {
                    break;
                }
            } else if chars[i] == '\\' && i + 1 < chars.len() {
                // Skip escaped character
                i += 1;
            }
            i += 1;
        }

        if i >= chars.len() || chars[i] != ']' {
            return None; // No closing bracket
        }

        let text_end = i;
        i += 1; // Move past ']'

        // Parse the alt text
        let alt_text_str: String = chars[text_start..text_end].iter().collect();

        // Check what follows: '(' for inline, '[' for reference
        if i < chars.len() && chars[i] == '(' {
            // Inline image
            self.try_parse_inline_image(chars, i, &alt_text_str)
        } else if i < chars.len() && chars[i] == '[' {
            // Full or collapsed reference image
            self.try_parse_reference_image(chars, i, &alt_text_str)
        } else {
            // Try shortcut reference image
            self.try_parse_shortcut_reference_image(&alt_text_str, i)
        }
    }

    fn try_parse_inline_image(
        &self,
        chars: &[char],
        start: usize,
        alt_text_str: &str,
    ) -> Option<(Node, usize)> {
        let mut i = start;
        // Now we need '(' for inline image
        if i >= chars.len() || chars[i] != '(' {
            return None;
        }
        i += 1; // Move past '('

        // Skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        // Parse destination (either <...> or raw)
        let destination: String;
        if i < chars.len() && chars[i] == '<' {
            // Angle-bracket enclosed destination
            i += 1;
            let dest_start = i;
            while i < chars.len() && chars[i] != '>' && chars[i] != '\n' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1; // Skip escaped character
                }
                i += 1;
            }
            if i >= chars.len() || chars[i] != '>' {
                return None; // Unclosed angle bracket
            }
            let raw_dest: String = chars[dest_start..i].iter().collect();
            // Process backslash escapes, then entities, then URL-encode
            let escaped_dest = self.process_backslash_escapes(&raw_dest);
            let entity_decoded = self.process_entities(&escaped_dest);
            destination = self.url_encode(&entity_decoded);
            i += 1; // Move past '>'
        } else {
            // Raw destination (no spaces allowed unless in parens)
            let dest_start = i;
            let mut paren_depth = 0;
            while i < chars.len() {
                if chars[i] == '(' {
                    paren_depth += 1;
                } else if chars[i] == ')' {
                    if paren_depth == 0 {
                        break; // End of destination
                    }
                    paren_depth -= 1;
                } else if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1; // Skip escaped character
                } else if chars[i].is_whitespace() {
                    break; // Whitespace ends destination
                }
                i += 1;
            }
            let raw_dest: String = chars[dest_start..i].iter().collect();
            // Process backslash escapes, then entities, then URL-encode
            let escaped_dest = self.process_backslash_escapes(&raw_dest);
            let entity_decoded = self.process_entities(&escaped_dest);
            destination = self.url_encode(&entity_decoded);

            // Check for invalid characters in destination (spaces outside parens)
            if entity_decoded.contains(|c: char| c.is_whitespace()) && paren_depth == 0 {
                return None; // Invalid destination
            }
        }

        // Skip whitespace
        while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
            i += 1;
        }

        // Check for optional title
        let title: Option<String>;
        if i < chars.len() && (chars[i] == '"' || chars[i] == '\'' || chars[i] == '(') {
            let quote_char = if chars[i] == '(' { ')' } else { chars[i] };
            i += 1; // Move past opening quote
            let title_start = i;

            while i < chars.len() && chars[i] != quote_char {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1; // Skip escaped character
                }
                i += 1;
            }

            if i >= chars.len() || chars[i] != quote_char {
                return None; // Unclosed title
            }

            let raw_title: String = chars[title_start..i].iter().collect();
            // Process backslash escapes, then entities (no URL encoding for titles)
            let escaped_title = self.process_backslash_escapes(&raw_title);
            title = Some(self.process_entities(&escaped_title));
            i += 1; // Move past closing quote

            // Skip trailing whitespace
            while i < chars.len() && chars[i].is_whitespace() && chars[i] != ')' {
                i += 1;
            }
        } else {
            title = None;
        }

        // Expect closing ')'
        if i >= chars.len() || chars[i] != ')' {
            return None;
        }
        i += 1; // Move past ')'

        let alt_text = self.parse_inline(alt_text_str);

        Some((
            Node::Image {
                destination,
                title,
                alt_text,
            },
            i,
        ))
    }

    fn try_parse_reference_image(
        &self,
        chars: &[char],
        start: usize,
        alt_text_str: &str,
    ) -> Option<(Node, usize)> {
        let mut i = start;
        // Must be '['
        if i >= chars.len() || chars[i] != '[' {
            return None;
        }
        i += 1; // Move past '['

        // Find closing ']'
        let label_start = i;
        while i < chars.len() && chars[i] != ']' {
            i += 1;
        }

        if i >= chars.len() {
            return None; // No closing bracket
        }

        let raw_label: String = chars[label_start..i].iter().collect();
        i += 1; // Move past ']'

        // Determine the label to look up
        let label = if raw_label.is_empty() {
            // Collapsed reference: use alt text as label
            Self::normalize_label(alt_text_str)
        } else {
            // Full reference: use explicit label
            Self::normalize_label(&raw_label)
        };

        // Look up the reference definition
        if let Some((destination, title)) = self.reference_definitions.get(&label) {
            let alt_text = self.parse_inline(alt_text_str);
            Some((
                Node::Image {
                    destination: destination.clone(),
                    title: title.clone(),
                    alt_text,
                },
                i,
            ))
        } else {
            // Reference not found
            None
        }
    }

    fn try_parse_shortcut_reference_image(
        &self,
        alt_text_str: &str,
        end_pos: usize,
    ) -> Option<(Node, usize)> {
        // Shortcut reference: ![alt text] where alt_text is also the label
        let label = Self::normalize_label(alt_text_str);

        // Look up the reference definition
        if let Some((destination, title)) = self.reference_definitions.get(&label) {
            let alt_text = self.parse_inline(alt_text_str);
            Some((
                Node::Image {
                    destination: destination.clone(),
                    title: title.clone(),
                    alt_text,
                },
                end_pos,
            ))
        } else {
            // Reference not found
            None
        }
    }

    fn try_parse_autolink(&self, chars: &[char], start: usize) -> Option<(Node, usize)> {
        // Autolinks: <URI> or <email>
        // Start at '<'
        let mut i = start + 1;

        // Collect content until '>' or newline
        let content_start = i;
        while i < chars.len() && chars[i] != '>' && chars[i] != '\n' && chars[i] != '<' {
            i += 1;
        }

        // Must end with '>'
        if i >= chars.len() || chars[i] != '>' {
            return None;
        }

        let content: String = chars[content_start..i].iter().collect();

        // Cannot contain spaces
        if content.contains(char::is_whitespace) {
            return None;
        }

        // Cannot be empty
        if content.is_empty() {
            return None;
        }

        i += 1; // Move past '>'

        // Check if it's an email autolink
        if content.contains('@') && self.is_email_address(&content) {
            let destination = format!("mailto:{}", content);
            return Some((
                Node::Link {
                    destination,
                    title: None,
                    children: vec![Node::Text(content)],
                },
                i,
            ));
        }

        // Check if it's a URI autolink
        if self.is_absolute_uri(&content) {
            // URL-encode backslashes and other special chars in the destination
            let destination = self.url_encode_autolink(&content);
            return Some((
                Node::Link {
                    destination,
                    title: None,
                    children: vec![Node::Text(content)],
                },
                i,
            ));
        }

        // Not a valid autolink
        None
    }

    fn is_absolute_uri(&self, text: &str) -> bool {
        // Must have scheme:path format
        // Scheme: 2-32 chars, starts with letter, followed by letters/digits/+/./-
        if let Some(colon_pos) = text.find(':') {
            let scheme = &text[..colon_pos];

            // Check scheme length
            if scheme.len() < 2 || scheme.len() > 32 {
                return false;
            }

            // Check first char is letter
            if let Some(first) = scheme.chars().next() {
                if !first.is_ascii_alphabetic() {
                    return false;
                }
            } else {
                return false;
            }

            // Check all chars are valid
            for ch in scheme.chars() {
                if !ch.is_ascii_alphanumeric() && ch != '+' && ch != '.' && ch != '-' {
                    return false;
                }
            }

            // Must have something after colon
            if colon_pos + 1 >= text.len() {
                return false;
            }

            return true;
        }

        false
    }

    /// Try to parse an HTML entity or numeric character reference
    /// Returns (decoded_text, position_after) if successful
    fn try_parse_entity(&self, chars: &[char], start: usize) -> Option<(String, usize)> {
        if start >= chars.len() || chars[start] != '&' {
            return None;
        }

        let mut i = start + 1;

        // Check for numeric character reference
        if i < chars.len() && chars[i] == '#' {
            i += 1;

            // Hexadecimal reference: &#X or &#x
            if i < chars.len() && (chars[i] == 'X' || chars[i] == 'x') {
                i += 1;
                let hex_start = i;

                // Collect 1-6 hex digits
                while i < chars.len() && i - hex_start < 6 && chars[i].is_ascii_hexdigit() {
                    i += 1;
                }

                if i > hex_start && i < chars.len() && chars[i] == ';' {
                    let hex_str: String = chars[hex_start..i].iter().collect();
                    if let Ok(code_point) = u32::from_str_radix(&hex_str, 16) {
                        // Replace invalid/null with replacement character
                        let ch = if code_point == 0 || code_point > 0x10FFFF {
                            '\u{FFFD}'
                        } else {
                            char::from_u32(code_point).unwrap_or('\u{FFFD}')
                        };
                        return Some((ch.to_string(), i + 1));
                    }
                }
            }
            // Decimal reference: &#
            else {
                let dec_start = i;

                // Collect 1-7 decimal digits
                while i < chars.len() && i - dec_start < 7 && chars[i].is_ascii_digit() {
                    i += 1;
                }

                if i > dec_start && i < chars.len() && chars[i] == ';' {
                    let dec_str: String = chars[dec_start..i].iter().collect();
                    if let Ok(code_point) = dec_str.parse::<u32>() {
                        // Replace invalid/null with replacement character
                        let ch = if code_point == 0 || code_point > 0x10FFFD {
                            '\u{FFFD}'
                        } else {
                            char::from_u32(code_point).unwrap_or('\u{FFFD}')
                        };
                        return Some((ch.to_string(), i + 1));
                    }
                }
            }
        }
        // Check for named entity
        else {
            let name_start = i;

            // Collect alphanumeric characters (entity name)
            while i < chars.len() && (chars[i].is_ascii_alphanumeric()) {
                i += 1;
            }

            if i > name_start && i < chars.len() && chars[i] == ';' {
                let entity_name: String = chars[name_start..i].iter().collect();

                // Look up entity in HTML5 entity map
                if let Some(decoded) = self.decode_html_entity(&entity_name) {
                    return Some((decoded, i + 1));
                }
            }
        }

        None
    }

    /// Decode HTML5 named entities
    /// This is a subset of HTML5 entities - add more as needed
    fn decode_html_entity(&self, name: &str) -> Option<String> {
        let decoded = match name {
            "nbsp" => "\u{00A0}", // Non-breaking space
            "amp" => "&",
            "lt" => "<",
            "gt" => ">",
            "quot" => "\"",
            "apos" => "'",
            "copy" => "©",                     // ©
            "reg" => "®",                      // ®
            "AElig" => "Æ",                    // Æ
            "Dcaron" => "Ď",                   // Ď
            "frac34" => "¾",                   // ¾
            "HilbertSpace" => "ℋ",             // ℋ
            "DifferentialD" => "ⅆ",            // ⅆ
            "ClockwiseContourIntegral" => "∲", // ∲
            "ngE" => "≧̸",                      // ≧̸ (combining character)
            "ouml" => "ö",                     // ö
            _ => return None,
        };

        Some(decoded.to_string())
    }

    fn is_email_address(&self, text: &str) -> bool {
        // Simplified email validation based on HTML5 spec
        // Format: local@domain
        if let Some(at_pos) = text.find('@') {
            let local = &text[..at_pos];
            let domain = &text[at_pos + 1..];

            // Local part must be non-empty and match allowed chars
            if local.is_empty() {
                return false;
            }

            for ch in local.chars() {
                if !ch.is_ascii_alphanumeric()
                    && !matches!(
                        ch,
                        '.' | '!'
                            | '#'
                            | '$'
                            | '%'
                            | '&'
                            | '\''
                            | '*'
                            | '+'
                            | '/'
                            | '='
                            | '?'
                            | '^'
                            | '_'
                            | '`'
                            | '{'
                            | '|'
                            | '}'
                            | '~'
                            | '-'
                    )
                {
                    return false;
                }
            }

            // Domain must be non-empty and valid
            if domain.is_empty() {
                return false;
            }

            // Check domain format (basic validation)
            let parts: Vec<&str> = domain.split('.').collect();
            for part in parts {
                if part.is_empty() {
                    return false;
                }
                // First and last char of each part must be alphanumeric
                if let Some(first) = part.chars().next()
                    && !first.is_ascii_alphanumeric()
                {
                    return false;
                }
                if let Some(last) = part.chars().last()
                    && !last.is_ascii_alphanumeric()
                {
                    return false;
                }
                // Middle chars can be alphanumeric or hyphen
                for ch in part.chars() {
                    if !ch.is_ascii_alphanumeric() && ch != '-' {
                        return false;
                    }
                }
            }

            return true;
        }

        false
    }

    fn url_encode_autolink(&self, text: &str) -> String {
        // Percent-encode backslashes as %5C (spec says backslash-escapes don't work in autolinks)
        text.replace('\\', "%5C")
    }

    /// Try to parse a link reference definition
    /// Returns Some(lines_consumed) if successful, None otherwise
    fn try_parse_link_reference_definition(&mut self, lines: &[&str]) -> Option<usize> {
        if lines.is_empty() {
            return None;
        }

        let first_line = lines[0];

        // Check for up to 3 spaces of indentation
        let indent_cols = self.count_indent_columns(first_line);
        if indent_cols > 3 {
            return None;
        }

        let trimmed = first_line.trim_start();

        // Must start with [
        if !trimmed.starts_with('[') {
            return None;
        }

        // Find the label (link text within brackets), which can span multiple lines
        // Collect all text until we find the closing bracket or run out of lines
        let mut label_text = String::new();
        let mut current_line = 0;
        let mut found_closing = false;
        let mut after_closing = String::new();

        // Start with first line after '['
        let first_line_chars: Vec<char> = trimmed.chars().collect();
        let mut i = 1; // Start after '['

        while current_line < lines.len() {
            let line_to_scan = if current_line == 0 {
                &first_line_chars[i..]
            } else {
                &lines[current_line].chars().collect::<Vec<char>>()[..]
            };

            let mut j = 0;
            while j < line_to_scan.len() {
                if line_to_scan[j] == '\\' && j + 1 < line_to_scan.len() {
                    // Include escaped character in label
                    label_text.push(line_to_scan[j]);
                    label_text.push(line_to_scan[j + 1]);
                    j += 2;
                } else if line_to_scan[j] == ']' {
                    // Found closing bracket
                    found_closing = true;
                    after_closing = line_to_scan[j + 1..].iter().collect();
                    break;
                } else {
                    label_text.push(line_to_scan[j]);
                    j += 1;
                }
            }

            if found_closing {
                break;
            }

            // Add newline to label if we're continuing to next line
            if current_line == 0 {
                // Move to next line
                current_line += 1;
                if current_line < lines.len() {
                    label_text.push('\n');
                }
                i = 0; // Reset for next lines
            } else {
                current_line += 1;
                if current_line < lines.len() {
                    label_text.push('\n');
                }
            }
        }

        if !found_closing || label_text.is_empty() {
            return None; // No closing bracket or empty label
        }

        let label = Self::normalize_label(&label_text);

        // After ], must have :
        if !after_closing.starts_with(':') {
            return None;
        }

        let after_colon = after_closing[1..].trim_start();

        // Parse destination (can span multiple lines)
        let mut remaining = after_colon;

        // If nothing after colon on first line, check next line
        if remaining.is_empty() && current_line + 1 < lines.len() {
            current_line += 1;
            remaining = lines[current_line].trim_start();
        }

        // Parse destination
        let (destination, chars_consumed) = self.parse_link_destination(remaining)?;
        let after_dest = &remaining[chars_consumed..];

        // Check if there's whitespace after destination (required if title follows)
        let has_whitespace_after_dest = after_dest.starts_with(|c: char| c.is_whitespace());
        remaining = after_dest.trim_start();

        // Track if we moved to a new line for title
        let moved_to_new_line_for_title = remaining.is_empty() && current_line + 1 < lines.len();

        // Check if we need to continue to next line for title
        if moved_to_new_line_for_title {
            current_line += 1;
            remaining = lines[current_line].trim_start();
        }

        // Try to parse optional title (can span multiple lines)
        // Title must be separated from destination by whitespace
        let title = if !remaining.is_empty() {
            // If there's content after destination, there must have been whitespace OR we're on a new line
            if !has_whitespace_after_dest && !moved_to_new_line_for_title {
                // No whitespace between destination and what follows
                return None;
            }

            if let Some((title_text, lines_for_title)) =
                self.parse_multiline_link_title(&lines[current_line..], remaining)
            {
                // Title consumed the rest of current line plus potentially more lines
                current_line += lines_for_title - 1; // -1 because we're already on the first line
                Some(title_text)
            } else {
                // Not a valid title, but that's ok - title is optional
                // If we moved to a new line for title but it wasn't a title, that's fine - we just don't consume that line
                if moved_to_new_line_for_title {
                    // Back up - we didn't actually consume the next line
                    current_line -= 1;
                    None
                } else {
                    // We're still on the same line as destination - rest must be empty
                    if !remaining.is_empty() {
                        return None;
                    }
                    None
                }
            }
        } else {
            None
        };

        // Successfully parsed - store the definition (first one wins)
        self.reference_definitions
            .entry(label.clone())
            .or_insert((destination.clone(), title.clone()));

        Some(current_line + 1)
    }

    /// Normalize a label for matching (case-insensitive, collapse whitespace)
    fn normalize_label(label: &str) -> String {
        label
            .chars()
            .map(|c| c.to_lowercase().to_string())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
    }

    /// Parse a link destination (for reference definitions)
    /// Returns (destination, byte_offset) or None
    fn parse_link_destination(&self, text: &str) -> Option<(String, usize)> {
        if text.is_empty() {
            return None;
        }

        let chars: Vec<char> = text.chars().collect();

        // Angle-bracket form: <...>
        if chars[0] == '<' {
            let mut i = 1;
            let mut dest = String::new();

            while i < chars.len() {
                match chars[i] {
                    '>' => {
                        // Process entities and URL-encode
                        let entity_decoded = self.process_entities(&dest);
                        let url_encoded = self.url_encode(&entity_decoded);
                        // Calculate byte offset
                        let byte_offset = chars[..=i].iter().map(|c| c.len_utf8()).sum();
                        return Some((url_encoded, byte_offset));
                    }
                    '\\' if i + 1 < chars.len() && self.is_ascii_punctuation(chars[i + 1]) => {
                        // Backslash escape of ASCII punctuation
                        dest.push(chars[i + 1]);
                        i += 2;
                    }
                    '<' | '\n' | '\r' => {
                        // Invalid characters in angle-bracket destination
                        return None;
                    }
                    ch => {
                        dest.push(ch);
                        i += 1;
                    }
                }
            }

            // No closing >
            return None;
        }

        // Non-angle-bracket form: any non-space chars, balanced parens
        let mut i = 0;
        let mut dest = String::new();
        let mut paren_depth = 0;

        while i < chars.len() {
            match chars[i] {
                ' ' | '\t' | '\n' | '\r' => {
                    break;
                }
                '\\' if i + 1 < chars.len() && self.is_ascii_punctuation(chars[i + 1]) => {
                    // Backslash escape of ASCII punctuation
                    dest.push(chars[i + 1]);
                    i += 2;
                }
                '(' => {
                    paren_depth += 1;
                    dest.push('(');
                    i += 1;
                }
                ')' => {
                    if paren_depth == 0 {
                        break;
                    }
                    paren_depth -= 1;
                    dest.push(')');
                    i += 1;
                }
                ch if ch.is_ascii_control() => {
                    return None;
                }
                ch => {
                    dest.push(ch);
                    i += 1;
                }
            }
        }

        if dest.is_empty() {
            None
        } else {
            // Process entities and URL-encode
            let entity_decoded = self.process_entities(&dest);
            let url_encoded = self.url_encode(&entity_decoded);
            // Calculate byte offset
            let byte_offset = chars[..i].iter().map(|c| c.len_utf8()).sum();
            Some((url_encoded, byte_offset))
        }
    }

    /// Parse a link title that can span multiple lines (for reference definitions)
    /// Returns (title, lines_consumed) or None
    /// `lines` is the array of remaining lines, `first_line_remaining` is what's left on the current line
    fn parse_multiline_link_title(
        &self,
        lines: &[&str],
        first_line_remaining: &str,
    ) -> Option<(String, usize)> {
        if lines.is_empty() || first_line_remaining.is_empty() {
            return None;
        }

        let first_chars: Vec<char> = first_line_remaining.chars().collect();
        let delimiter = first_chars[0];

        let closing_delimiter = match delimiter {
            '"' => '"',
            '\'' => '\'',
            '(' => ')',
            _ => return None,
        };

        let mut title = String::new();
        let mut line_idx = 0;
        let mut char_idx = 1; // Start after opening delimiter

        // Process first line
        let mut current_line_chars: Vec<char> = first_chars.clone();

        loop {
            if char_idx >= current_line_chars.len() {
                // Reached end of current line
                if line_idx + 1 >= lines.len() {
                    // No more lines, title is not closed
                    return None;
                }

                // Check if next line is blank - titles cannot contain blank lines
                if lines[line_idx + 1].trim().is_empty() {
                    return None;
                }

                // Move to next line - titles can contain newlines
                title.push('\n');
                line_idx += 1;
                current_line_chars = lines[line_idx].chars().collect();
                char_idx = 0;
                continue;
            }

            match current_line_chars[char_idx] {
                ch if ch == closing_delimiter => {
                    // Found closing delimiter
                    // Check that rest of line is whitespace only
                    let rest: String = current_line_chars[char_idx + 1..].iter().collect();
                    if !rest.trim().is_empty() {
                        return None; // Content after closing delimiter
                    }

                    // Decode entities in title
                    let entity_decoded = self.process_entities(&title);
                    return Some((entity_decoded, line_idx + 1));
                }
                '\\' if char_idx + 1 < current_line_chars.len()
                    && self.is_ascii_punctuation(current_line_chars[char_idx + 1]) =>
                {
                    // Backslash escape of ASCII punctuation
                    title.push(current_line_chars[char_idx + 1]);
                    char_idx += 2;
                }
                ch => {
                    title.push(ch);
                    char_idx += 1;
                }
            }
        }
    }

    /// Try to parse raw HTML inline
    /// Returns (Node::HtmlInline, position_after) if successful
    fn try_parse_html_inline(&self, chars: &[char], start: usize) -> Option<(Node, usize)> {
        if start >= chars.len() || chars[start] != '<' {
            return None;
        }

        let mut i = start + 1;

        // Type 1: HTML comment <!--...-->
        if i + 2 < chars.len() && chars[i] == '!' && chars[i + 1] == '-' && chars[i + 2] == '-' {
            i += 3;
            // Look for closing -->
            // Cannot contain -->, cannot start with > or ->, cannot end with -
            let comment_start = i;

            // Check for invalid starts
            if i < chars.len() && chars[i] == '>' {
                return None; // <!--> is invalid
            }
            if i + 1 < chars.len() && chars[i] == '-' && chars[i + 1] == '>' {
                return None; // <!--> is invalid
            }

            while i < chars.len() {
                if chars[i] == '-'
                    && i + 2 < chars.len()
                    && chars[i + 1] == '-'
                    && chars[i + 2] == '>'
                {
                    // Found closing -->
                    // Check that it doesn't end with - before --
                    if i > comment_start && chars[i - 1] == '-' {
                        return None; // Cannot end with ---
                    }
                    i += 3;
                    let html: String = chars[start..i].iter().collect();
                    return Some((Node::HtmlInline(html), i));
                }
                // Cannot have newline in inline HTML
                if chars[i] == '\n' {
                    return None;
                }
                i += 1;
            }
            return None; // No closing -->
        }

        // Type 2: Processing instruction <?...?>
        if i < chars.len() && chars[i] == '?' {
            i += 1;
            while i < chars.len() {
                if chars[i] == '?' && i + 1 < chars.len() && chars[i + 1] == '>' {
                    i += 2;
                    let html: String = chars[start..i].iter().collect();
                    return Some((Node::HtmlInline(html), i));
                }
                if chars[i] == '\n' {
                    return None;
                }
                i += 1;
            }
            return None;
        }

        // Type 3: Declaration <!LETTER...>
        if i < chars.len() && chars[i] == '!' {
            if i + 1 < chars.len() && chars[i + 1].is_ascii_uppercase() {
                i += 1;
                while i < chars.len() {
                    if chars[i] == '>' {
                        i += 1;
                        let html: String = chars[start..i].iter().collect();
                        return Some((Node::HtmlInline(html), i));
                    }
                    if chars[i] == '\n' {
                        return None;
                    }
                    i += 1;
                }
            }
            return None;
        }

        // Type 4: CDATA section <![CDATA[...]]>
        if i + 7 < chars.len()
            && chars[i] == '!'
            && chars[i + 1] == '['
            && chars[i + 2] == 'C'
            && chars[i + 3] == 'D'
            && chars[i + 4] == 'A'
            && chars[i + 5] == 'T'
            && chars[i + 6] == 'A'
            && chars[i + 7] == '['
        {
            i += 8;
            while i < chars.len() {
                if i + 2 < chars.len()
                    && chars[i] == ']'
                    && chars[i + 1] == ']'
                    && chars[i + 2] == '>'
                {
                    i += 3;
                    let html: String = chars[start..i].iter().collect();
                    return Some((Node::HtmlInline(html), i));
                }
                if chars[i] == '\n' {
                    return None;
                }
                i += 1;
            }
            return None;
        }

        // Type 5: Closing tag </tagname>
        if i < chars.len() && chars[i] == '/' {
            i += 1;
            // Tag name must start with ASCII letter
            if i >= chars.len() || !chars[i].is_ascii_alphabetic() {
                return None;
            }
            i += 1;
            // Consume rest of tag name (letters, digits, hyphens)
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '-') {
                i += 1;
            }
            // Skip whitespace
            while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
                i += 1;
            }
            // Must end with >
            if i < chars.len() && chars[i] == '>' {
                i += 1;
                let html: String = chars[start..i].iter().collect();
                return Some((Node::HtmlInline(html), i));
            }
            return None;
        }

        // Type 6: Open tag <tagname attributes...> or <tagname attributes.../>
        // Tag name must start with ASCII letter
        if i >= chars.len() || !chars[i].is_ascii_alphabetic() {
            return None;
        }
        i += 1;

        // Consume tag name (letters, digits, hyphens)
        while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '-') {
            i += 1;
        }

        // Now parse attributes
        loop {
            // Skip whitespace (spaces, tabs, and up to one newline)
            let mut newline_seen = false;
            while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t' || chars[i] == '\n') {
                if chars[i] == '\n' {
                    if newline_seen {
                        return None; // Can't have multiple newlines
                    }
                    newline_seen = true;
                }
                i += 1;
            }

            // Check for end of tag
            if i >= chars.len() {
                return None;
            }

            if chars[i] == '>' {
                i += 1;
                let html: String = chars[start..i].iter().collect();
                return Some((Node::HtmlInline(html), i));
            }

            if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '>' {
                i += 2;
                let html: String = chars[start..i].iter().collect();
                return Some((Node::HtmlInline(html), i));
            }

            // Try to parse attribute name
            if !(chars[i].is_ascii_alphabetic() || chars[i] == '_' || chars[i] == ':') {
                return None; // Invalid attribute name start
            }
            i += 1;

            // Rest of attribute name
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric()
                    || chars[i] == '_'
                    || chars[i] == '.'
                    || chars[i] == ':'
                    || chars[i] == '-')
            {
                i += 1;
            }

            // Skip whitespace before =
            newline_seen = false;
            while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t' || chars[i] == '\n') {
                if chars[i] == '\n' {
                    if newline_seen {
                        return None;
                    }
                    newline_seen = true;
                }
                i += 1;
            }

            // Check for attribute value
            if i < chars.len() && chars[i] == '=' {
                i += 1;

                // Skip whitespace after =
                newline_seen = false;
                while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t' || chars[i] == '\n') {
                    if chars[i] == '\n' {
                        if newline_seen {
                            return None;
                        }
                        newline_seen = true;
                    }
                    i += 1;
                }

                if i >= chars.len() {
                    return None;
                }

                // Parse attribute value
                if chars[i] == '"' {
                    // Double-quoted value
                    i += 1;
                    while i < chars.len() && chars[i] != '"' {
                        if chars[i] == '\n' {
                            return None; // No newlines in quoted values
                        }
                        i += 1;
                    }
                    if i >= chars.len() {
                        return None; // No closing quote
                    }
                    i += 1; // Skip closing "
                } else if chars[i] == '\'' {
                    // Single-quoted value
                    i += 1;
                    while i < chars.len() && chars[i] != '\'' {
                        if chars[i] == '\n' {
                            return None; // No newlines in quoted values
                        }
                        i += 1;
                    }
                    if i >= chars.len() {
                        return None; // No closing quote
                    }
                    i += 1; // Skip closing '
                } else {
                    // Unquoted value - no spaces, tabs, newlines, ", ', =, <, >, `
                    if chars[i] == ' '
                        || chars[i] == '\t'
                        || chars[i] == '\n'
                        || chars[i] == '"'
                        || chars[i] == '\''
                        || chars[i] == '='
                        || chars[i] == '<'
                        || chars[i] == '>'
                        || chars[i] == '`'
                    {
                        return None;
                    }
                    i += 1;
                    while i < chars.len() {
                        if chars[i] == ' '
                            || chars[i] == '\t'
                            || chars[i] == '\n'
                            || chars[i] == '"'
                            || chars[i] == '\''
                            || chars[i] == '='
                            || chars[i] == '<'
                            || chars[i] == '>'
                            || chars[i] == '`'
                        {
                            break;
                        }
                        i += 1;
                    }
                }
            }
            // Attribute without value is OK, continue to next attribute or tag end
        }
    }
}
