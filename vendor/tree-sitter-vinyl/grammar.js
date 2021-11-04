module.exports = grammar({
  name: 'vinyl',

  rules: {
    source_file: $ => repeat($._declaration),
	
	// Types
	
	_type: $ => choice(
		$.primitive_type,
		$.implicit_type,
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

    return_type: $ => choice($.primitive_type, 'void'),
	
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
	_declaration: $ => choice(
		$.variable_declaration
	),

	identifier: $ => token(seq(/[a-zA-Zα-ωΑ-Ωµ_][a-zA-Zα-ωΑ-Ωµ_0-9]*/)),
	
	variable_declaration: $ => seq(
		field('type', $._type),
		field('name', $.identifier),
		optional(seq(
			'=',
			field('expression', $._expression))),
        ';'
	),
	
	// Expressions
	_expression: $ => choice(
        $.array_creation_expression,
		$._literal
	),
	
    array_creation_expression: $ => seq(
        'new',
        field('type', $.array_type))
  }
});
