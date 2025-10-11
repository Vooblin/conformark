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
            // Non-empty, non-special lines become paragraphs
            else {
                blocks.push(Node::Paragraph(vec![Node::Text(line.to_string())]));
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

        Some(Node::Heading {
            level: hash_count as u8,
            children: vec![Node::Text(text.to_string())],
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
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}
