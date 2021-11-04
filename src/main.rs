use tree_sitter::{Parser, Language, Node};


extern "C" {fn tree_sitter_vinyl() -> Language;}

fn main() {

    let language = unsafe {tree_sitter_vinyl()};
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();

    let source_code = std::fs::read_to_string("vendor/tree-sitter-vinyl/test.vnl").unwrap();
    let tree = parser.parse(&source_code, None).unwrap();
    let root = tree.root_node();

    loop_root(&root, 0, &source_code);

    println!("\n");
    println!("source code:");
    println!("{}", &source_code);
}

// NOTE: Loop through parsed smybols.
fn loop_root(node: &Node, jump_count: usize, source: &String) {

    let mut cursor = node.walk();

    if node.child_count() > 0 {
        for child in node.children(&mut cursor) {
            
            if child.child_count() > 0 {
                println!("{:width$}{}", "", child.kind(), width = jump_count);
                loop_root(&child, jump_count + 1, source);
            }
            else {
                let value = get_node_value(&source, child.start_byte(), child.end_byte());
                if value != "" {
                    println!("{:width$}{}: {}", "", child.kind(), value, width = jump_count);
                } else {
                    println!("{:width$}{}", "", child.kind(), width = jump_count);
                }
            }
        }
    }
}

// NOTE: Gets node's value
fn get_node_value(source: &String, start: usize, end: usize) -> String {
    let value = source[start..end].to_string();
    match value.as_str() {
        "=" => return String::from(""),
        ";" => return String::from(""),
        "\"" => return String::from(""),
        "true" => return String::from(""),
        "false" => return String::from(""),
        _ => return value
    };
}
