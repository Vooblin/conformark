/// CommonMark parser implementation
use crate::ast::Node;

pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Parser
    }

    pub fn parse(&self, input: &str) -> Node {
        let mut blocks = Vec::new();

        for line in input.lines() {
            // Try to parse ATX heading
            if let Some(heading) = self.parse_atx_heading(line) {
                blocks.push(heading);
            } else if !line.trim().is_empty() {
                // Non-empty, non-heading lines become paragraphs
                blocks.push(Node::Paragraph(vec![Node::Text(line.to_string())]));
            }
        }

        Node::Document(blocks)
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
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}
