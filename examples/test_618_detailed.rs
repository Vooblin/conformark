use conformark::markdown_to_html;

fn main() {
    // Start with simpler cases and build up
    let tests = vec![
        ("<a>", "<p><a></p>\n", "simple tag"),
        (
            "<a foo=\"bar\">",
            "<p><a foo=\"bar\"></p>\n",
            "tag with simple attribute",
        ),
        (
            "<a foo=\"bar\" bam=\"baz\">",
            "<p><a foo=\"bar\" bam=\"baz\"></p>\n",
            "tag with two attributes",
        ),
        (
            "<a foo=\"bar\" bam='baz'>",
            "<p><a foo=\"bar\" bam='baz'></p>\n",
            "tag with single-quoted attribute",
        ),
        (
            "<a\nfoo=\"bar\">",
            "<p><a\nfoo=\"bar\"></p>\n",
            "tag with newline before attribute",
        ),
        (
            "<a foo=\"bar\"\nbam=\"baz\">",
            "<p><a foo=\"bar\"\nbam=\"baz\"></p>\n",
            "tag with newline between attributes",
        ),
        (
            "<a _boolean>",
            "<p><a _boolean></p>\n",
            "attribute starting with underscore",
        ),
        (
            "<a zoop:33=\"test\">",
            "<p><a zoop:33=\"test\"></p>\n",
            "attribute with colon",
        ),
        (
            "<a bam = 'baz <em>\"</em>'>",
            "<p><a bam = 'baz <em>\"</em>'></p>\n",
            "embedded tag in attribute",
        ),
        (
            "<a foo=\"bar\" bam = 'baz <em>\"</em>'\n_boolean zoop:33=zoop:33 />",
            "<p><a foo=\"bar\" bam = 'baz <em>\"</em>'\n_boolean zoop:33=zoop:33 /></p>\n",
            "full test 618",
        ),
    ];

    for (input, expected, description) in tests {
        let result = markdown_to_html(input);
        let matches = result == expected;

        if matches {
            println!("✅ {}", description);
        } else {
            println!("❌ {}", description);
            println!("  Input:    {:?}", input);
            println!("  Expected: {:?}", expected);
            println!("  Got:      {:?}", result);
        }
    }
}
