use conformark::markdown_to_html;

fn main() {
    let input = "<a foo=\"bar\" bam = 'baz <em>\"</em>'\n_boolean zoop:33=zoop:33 />\n";

    println!("Input:");
    println!("{:?}", input);
    println!("\n{}", input);

    let result = markdown_to_html(input);

    println!("\nActual output:");
    println!("{:?}", result);
    println!("\n{}", result);

    let expected = "<p><a foo=\"bar\" bam = 'baz <em>\"</em>'\n_boolean zoop:33=zoop:33 /></p>\n";

    println!("\nExpected output:");
    println!("{:?}", expected);
    println!("\n{}", expected);

    println!("\nMatch: {}", result == expected);
}
