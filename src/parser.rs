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
                    let children = self.parse_inline(line);
                    blocks.push(Node::Paragraph(children));
                    i += 1;
                }
            }
            // Last line with no possibility of Setext underline
            else {
                let children = self.parse_inline(line);
                blocks.push(Node::Paragraph(children));
                i += 1;
            }
        }

        Node::Document(blocks)
    }

    fn is_indented_code_line(&self, line: &str) -> bool {
        // A line with 4+ spaces/tabs of indentation (tabs count as 4 spaces)
        let mut indent = 0;
        for ch in line.chars() {
            match ch {
                ' ' => indent += 1,
                '\t' => indent += 4,
                _ => break,
            }
        }
        indent >= 4 && !line.trim().is_empty()
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
        let mut remaining_indent = 4;
        let mut result = String::new();
        let mut chars = line.chars();

        // Remove up to 4 spaces of indentation
        while remaining_indent > 0 {
            match chars.next() {
                Some(' ') => remaining_indent -= 1,
                Some('\t') => {
                    // Tab counts as 4 spaces, but we only remove what we need
                    if remaining_indent >= 4 {
                        remaining_indent -= 4;
                    } else {
                        // Partial tab removal: add spaces for the remainder
                        for _ in 0..(4 - remaining_indent) {
                            result.push(' ');
                        }
                        remaining_indent = 0;
                    }
                }
                Some(ch) => {
                    result.push(ch);
                    break;
                }
                None => break,
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
            // Extract first word for language class
            info_string
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string()
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
            // Remove optional space after >
            if let Some(stripped) = after_marker.strip_prefix(' ') {
                stripped.to_string()
            } else {
                after_marker.to_string()
            }
        } else {
            line.to_string()
        }
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

                // Parse this list item (simple version: just the first line)
                let (item, consumed) = self.parse_list_item(&lines[i..], &current_type);
                items.push(item);
                i += consumed;
            } else if i > 0 && lines[i].trim().is_empty() {
                // Blank line - might continue or end the list
                // For now, end the list on blank line (simplified)
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

    /// Parse a single list item (simplified: just first line)
    fn parse_list_item(&self, lines: &[&str], list_type: &ListType) -> (Node, usize) {
        let line = lines[0];

        // Extract the content after the marker
        let content = self.extract_list_item_content(line, list_type);

        // Create a simple list item with text content
        let item = Node::ListItem(vec![Node::Text(content)]);

        (item, 1) // Consume 1 line for now
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
    /// Parse inline elements (code spans, emphasis, links, etc.) from text
    fn parse_inline(&self, text: &str) -> Vec<Node> {
        let mut nodes = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Try to parse code span
            if chars[i] == '`' {
                // Count opening backticks
                let mut backtick_count = 0;
                let start = i;
                while i < chars.len() && chars[i] == '`' {
                    backtick_count += 1;
                    i += 1;
                }

                // Look for matching closing backticks
                let mut found_close = false;
                let content_start = i;
                let mut j = i;

                while j < chars.len() {
                    if chars[j] == '`' {
                        // Count closing backticks
                        let close_start = j;
                        let mut close_count = 0;
                        while j < chars.len() && chars[j] == '`' {
                            close_count += 1;
                            j += 1;
                        }

                        if close_count == backtick_count {
                            // Found matching close
                            let content_end = close_start;
                            let mut content: String =
                                chars[content_start..content_end].iter().collect();

                            // Convert line endings to spaces
                            content = content.replace(['\n', '\r'], " ");

                            // Strip single leading and trailing space if content has both and isn't all spaces
                            if content.len() > 2
                                && content.starts_with(' ')
                                && content.ends_with(' ')
                                && !content.trim().is_empty()
                            {
                                content = content[1..content.len() - 1].to_string();
                            }

                            nodes.push(Node::Code(content));
                            i = j;
                            found_close = true;
                            break;
                        }
                    } else {
                        j += 1;
                    }
                }

                if !found_close {
                    // No matching close found, treat opening backticks as literal text
                    nodes.push(Node::Text(chars[start..i].iter().collect()));
                }
            } else {
                // Collect regular text until next special character
                let text_start = i;
                while i < chars.len() && chars[i] != '`' {
                    i += 1;
                }
                if i > text_start {
                    nodes.push(Node::Text(chars[text_start..i].iter().collect()));
                }
            }
        }

        // If no inline elements found, return single text node
        if nodes.is_empty() && !text.is_empty() {
            nodes.push(Node::Text(text.to_string()));
        }

        nodes
    }
}
