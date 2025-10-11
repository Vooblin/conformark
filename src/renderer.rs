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
        Node::UnorderedList(children) => {
            let content: String = children.iter().map(render_node).collect();
            format!("<ul>\n{}</ul>\n", content)
        }
        Node::OrderedList { start, children } => {
            let content: String = children.iter().map(render_node).collect();
            if *start == 1 {
                format!("<ol>\n{}</ol>\n", content)
            } else {
                format!("<ol start=\"{}\">\n{}</ol>\n", start, content)
            }
        }
        Node::ListItem(children) => {
            let content: String = children.iter().map(render_node).collect();
            // Check if content has block-level elements (contains newlines from nested blocks)
            if content.contains("</p>")
                || content.contains("</blockquote>")
                || content.contains("</pre>")
                || content.contains("</ul>")
                || content.contains("</ol>")
            {
                format!("<li>\n{}</li>\n", content)
            } else {
                // Simple content - no wrapping paragraph, trim trailing newline from Text
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
