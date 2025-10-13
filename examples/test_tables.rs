use conformark::markdown_to_html;

fn main() {
    println!("Testing GFM Table Support\n");

    // Test 1: Basic table
    let test1 = "| Header 1 | Header 2 |\n| -------- | -------- |\n| Cell 1   | Cell 2   |";
    println!("Test 1: Basic table");
    println!("Input:\n{}\n", test1);
    println!("Output:\n{}\n", markdown_to_html(test1));

    // Test 2: Table with alignment
    let test2 = "| Left | Center | Right |\n|:-----|:------:|------:|\n| L1   | C1     | R1    |\n| L2   | C2     | R2    |";
    println!("Test 2: Table with alignment");
    println!("Input:\n{}\n", test2);
    println!("Output:\n{}\n", markdown_to_html(test2));

    // Test 3: Table with inline formatting
    let test3 = "| Name | Description |\n| ---- | ----------- |\n| **Bold** | *Italic* |\n| `code` | [link](http://example.com) |";
    println!("Test 3: Table with inline formatting");
    println!("Input:\n{}\n", test3);
    println!("Output:\n{}\n", markdown_to_html(test3));

    // Test 4: Table with escaped pipes
    let test4 = "| Column 1 | Column 2 |\n| -------- | -------- |\n| A \\| B   | C \\| D   |";
    println!("Test 4: Table with escaped pipes");
    println!("Input:\n{}\n", test4);
    println!("Output:\n{}\n", markdown_to_html(test4));

    // Test 5: Minimal table
    let test5 = "| H |\n|---|\n| C |";
    println!("Test 5: Minimal table");
    println!("Input:\n{}\n", test5);
    println!("Output:\n{}\n", markdown_to_html(test5));
}
