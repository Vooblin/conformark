/// HTML renderer for CommonMark AST
use crate::ast::Node;

pub struct HtmlRenderer;

impl HtmlRenderer {
    pub fn new() -> Self {
        HtmlRenderer
    }

    pub fn render(&self, node: &Node) -> String {
        render_node(node)
    }
}

impl Default for HtmlRenderer {
    fn default() -> Self {
        Self::new()
    }
}

fn render_node(node: &Node) -> String {
    match node {
        Node::Document(children) => children.iter().map(render_node).collect(),
        Node::Paragraph(children) => {
            let content: String = children.iter().map(render_node).collect();
            format!("<p>{}</p>\n", content)
        }
        Node::Heading { level, children } => {
            let content: String = children.iter().map(render_node).collect();
            format!("<h{}>{}</h{}>\n", level, content, level)
        }
        Node::CodeBlock { info, literal } => {
            if info.is_empty() {
                format!("<pre><code>{}</code></pre>\n", escape_html(literal))
            } else {
                format!(
                    "<pre><code class=\"language-{}\">{}</code></pre>\n",
                    escape_html(info),
                    escape_html(literal)
                )
            }
        }
        Node::ThematicBreak => "<hr />\n".to_string(),
        Node::BlockQuote(children) => {
            let content: String = children.iter().map(render_node).collect();
            format!("<blockquote>\n{}</blockquote>\n", content)
        }
        Node::UnorderedList { tight: _, children } => {
            let content: String = children.iter().map(render_node).collect();
            format!("<ul>\n{}</ul>\n", content)
        }
        Node::OrderedList {
            start,
            tight: _,
            children,
        } => {
            let content: String = children.iter().map(render_node).collect();
            if *start == 1 {
                format!("<ol>\n{}</ol>\n", content)
            } else {
                format!("<ol start=\"{}\">\n{}</ol>\n", start, content)
            }
        }
        Node::ListItem { tight, children } => {
            // Determine if this item should render its paragraphs with <p> tags
            // If tight is true, single paragraphs are unwrapped

            // Check if we have a mix of inline and block content
            let has_blocks = children.iter().any(|child| {
                matches!(
                    child,
                    Node::Paragraph(_)
                        | Node::BlockQuote(_)
                        | Node::CodeBlock { .. }
                        | Node::UnorderedList { .. }
                        | Node::OrderedList { .. }
                        | Node::ThematicBreak
                        | Node::HtmlBlock(_)
                )
            });

            if *tight && children.len() == 1 {
                // Tight item with single child - unwrap paragraph if it's the only content
                match &children[0] {
                    Node::Paragraph(para_children) => {
                        let content: String = para_children.iter().map(render_node).collect();
                        return format!("<li>{}</li>\n", content.trim_end());
                    }
                    _ => {
                        // Single non-paragraph block
                        let content = render_node(&children[0]);
                        if content.ends_with('\n') {
                            return format!("<li>\n{}</li>\n", content);
                        } else {
                            return format!("<li>{}</li>\n", content);
                        }
                    }
                }
            }

            if has_blocks {
                // Render inline elements first (if any) on the same line as <li>
                let mut inline_content = String::new();
                let mut block_content = String::new();

                for child in children {
                    match child {
                        Node::Text(_)
                        | Node::Code(_)
                        | Node::Emphasis(_)
                        | Node::Strong(_)
                        | Node::Link { .. }
                        | Node::Image { .. }
                        | Node::HtmlInline(_)
                        | Node::HardBreak => {
                            inline_content.push_str(&render_node(child));
                        }
                        Node::Paragraph(para_children) if *tight => {
                            // In a tight list item, unwrap first paragraph to inline
                            let para_content: String =
                                para_children.iter().map(render_node).collect();
                            // First paragraph goes on same line as <li>
                            if inline_content.is_empty() && block_content.is_empty() {
                                inline_content.push_str(&para_content);
                            } else {
                                // Subsequent paragraphs in tight items also unwrapped but as block-level
                                // Don't add extra newline - content already has it or gets trimmed later
                                block_content.push_str(&para_content);
                            }
                        }
                        _ => {
                            block_content.push_str(&render_node(child));
                        }
                    }
                }

                if !inline_content.is_empty() && !block_content.is_empty() {
                    // Mix of inline and block: inline on same line, blocks indented
                    format!(
                        "<li>{}\n{}</li>\n",
                        inline_content.trim_end(),
                        block_content
                    )
                } else if !block_content.is_empty() {
                    // Only blocks: newline after <li>
                    format!("<li>\n{}</li>\n", block_content)
                } else {
                    // Only inline (shouldn't happen if has_blocks is true, but handle it)
                    format!("<li>{}</li>\n", inline_content.trim_end())
                }
            } else {
                // Simple inline content only
                let content: String = children.iter().map(render_node).collect();
                let trimmed = content.trim_end_matches('\n');
                format!("<li>{}</li>\n", trimmed)
            }
        }
        Node::Text(text) => escape_html(text),
        Node::Code(code) => format!("<code>{}</code>", escape_html(code)),
        Node::Emphasis(children) => {
            let content: String = children.iter().map(render_node).collect();
            format!("<em>{}</em>", content)
        }
        Node::Strong(children) => {
            let content: String = children.iter().map(render_node).collect();
            format!("<strong>{}</strong>", content)
        }
        Node::Link {
            destination,
            title,
            children,
        } => {
            let content: String = children.iter().map(render_node).collect();
            if let Some(title_text) = title {
                format!(
                    "<a href=\"{}\" title=\"{}\">{}</a>",
                    escape_html(destination),
                    escape_html(title_text),
                    content
                )
            } else {
                format!("<a href=\"{}\">{}</a>", escape_html(destination), content)
            }
        }
        Node::Image {
            destination,
            title,
            alt_text,
        } => {
            // Convert alt_text nodes to plain text (strip formatting)
            let alt = alt_text_to_string(alt_text);
            if let Some(title_text) = title {
                format!(
                    "<img src=\"{}\" alt=\"{}\" title=\"{}\" />",
                    escape_html(destination),
                    escape_html(&alt),
                    escape_html(title_text)
                )
            } else {
                format!(
                    "<img src=\"{}\" alt=\"{}\" />",
                    escape_html(destination),
                    escape_html(&alt)
                )
            }
        }
        Node::HardBreak => "<br />\n".to_string(),
        Node::HtmlBlock(content) => content.clone(), // Pass through raw HTML unchanged
        Node::HtmlInline(content) => content.clone(), // Pass through raw HTML unchanged
    }
}

fn escape_html(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '&' => "&amp;".to_string(),
            '"' => "&quot;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

/// Convert inline nodes to plain text (for image alt text)
/// This strips all formatting and just keeps the text content
fn alt_text_to_string(nodes: &[Node]) -> String {
    nodes
        .iter()
        .map(|node| match node {
            Node::Text(text) => text.clone(),
            Node::Code(code) => code.clone(),
            Node::Emphasis(children) | Node::Strong(children) => alt_text_to_string(children),
            Node::Link { children, .. } => alt_text_to_string(children),
            Node::Image { alt_text, .. } => alt_text_to_string(alt_text),
            Node::HardBreak => "\n".to_string(),
            _ => String::new(),
        })
        .collect()
}
