use conformark::markdown_to_html;

fn main() {
    let input = "<del>*foo*</del>\n";
    let output = markdown_to_html(input);
    let expected = "<p><del><em>foo</em></del></p>\n";

    println!("Input: {:?}", input);
    println!("Output: {:?}", output);
    println!("Expected: {:?}", expected);
    println!("Match: {}", output == expected);
}
