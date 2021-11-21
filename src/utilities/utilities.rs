use tree_sitter::Node;

pub fn treesitter_to_enum(root: &Node) {
    let kind = match root.kind() {
        "(" => "LeftParen",
        ")" => "RightParen",
        "{" => "LeftCurly",
        "}" => "RigthCurly",
        ";" => "SemiColon",
        "=" => "EqualSign",
        "variable_declaration" => "VariableDecleration",
        "function_decleration" => "FunctionDecleration",
        "primitive_type" => "PrimitiveType",
        "identifier" => "Identifier",
        "literal" => "Literal",
        "integer_literal" => "integer_literal",
        "floating_point_literal" => "FloatingPointLiteral",
        "string_literal" => "StringLiteral",
        "char_literal" => "CharLiteral",
        "bool_literal" => "BoolLiteral",
        "binary_expression" => "BinaryExpression",
        "&&" => "And",
        "||" => "Or",
        "&" => "BitAnd",
        "|" => "BitOr",
        "^" => "BitXor",
        "==" => "Equal",
        "!=" => "NotEqual",
        "<" => "LessThan",
        "<=" =>  "LessThanOrEqual",
        ">" => "GreaterThan",
        ">=" => "GreaterThanOrEqual",
        "<<" => "ShiftLeft",
        ">>" => "ShiftRight",
        "+" => "PlusSign",
        "-" => "MinusSign",
        "*" => "Multiply",
        "/" => "Divide",
        "%" => "Modulus",
        "parameters" => "Parameters",
        "parameter" => "Parameter",
        "block" => "Block",
        _ => "",
    };
    let mut cursor = root.walk();

    println!("{} = {},", kind, root.kind_id());
    for child in root.children(&mut cursor) {
        treesitter_to_enum(&child);
    }
}