export default grammar({
  name: "vinyl",

  extras: $ => [/\s/, $.comment],

  conflicts: $ => [[$._statement, $._expression]],

  rules: {
    source_file: $ => repeat($._definition),

    _definition: $ => seq(
      repeat($.attribute),
      choice(
        $.function_definition,
      ),
    ),

    comment: $ => token(choice(
      seq("#", /.*/),
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
      $.array_type,
    ),

    primitive_type: $ => choice(
      'int8', 'int16', 'int32', 'int64', 'int128', 'isize',
      'uint8', 'uint16', 'uint32', 'uint64', 'uint128', 'usize',
      'float32', 'float64',
      'bool', 'char', 'string', 'unit',
      'int', 'float'
    ),
    simple_type: $ => choice($.primitive_type, $.identifier),

    generic_type: $ => seq(
      $.identifier,
      "<",
      commaSep1($._type),
      ">",
    ),

    array_type: $ => seq(
      "[",
      $._type,
      ";",
      $.integer_literal,
      "]",
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
      optional($._expression),
      "}",
    ),

    _statement: $ => choice(
      $.let_declaration,
      $.expression_statement,
      $.return_statement,
      $.if_expression,
      $.while_statement,
      $.loop_statement,
      $.break_statement,
      $.continue_statement,
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
      $.raw_string_literal,
      $.char_literal,
      $.integer_literal,
      $.float_literal,
      $.bool_literal,
      $.unit_literal,
      $.call_expression,
      $.binary_expression,
      $.index_expression,
      $.array_expression,
      $.parenthesized_expression,
      $.block,
      $.if_expression,
    ),

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    char_literal: $ => seq(
      "'",
      choice(/[^'\\]/, /\\./),
      "'",
    ),

    string_literal: $ => seq(
      optional("f"),
      '"',
      repeat(/[^"\\]|\\"/),
      '"',
    ),

    raw_string_literal: $ => token(seq(
      "r",
      '"',
      repeat(/[^"]/),
      '"',
    )),

    attribute: $ => seq(
      "@",
      field("name", $.identifier),
      optional(seq(
        "(",
        commaSep($._expression),
        ")",
      )),
    ),

    integer_literal: $ => token(choice(
      seq(optional("-"), /[0-9]+/),
      seq(optional("-"), "0x", /[0-9a-fA-F]+/),
      seq(optional("-"), "0o", /[0-7]+/),
      seq(optional("-"), "0b", /[01]+/),
    )),

    float_literal: $ => token(seq(optional("-"), /[0-9]+/, ".", /[0-9]+/)),

    bool_literal: $ => choice("true", "false"),

    unit_literal: $ => "unit",

    call_expression: $ => seq(
      field("function", $.identifier),
      field("arguments", $.arguments),
    ),

    arguments: $ => seq(
      "(",
      commaSep($._expression),
      ")",
    ),

    binary_expression: $ => {
      const PREC = {
        POWER: 11,
        MULTIPLICATIVE: 10,
        ADDITIVE: 9,
        SHIFT: 8,
        BITAND: 7,
        BITXOR: 6,
        BITOR: 5,
        RANGE: 4,
        COMPARISON: 3,
        AND: 2,
        OR: 1,
      };
      return choice(
        prec.right(PREC.POWER, seq($._expression, field("operator", "**"), $._expression)),
        prec.left(PREC.MULTIPLICATIVE, seq($._expression, field("operator", choice("*", "/", "%", "//")), $._expression)),
        prec.left(PREC.ADDITIVE, seq($._expression, field("operator", choice("+", "-")), $._expression)),
        prec.left(PREC.SHIFT, seq($._expression, field("operator", choice("<<", ">>")), $._expression)),
        prec.left(PREC.BITAND, seq($._expression, field("operator", "&"), $._expression)),
        prec.left(PREC.BITXOR, seq($._expression, field("operator", "^"), $._expression)),
        prec.left(PREC.BITOR, seq($._expression, field("operator", "|"), $._expression)),
        prec.left(PREC.RANGE, seq($._expression, field("operator", choice("..", "..=")), $._expression)),
        prec.left(PREC.COMPARISON, seq($._expression, field("operator", choice("==", "!=", "<", ">", "<=", ">=")), $._expression)),
        prec.left(PREC.AND, seq($._expression, field("operator", choice("&&", "and")), $._expression)),
        prec.left(PREC.OR, seq($._expression, field("operator", choice("||", "or")), $._expression)),
      );
    },

    index_expression: $ => prec(12, seq(
      $._expression,
      "[",
      $._expression,
      "]",
    )),

    array_expression: $ => seq(
      "[",
      commaSep($._expression),
      "]",
    ),

    parenthesized_expression: $ => seq("(", $._expression, ")"),

    if_expression: $ => seq(
      "if",
      $._expression,
      $.block,
      repeat(seq("else", "if", $._expression, $.block)),
      optional(seq("else", $.block)),
    ),

    while_statement: $ => seq(
      "while",
      $._expression,
      $.block,
    ),

    loop_statement: $ => seq(
      "loop",
      $.block,
    ),

    break_statement: $ => seq("break", ";"),

    continue_statement: $ => seq("continue", ";"),
  },
});

function commaSep(rule) {
  return optional(commaSep1(rule));
}

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}
