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
    UnorderedList {
        tight: bool,         // Tight lists don't add <p> tags in simple items
        children: Vec<Node>, // Contains ListItem nodes
    },
    OrderedList {
        start: u32,
        tight: bool,
        children: Vec<Node>,
    }, // Contains ListItem nodes
    ListItem {
        tight: bool, // Whether this item should render tightly (no <p> for simple content)
        children: Vec<Node>, // Contains block-level content
    },
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
    Image {
        destination: String,
        title: Option<String>,
        alt_text: Vec<Node>, // Alt text can contain inline elements
    },
    HardBreak,          // <br /> tag (backslash at end of line)
    HtmlBlock(String),  // Raw HTML block (passed through unchanged)
    HtmlInline(String), // Raw HTML inline (passed through unchanged)
    // GFM extension nodes
    Table {
        alignments: Vec<Alignment>, // Column alignments
        children: Vec<Node>,        // Contains TableRow nodes
    },
    TableRow(Vec<Node>), // Contains TableCell nodes
    TableCell {
        is_header: bool,
        children: Vec<Node>, // Inline content
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Alignment {
    None,
    Left,
    Right,
    Center,
}
