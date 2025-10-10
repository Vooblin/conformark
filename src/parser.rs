/// CommonMark parser implementation
use crate::ast::Node;

pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Parser
    }

    pub fn parse(&self, _input: &str) -> Node {
        // Stub implementation - returns empty document
        Node::Document(vec![])
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}
