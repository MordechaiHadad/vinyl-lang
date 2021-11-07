module.exports = grammar({
  name: 'vinyl',

  rules: {
    source_file: $ => repeat($._statement),
	
    _statement: $ => choice(
        $._declaration,
        $._expression),

    _declaration: $ => choice(
		$.variable_declaration,
        $.function_declaration,
	),

	variable_declaration: $ => seq(
		field('type', choice($._type, $.implicit_type)),
		field('name', $.identifier),
		optional(seq(
			'=',
			field('expression', $._expression))),
        ';'
	),

    function_declaration: $ => seq(
        field('return_type', choice($._type, $.void_type)),
        field('identifier', $.identifier),
        field('parameters', $.parameters),
    ),

	// Types
	
	_type: $ => choice(
		$.primitive_type,
        $.array_type
	),


	primitive_type: $ => token(choice(
		'bool',
		'char',
		'string',
		'int8',
		'int16',
		'int32',
		'int64',
		'int128',
		'uint8',
		'uint16',
		'uint32',
		'uint64',
		'uint128',
        'float32',
        'float64',
        'float128'
	)),
	
	implicit_type: $ => 'var',

    void_type: $ => 'void',
	
	array_type: $ => seq(
		field('type', $._type),
        '[',
		optional(field('size', $.integer_literal)),
        ']'
    ),

	// Literals
	
	_literal: $ => choice(
		$.integer_literal,
		$.string_literal,
		$.char_literal,
		$.bool_literal,
        $.floating_point_literal,
	),
	
	integer_literal: $ => token(seq(/[0-9][0-9_]*/)),

    floating_point_literal: $ => token(seq(/[0-9][0-9_]*\.[0-9]*/)),

	string_literal: $ => seq(
		'"',
		repeat(/[^"\\\n]+/),
		'"'
	),
	
	char_literal: $ => seq(
		"'",
		token.immediate(/[^'\\]/),
		"'"
	),
	
	bool_literal: $ => choice(
		'true',
		'false'
	),
	
	// Declerations

	identifier: $ => token(seq(/[a-zA-Zα-ωΑ-Ωµ_][a-zA-Zα-ωΑ-Ωµ_0-9]*/)),
	
	
	// Expressions
	_expression: $ => choice(
        $.array_creation_expression,
		$._literal
	),
	
    array_creation_expression: $ => seq(
        'new',
        field('type', $.array_type)),

    parameters: $ => seq(
        '(',
        ')',
    ),

    block: $ => seq(
        '{',
        repeat($._statement),
        '}',
    ),
  }
});
