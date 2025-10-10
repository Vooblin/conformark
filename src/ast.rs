/// AST node types for CommonMark documents
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Node {
    Document(Vec<Node>),
    // Block-level nodes
    Paragraph(Vec<Node>),
    Heading { level: u8, children: Vec<Node> },
    CodeBlock { info: String, literal: String },
    // Inline nodes
    Text(String),
    // More node types will be added incrementally
}
