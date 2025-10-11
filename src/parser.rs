/// CommonMark parser implementation
use crate::ast::Node;

pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Parser
    }

    pub fn parse(&self, input: &str) -> Node {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = input.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Try to parse ATX heading first
            if let Some(heading) = self.parse_atx_heading(line) {
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
                    let children = self.parse_inline(lines[i].trim());
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
                    // Include blank lines
                    for _ in i..j {
                        code_lines.push(String::new());
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
        // Tabs advance to next multiple of 4 columns
        let mut col = 0;
        let mut chars = line.chars().peekable();
        let mut result = String::new();

        // Skip up to 4 columns of indentation
        while col < 4 {
            match chars.peek() {
                Some(&' ') => {
                    chars.next();
                    col += 1;
                }
                Some(&'\t') => {
                    chars.next();
                    let next_tab_stop = (col / 4 + 1) * 4;
                    if next_tab_stop <= 4 {
                        // Tab fits entirely within the 4 columns to remove
                        col = next_tab_stop;
                    } else {
                        // Partial tab: it extends beyond 4 columns
                        // Add spaces for the part that extends beyond
                        let spaces_to_add = next_tab_stop - 4;
                        for _ in 0..spaces_to_add {
                            result.push(' ');
                        }
                        col = 4;
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

        Some((fence_char, fence_len, indent))
    }

    /// Parse a fenced code block starting from the current position
    fn parse_fenced_code_block(
        &self,
        lines: &[&str],
        fence_char: char,
        fence_len: usize,
        _indent: usize,
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
            // Extract first word for language class and process backslash escapes
            let raw_info = info_string.split_whitespace().next().unwrap_or("");
            self.process_backslash_escapes(raw_info)
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

            // Add this line to the code block
            code_lines.push(line.to_string());
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

        // Extract heading text, trim leading/trailing whitespace and trailing #
        let text = after_hashes.trim();
        let text = text.trim_end_matches(['#', ' ', '\t']);

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

    /// Parse a blockquote starting from the current position
    fn parse_blockquote(&self, lines: &[&str]) -> (Node, usize) {
        let mut quote_lines = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Check if this line is part of the blockquote
            if self.is_blockquote_start(line) {
                // Strip the blockquote marker and add to quote lines
                let stripped = self.strip_blockquote_marker(line);
                quote_lines.push(stripped);
                i += 1;
            } else if !line.trim().is_empty() {
                // Check for lazy continuation
                // A non-empty line without > can continue if it's not a new block structure
                if self.can_lazy_continue(line) {
                    quote_lines.push(line.to_string());
                    i += 1;
                } else {
                    // This starts a new block, stop the blockquote
                    break;
                }
            } else {
                // Blank line - look ahead to see if blockquote continues
                let mut j = i + 1;
                while j < lines.len() && lines[j].trim().is_empty() {
                    j += 1;
                }

                if j < lines.len() && self.is_blockquote_start(lines[j]) {
                    // Blockquote continues after blank lines, include them
                    quote_lines.extend(std::iter::repeat_n(String::new(), j - i));
                    i = j;
                } else {
                    // Blockquote ends
                    break;
                }
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
        // Lines that start new block structures cannot lazy continue
        // Check for common block starters

        // 4+ spaces = code block
        if self.is_indented_code_line(line) {
            return false;
        }

        // Thematic break
        if self.is_thematic_break(line) {
            return false;
        }

        // ATX heading
        if self.parse_atx_heading(line).is_some() {
            return false;
        }

        // Fenced code block
        if self.is_fenced_code_start(line).is_some() {
            return false;
        }

        // Otherwise, can lazy continue
        true
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

        // Check if we have at least one more line
        if lines.len() < 2 {
            return None;
        }

        // Check if second line is a valid Setext underline
        self.is_setext_underline(lines[1])
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

        // Find first non-whitespace character
        let first_char = content.trim_start().chars().next()?;

        // Must be '=' or '-'
        let level = match first_char {
            '=' => 1,
            '-' => 2,
            _ => return None,
        };

        // All non-whitespace characters must be the same (= or -)
        for ch in content.chars() {
            if ch != ' ' && ch != '\t' && ch != first_char {
                return None;
            }
        }

        // Must have at least one underline character (not just whitespace)
        if content.trim().is_empty() {
            return None;
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

    /// Parse a list (collecting consecutive items with same marker type)
    fn parse_list(&self, lines: &[&str], list_type: ListType) -> (Node, usize) {
        let mut items = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            // Check if current line is a list item of the same type
            if let Some(current_type) = self.is_list_start(lines[i]) {
                if !list_type.is_compatible(&current_type) {
                    // Different list type, stop this list
                    break;
                }

                // Parse this list item (multi-line support)
                let (item, consumed) = self.parse_list_item(&lines[i..], &current_type);
                items.push(item);
                i += consumed;
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

        // Create the appropriate list node
        let list_node = match list_type {
            ListType::Unordered(_) => Node::UnorderedList(items),
            ListType::Ordered(start, _) => Node::OrderedList {
                start,
                children: items,
            },
        };

        (list_node, i)
    }

    /// Parse a single list item with multi-line support
    fn parse_list_item(&self, lines: &[&str], list_type: &ListType) -> (Node, usize) {
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

        while i < lines.len() {
            let line = lines[i];

            // Check if this is a new list item
            if self.is_list_start(line).is_some() {
                break;
            }

            // Blank line
            if line.trim().is_empty() {
                has_blank = true;
                item_lines.push(String::new());
                i += 1;
                continue;
            }

            // Check indentation to see if line belongs to this item
            let line_indent = self.count_indent_columns(line);

            if line_indent >= content_indent {
                // Remove the item indentation and add to item
                let dedented = self.remove_indent(line, content_indent);
                item_lines.push(dedented);
                i += 1;
            } else {
                // Not enough indentation, stop item
                break;
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

        // Determine if this is a "loose" list item (contains blank lines between blocks)
        // For now, if we have blank lines and multiple blocks, wrap in <p> tags
        let final_children = if has_blank && children.len() > 1 {
            // Wrap non-paragraph blocks in paragraphs if needed
            children
        } else if has_blank && children.len() == 1 {
            // Single block with blank lines - wrap in paragraph
            match &children[0] {
                Node::Paragraph(_) => children,
                _ => children, // Already in proper format
            }
        } else {
            children
        };

        let item = Node::ListItem(final_children);
        (item, i)
    }

    /// Calculate the required indent for list item continuation
    /// W (marker width) + N (spaces after marker, 1-4)
    fn calculate_list_item_indent(&self, line: &str, list_type: &ListType) -> usize {
        let trimmed = line.trim_start();
        let initial_indent = self.count_indent_columns(&line[..line.len() - trimmed.len()]);

        match list_type {
            ListType::Unordered(_) => {
                // Marker is 1 char, find spaces after
                let after_marker = &trimmed[1..];
                let spaces = after_marker.len() - after_marker.trim_start().len();
                let spaces = spaces.clamp(1, 4); // 1-4 spaces
                initial_indent + 1 + spaces
            }
            ListType::Ordered(_, delimiter) => {
                // Find delimiter position to get marker width
                if let Some(pos) = trimmed.find(*delimiter) {
                    let marker_width = pos + 1;
                    let after_marker = &trimmed[marker_width..];
                    let spaces = after_marker.len() - after_marker.trim_start().len();
                    let spaces = spaces.clamp(1, 4); // 1-4 spaces
                    initial_indent + marker_width + spaces
                } else {
                    initial_indent + 2 // Fallback
                }
            }
        }
    }

    /// Remove a specific amount of indentation from a line
    fn remove_indent(&self, line: &str, cols: usize) -> String {
        let mut removed = 0;
        let mut pos = 0;

        for ch in line.chars() {
            if removed >= cols {
                break;
            }

            match ch {
                ' ' => {
                    removed += 1;
                    pos += 1;
                }
                '\t' => {
                    let next_tab_stop = (removed / 4 + 1) * 4;
                    removed = next_tab_stop;
                    pos += 1;
                }
                _ => break,
            }
        }

        line[pos..].to_string()
    }

    /// Extract the content after a list marker
    fn extract_list_item_content(&self, line: &str, list_type: &ListType) -> String {
        let trimmed = line.trim_start();

        match list_type {
            ListType::Unordered(_) => {
                // Skip the marker character and following spaces
                let after_marker = &trimmed[1..].trim_start();
                after_marker.to_string()
            }
            ListType::Ordered(_, delimiter) => {
                // Find the delimiter position
                if let Some(pos) = trimmed.find(*delimiter) {
                    let after_marker = &trimmed[pos + 1..].trim_start();
                    after_marker.to_string()
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

            // Stop on indented code block (4+ spaces)
            if self.is_indented_code_line(line) {
                break;
            }

            // Stop on blockquote
            if self.is_blockquote_start(line) {
                break;
            }

            // Stop on list
            if self.is_list_start(line).is_some() {
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
        let text = paragraph_lines.join("\n");
        let children = self.parse_inline(&text);

        (Node::Paragraph(children), i)
    }

    /// Parse inline elements (code spans, emphasis, links, etc.) from text
    fn parse_inline(&self, text: &str) -> Vec<Node> {
        let mut nodes = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Handle backslash escapes first
            if chars[i] == '\\' && i + 1 < chars.len() {
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

            // Try to parse code span first (takes precedence)
            if chars[i] == '`'
                && let Some((code_node, new_i)) = self.try_parse_code_span(&chars, i)
            {
                nodes.push(code_node);
                i = new_i;
                continue;
            }

            // Try to parse autolink (before regular links)
            if chars[i] == '<'
                && let Some((autolink_node, new_i)) = self.try_parse_autolink(&chars, i)
            {
                nodes.push(autolink_node);
                i = new_i;
                continue;
            }

            // Try to parse image (before link, since images start with ![)
            if chars[i] == '!'
                && i + 1 < chars.len()
                && chars[i + 1] == '['
                && let Some((image_node, new_i)) = self.try_parse_image(&chars, i)
            {
                nodes.push(image_node);
                i = new_i;
                continue;
            }

            // Try to parse link
            if chars[i] == '['
                && let Some((link_node, new_i)) = self.try_parse_link(&chars, i)
            {
                nodes.push(link_node);
                i = new_i;
                continue;
            }

            // Try to parse emphasis/strong with * or _
            if (chars[i] == '*' || chars[i] == '_')
                && let Some((emph_node, new_i)) = self.try_parse_emphasis(&chars, i)
            {
                nodes.push(emph_node);
                i = new_i;
                continue;
            }

            // Collect regular text until next special character
            let text_start = i;
            while i < chars.len()
                && chars[i] != '\\'
                && chars[i] != '`'
                && chars[i] != '*'
                && chars[i] != '_'
                && chars[i] != '['
                && chars[i] != '!'
                && chars[i] != '<'
            {
                i += 1;
            }
            if i > text_start {
                nodes.push(Node::Text(chars[text_start..i].iter().collect()));
            }

            // If we didn't move forward, just consume one character as text
            if i == text_start {
                nodes.push(Node::Text(chars[i].to_string()));
                i += 1;
            }
        }

        // If no inline elements found, return single text node
        if nodes.is_empty() && !text.is_empty() {
            nodes.push(Node::Text(text.to_string()));
        }

        nodes
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

    fn try_parse_emphasis(&self, chars: &[char], start: usize) -> Option<(Node, usize)> {
        let delimiter = chars[start];
        let mut i = start;
        let mut count = 0;

        // Count delimiters
        while i < chars.len() && chars[i] == delimiter {
            count += 1;
            i += 1;
        }

        // Check if this is a left-flanking delimiter run
        if !self.is_left_flanking(chars, start, count) {
            return None;
        }

        // Try strong emphasis first (** or __)
        if count >= 2
            && let Some((content, end_pos)) = self.find_emphasis_close(chars, i, delimiter, 2)
        {
            let inner_text: String = chars[i..content].iter().collect();
            let inner_nodes = self.parse_inline(&inner_text);
            return Some((Node::Strong(inner_nodes), end_pos));
        }

        // Try regular emphasis (* or _)
        if count >= 1
            && let Some((content, end_pos)) = self.find_emphasis_close(chars, i, delimiter, 1)
        {
            let inner_text: String = chars[i..content].iter().collect();
            let inner_nodes = self.parse_inline(&inner_text);
            return Some((Node::Emphasis(inner_nodes), end_pos));
        }

        None
    }

    fn is_left_flanking(&self, chars: &[char], pos: usize, count: usize) -> bool {
        let after_pos = pos + count;

        // Must not be followed by whitespace
        if after_pos >= chars.len() {
            return false;
        }

        let after_char = chars[after_pos];
        if after_char.is_whitespace() {
            return false;
        }

        // For underscore, must not be preceded by alphanumeric (intraword restriction)
        if chars[pos] == '_' && pos > 0 {
            let before_char = chars[pos - 1];
            if before_char.is_alphanumeric() {
                return false;
            }
        }

        true
    }

    fn is_right_flanking(&self, chars: &[char], pos: usize, count: usize) -> bool {
        // Must not be preceded by whitespace
        if pos == 0 {
            return false;
        }

        let before_char = chars[pos - 1];
        if before_char.is_whitespace() {
            return false;
        }

        // For underscore, must not be followed by alphanumeric (intraword restriction)
        if pos + count < chars.len() && chars[pos] == '_' {
            let after_char = chars[pos + count];
            if after_char.is_alphanumeric() {
                return false;
            }
        }

        true
    }

    fn find_emphasis_close(
        &self,
        chars: &[char],
        start: usize,
        delimiter: char,
        needed_count: usize,
    ) -> Option<(usize, usize)> {
        let mut i = start;

        while i < chars.len() {
            if chars[i] == delimiter {
                let delim_start = i;
                let mut count = 0;

                while i < chars.len() && chars[i] == delimiter {
                    count += 1;
                    i += 1;
                }

                // Check if this is a valid right-flanking delimiter run
                if count >= needed_count && self.is_right_flanking(chars, delim_start, count) {
                    // Return content end position and position after delimiters
                    return Some((delim_start, delim_start + needed_count));
                }
            } else {
                i += 1;
            }
        }

        None
    }

    fn try_parse_link(&self, chars: &[char], start: usize) -> Option<(Node, usize)> {
        // Link syntax: [link text](destination "title")
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

        // Now we need '(' for inline link
        if i >= chars.len() || chars[i] != '(' {
            return None; // Not an inline link
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
            destination = self.process_backslash_escapes(&raw_dest);
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
            destination = self.process_backslash_escapes(&raw_dest);

            // Check for invalid characters in destination (spaces outside parens)
            if destination.contains(|c: char| c.is_whitespace()) && paren_depth == 0 {
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
            title = Some(self.process_backslash_escapes(&raw_title));
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

        // Parse the link text
        let link_text: String = chars[text_start..text_end].iter().collect();
        let children = self.parse_inline(&link_text);

        Some((
            Node::Link {
                destination,
                title,
                children,
            },
            i,
        ))
    }

    fn try_parse_image(&self, chars: &[char], start: usize) -> Option<(Node, usize)> {
        // Image syntax: ![alt text](destination "title")
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

        // Now we need '(' for inline image
        if i >= chars.len() || chars[i] != '(' {
            return None; // Not an inline image
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
            destination = self.process_backslash_escapes(&raw_dest);
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
            destination = self.process_backslash_escapes(&raw_dest);

            // Check for invalid characters in destination (spaces outside parens)
            if destination.contains(|c: char| c.is_whitespace()) && paren_depth == 0 {
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
            title = Some(self.process_backslash_escapes(&raw_title));
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

        // Parse the alt text (but flatten to plain text for alt attribute)
        let alt_text_str: String = chars[text_start..text_end].iter().collect();
        let alt_text = self.parse_inline(&alt_text_str);

        Some((
            Node::Image {
                destination,
                title,
                alt_text,
            },
            i,
        ))
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
}
