/// AST node types for CommonMark documents
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Node {
    Document(Vec<Node>),
    // Block-level nodes
    Paragraph(Vec<Node>),
    Heading {
        level: u8,
        children: Vec<Node>,
    },
    CodeBlock {
        info: String,
        literal: String,
    },
    ThematicBreak,
    BlockQuote(Vec<Node>),
    // List nodes
    UnorderedList(Vec<Node>), // Contains ListItem nodes
    OrderedList {
        start: u32,
        children: Vec<Node>,
    }, // Contains ListItem nodes
    ListItem(Vec<Node>),      // Contains block-level content
    // Inline nodes
    Text(String),
    Code(String),        // Inline code span
    Emphasis(Vec<Node>), // <em> tag
    Strong(Vec<Node>),   // <strong> tag
    Link {
        destination: String,
        title: Option<String>,
        children: Vec<Node>,
    },
    HardBreak, // <br /> tag (backslash at end of line)
               // More node types will be added incrementally
}
