;; --- Fallbacks / Baselines (Must be at the top) ---
(value_identifier) @variable
(type_identifier) @type

;; --- Comments ---
(comment) @comment

;; --- Keywords & Modifiers ---
[
  "fn"
  "struct"
  "tuple"
  "enum"
  "let"
  "return"
  "if"
  "else"
  "while"
  "loop"
  "break"
  "continue"
  "match"
] @keyword

[
  "public"
  "mut"
] @keyword.modifier

"import" @keyword.import

(unary_expression operator: "not" @keyword.operator)
(binary_expression operator: ["and" "or"] @keyword.operator)

;; --- Imports & Modules ---
(import_path [
  (value_identifier)
  (type_identifier)
] @module)

(scoped_value_expression
  module: [
    (value_identifier)
    (type_identifier)
  ] @module)

;; --- Functions ---
(function_definition
  name: (value_identifier) @function (#set! "priority" 105))

(call_expression
  function: (value_identifier) @function.call (#set! "priority" 105))

(call_expression
  function: (scoped_value_expression
    function: (value_identifier) @function.call) (#set! "priority" 105))

(scoped_value_expression
  function: (value_identifier) @function)

;; --- Types & Definitions ---
(primitive_type) @type.builtin
(simple_type (type_identifier) @type)
(generic_type (type_identifier) @type)
(struct_definition name: (type_identifier) @type)
(tuple_definition name: (type_identifier) @type)
(enum_definition name: (type_identifier) @type)
(struct_literal_expression name: (type_identifier) @type)
(struct_pattern (type_identifier) @type)

;; --- Enum Variants & Scoped Types ---
(enum_variant name: (type_identifier) @constructor)
(enum_variant_pattern (type_identifier) @constructor)

(scoped_type_expression
  type: [
    (value_identifier)
    (type_identifier)
  ] @type
  variant: (type_identifier) @constructor)

;; --- Fields & Properties ---
(field_definition name: (value_identifier) @property)
(field_access_expression field: (value_identifier) @property)
(field_access_expression field: (integer_literal) @property)
(struct_literal_fields name: (value_identifier) @property)
(field_pattern name: (value_identifier) @property)

;; --- Variables, Parameters & Patterns ---
(parameter name: (value_identifier) @variable.parameter)
(let_declaration name: (value_identifier) @variable)
(wildcard_pattern) @variable.builtin

;; --- Literals ---
(integer_literal) @number
(float_literal) @number.float
(bool_literal) @boolean
(char_literal) @character
(string_literal) @string
(raw_string_literal) @string
(unit_literal) @constant.builtin

;; --- Operators ---
(unary_expression operator: _ @operator)
(binary_expression operator: _ @operator)
(pipe_expression operator: _ @operator)
(assignment_statement operator: _ @operator)

;; --- Attributes ---
(attribute "@" @punctuation.special)
(attribute name: (value_identifier) @attribute)

;; --- Brackets ---
[
  "("
  ")"
  "{"
  "}"
  "<"
  ">"
  "["
  "]"
] @punctuation.bracket

;; --- Delimiters ---
[
  ","
  ";"
  ":"
  "="
  "::"
  "=>"
  "."
] @punctuation.delimiter
