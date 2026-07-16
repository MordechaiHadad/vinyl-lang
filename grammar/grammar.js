export default grammar({
  name: "vinyl",

  extras: $ => [/\s/, $.comment],

  conflicts: $ => [],

  rules: {
    source_file: $ => repeat($._definition),

    _definition: $ => choice(
      $.function_definition,
    ),

    comment: $ => token(choice(
      seq("//", /.*/),
      seq("/*", /[^*]*\*+([^/*][^*]*\*+)*/, "/"),
    )),

    function_definition: $ => seq(
      "fn",
      field("name", $.identifier),
      field("parameters", $.parameters),
      field("return_type", optional($.type_annotation)),
      field("body", $.block),
    ),

    type_annotation: $ => seq(":", $._type),

    _type: $ => choice(
      $.simple_type,
      $.generic_type,
    ),

    primitive_type: $ => choice(
      'int8', 'int16', 'int32', 'int64', 'int128',
      'uint8', 'uint16', 'uint32', 'uint64', 'uint128',
      'float32', 'float64',
      'bool', 'char', 'string',
    ),
    simple_type: $ => choice($.primitive_type, $.identifier),

    generic_type: $ => seq(
      $.identifier,
      "<",
      commaSep1($._type),
      ">",
    ),

    parameters: $ => seq(
      "(",
      commaSep($.parameter),
      ")",
    ),

    parameter: $ => seq(
      optional("mut"),
      field("name", $.identifier),
      field("type", $.type_annotation),
    ),

    block: $ => seq(
      "{",
      repeat($._statement),
      "}",
    ),

    _statement: $ => choice(
      $.let_declaration,
      $.expression_statement,
      $.return_statement,
      $.if_expression,
    ),

    let_declaration: $ => seq(
      "let",
      optional("mut"),
      field("name", $.identifier),
      optional($.type_annotation),
      "=",
      $._expression,
      ";",
    ),

    return_statement: $ => seq(
      "return",
      optional($._expression),
      ";",
    ),

    expression_statement: $ => seq(
      $._expression,
      ";",
    ),

    _expression: $ => choice(
      $.identifier,
      $.string_literal,
      $.integer_literal,
      $.float_literal,
      $.bool_literal,
      $.call_expression,
      $.binary_expression,
      $.parenthesized_expression,
      $.block,
    ),

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    string_literal: $ => seq(
      optional("f"),
      '"',
      repeat(/[^"\\]|\\"/),
      '"',
    ),

    integer_literal: $ => token(choice(
      seq(optional("-"), /[0-9]+/),
      seq(optional("-"), "0x", /[0-9a-fA-F]+/),
      seq(optional("-"), "0o", /[0-7]+/),
      seq(optional("-"), "0b", /[01]+/),
    )),

    float_literal: $ => token(seq(optional("-"), /[0-9]+/, ".", /[0-9]+/)),

    bool_literal: $ => choice("true", "false"),

    call_expression: $ => seq(
      field("function", $._expression),
      field("arguments", $.arguments),
    ),

    arguments: $ => seq(
      "(",
      commaSep($._expression),
      ")",
    ),

    binary_expression: $ => {
      const table = [
        [choice("||"), "||"],
        [choice("&&"), "&&"],
        [choice("==", "!=", "<", ">", "<=", ">="), "=="],
        [choice("+", "-"), "+"],
        [choice("*", "/", "%"), "*"],
      ];
      return choice(...table.map(([op, name]) =>
        prec.left(seq(
          $._expression,
          field("operator", op),
          $._expression,
        ))
      ));
    },

    parenthesized_expression: $ => seq("(", $._expression, ")"),

    if_expression: $ => seq(
      "if",
      $._expression,
      $.block,
      repeat(seq("else", "if", $._expression, $.block)),
      optional(seq("else", $.block)),
    ),
  },
});

function commaSep(rule) {
  return optional(commaSep1(rule));
}

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}
