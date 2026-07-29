const PREC = {
  PIPE: 0,
  CALL: 14,
  UNARY: 12,
  INDEX: 12,
  FIELD: 15,
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

export default grammar({
  name: "vinyl",

  extras: $ => [/\s/, $.comment],

  conflicts: $ => [
    [$._statement, $._expression],
    [$.unary_expression, $.index_expression],
    [$.scoped_type_expression],
    [$._expression, $.struct_literal_expression],
  ],

  rules: {
    source_file: $ => repeat($._definition),

    _definition: $ => seq(
      optional("public"),
      repeat($.attribute),
      choice(
        $.function_definition,
        $.struct_definition,
        $.tuple_definition,
        $.enum_definition,
        $.import_statement,
      ),
    ),

    comment: $ => token(choice(
      seq("#", /.*/),
      seq("/*", /[^*]*\*+([^/*][^*]*\*+)*/, "/"),
    )),

    function_definition: $ => seq(
      "fn",
      field("name", $.value_identifier),
      field("parameters", $.parameters),
      field("return_type", optional($.type_annotation)),
      field("body", $.block),
    ),

    struct_definition: $ => seq(
      "struct",
      field("name", $.type_identifier),
      "{",
      commaSep($.field_definition),
      optional(","),
      "}",
    ),

    field_definition: $ => seq(
      field("name", $.value_identifier),
      ":",
      $._type,
    ),

    tuple_definition: $ => seq(
      "tuple",
      field("name", $.type_identifier),
      "(",
      commaSep($._type),
      ")",
    ),

    enum_definition: $ => seq(
      "enum",
      field("name", $.type_identifier),
      "{",
      commaSep($.enum_variant),
      optional(","),
      "}",
    ),

    enum_variant: $ => seq(
      field("name", $.type_identifier),
      optional(choice(
        seq("(", commaSep($._type), ")"),
        seq("{", commaSep($.field_definition), optional(","), "}"),
      )),
    ),

    type_annotation: $ => seq(":", $._type),

    _type: $ => choice(
      $.simple_type,
      $.generic_type,
      $.array_type,
      $.reference_type,
      $.tuple_type,
    ),

    tuple_type: $ => seq(
      "(",
      commaSep($._type),
      ")",
    ),

    reference_type: $ => seq("&", $._type),

    primitive_type: $ => choice(
      'int8', 'int16', 'int32', 'int64', 'int128', 'isize',
      'uint8', 'uint16', 'uint32', 'uint64', 'uint128', 'usize',
      'float32', 'float64',
      'bool', 'char', 'string', 'unit',
      'int', 'float'
    ),
    simple_type: $ => choice($.primitive_type, $.type_identifier),

    generic_type: $ => seq(
      $.type_identifier,
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
      optional(field("mut", "mut")),
      field("name", $.value_identifier),
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
      $.assignment_statement,
      $.expression_statement,
      $.return_statement,
      $.if_expression,
      $.while_statement,
      $.loop_statement,
      $.break_statement,
      $.continue_statement,
      $.import_statement,
    ),

    let_declaration: $ => seq(
      "let",
      optional(field("mut", "mut")),
      field("name", $.value_identifier),
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

    assignment_statement: $ => seq(
      $._expression,
      field("operator", choice("=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=")),
      $._expression,
      ";",
    ),

    expression_statement: $ => seq(
      $._expression,
      ";",
    ),

    _expression: $ => choice(
      $.value_identifier,
      $.string_literal,
      $.raw_string_literal,
      $.char_literal,
      $.integer_literal,
      $.float_literal,
      $.bool_literal,
      $.unit_literal,
      $.call_expression,
      $.binary_expression,
      $.unary_expression,
      $.index_expression,
      $.field_access_expression,
      $.array_expression,
      $.tuple_expression,
      $.match_expression,
      $.pipe_expression,
      $.parenthesized_expression,
      $.block,
      $.if_expression,
      $.scoped_value_expression,
      $.scoped_type_expression,
      $.struct_literal_expression,
    ),

    value_identifier: $ => /[a-z_][a-zA-Z0-9_]*/,

    type_identifier: $ => /[A-Z][a-zA-Z0-9_]*/,

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

    import_statement: $ => seq(
      "import",
      field("path", $.import_path),
      ";",
    ),

    import_prefix: $ => choice("self", "package", "parent"),

    import_name: $ => sep1(choice($.value_identifier, $.type_identifier), "::"),

    import_path: $ => seq(
      repeat(seq(field("prefix", $.import_prefix), "::")),
      field("path", $.import_name),
    ),

    attribute: $ => seq(
      "@",
      field("name", $.value_identifier),
      optional(seq(
        "(",
        commaSep($._expression),
        ")",
      )),
    ),

    integer_literal: $ => token(choice(
      /[0-9]+/,
      seq("0x", /[0-9a-fA-F]+/),
      seq("0o", /[0-7]+/),
      seq("0b", /[01]+/),
    )),

    float_literal: $ => token(seq(/[0-9]+/, ".", /[0-9]+/)),

    bool_literal: $ => choice("true", "false"),

    unit_literal: $ => "unit",

    call_expression: $ => prec(PREC.CALL, seq(
      field("function", choice($.value_identifier, $.scoped_value_expression)),
      field("arguments", $.arguments),
    )),

    arguments: $ => seq(
      "(",
      commaSep($._expression),
      ")",
    ),

    unary_expression: $ => prec(PREC.UNARY, seq(
      field("operator", choice("-", "!", "not", "&")),
      $._expression
    )),

    binary_expression: $ => choice(
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
    ),

    index_expression: $ => prec(PREC.INDEX, seq(
      $._expression,
      "[",
      $._expression,
      "]",
    )),

    field_access_expression: $ => prec(PREC.FIELD, seq(
      $._expression,
      ".",
      field("field", choice($.value_identifier, $.integer_literal)),
    )),

    array_expression: $ => seq(
      "[",
      commaSep($._expression),
      "]",
    ),

    tuple_expression: $ => seq(
      "(",
      $._expression,
      ",",
      optional(seq(
        $._expression,
        repeat(seq(",", $._expression)),
      )),
      optional(","),
      ")",
    ),

    match_expression: $ => seq(
      "match",
      $._expression,
      "{",
      repeat($.match_arm),
      "}",
    ),

    struct_literal_expression: $ => seq(
      field("name", $.type_identifier),
      field("fields", $.struct_literal_fields),
    ),

    struct_literal_fields: $ => seq(
      "{",
      commaSep(seq(field("name", $.value_identifier), ":", field("value", $._expression))),
      "}",
    ),

    scoped_value_expression: $ => prec(PREC.FIELD, seq(
      field("module", choice($.value_identifier, $.type_identifier)),
      "::",
      field("function", $.value_identifier),
    )),

    scoped_type_expression: $ => prec(PREC.FIELD, seq(
      field("type", choice($.value_identifier, $.type_identifier)),
      "::",
      field("variant", $.type_identifier),
      optional(field("arguments", $.arguments)),
    )),

    match_arm: $ => seq(
      $.pattern,
      "=>",
      $._expression,
      optional(","),
    ),

    pattern: $ => choice(
      $.wildcard_pattern,
      $.identifier_pattern,
      $.literal_pattern,
      $.struct_pattern,
      $.tuple_pattern,
      $.enum_variant_pattern,
    ),

    wildcard_pattern: $ => "_",

    identifier_pattern: $ => $.value_identifier,

    literal_pattern: $ => choice(
      $.integer_literal,
      $.bool_literal,
      $.char_literal,
      $.string_literal,
    ),

    struct_pattern: $ => seq(
      $.type_identifier,
      "{",
      commaSep($.field_pattern),
      "}",
    ),

    field_pattern: $ => seq(
      field("name", $.value_identifier),
      optional(seq(":", $.pattern)),
    ),

    tuple_pattern: $ => seq(
      "(",
      commaSep1($.pattern),
      ")",
    ),

    enum_variant_pattern: $ => seq(
      $.type_identifier,
      "(",
      commaSep1($.pattern),
      ")",
    ),

    pipe_expression: $ => prec.left(PREC.PIPE, seq(
      field("left", $._expression),
      field("operator", choice("|>", "|>>")),
      field("right", $._expression),
    )),

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

function sep1(rule, separator) {
  return seq(rule, repeat(seq(separator, rule)));
}
