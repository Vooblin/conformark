/// A CommonMark-compliant Markdown parser and renderer
pub mod ast;
pub mod parser;
pub mod renderer;

use parser::Parser;
use renderer::HtmlRenderer;

/// Parse markdown text and render to HTML
pub fn markdown_to_html(markdown: &str) -> String {
    let parser = Parser::new();
    let ast = parser.parse(markdown);
    let renderer = HtmlRenderer::new();
    renderer.render(&ast)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        assert_eq!(markdown_to_html(""), "");
    }

    #[test]
    fn test_basic_image() {
        let result = markdown_to_html("![foo](/url \"title\")\n");
        assert_eq!(
            result,
            "<p><img src=\"/url\" alt=\"foo\" title=\"title\" /></p>\n"
        );
    }

    #[test]
    fn test_image_without_title() {
        let result = markdown_to_html("![bar](/path)\n");
        assert_eq!(result, "<p><img src=\"/path\" alt=\"bar\" /></p>\n");
    }
}
