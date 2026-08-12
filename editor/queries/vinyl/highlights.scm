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
  "type"
] @keyword

[
  "public"
  "mut"
] @keyword.modifier

"import" @keyword.import
(import_prefix) @keyword.import

(unary_expression operator: "not" @keyword.operator)
(binary_expression operator: ["and" "or"] @keyword.operator)

;; --- Imports & Modules ---
(import_name [
  (value_identifier)
  (type_identifier)
] @module)

(import_wildcard) @punctuation.delimiter

(import_group
  (value_identifier) @function)
(import_group
  (type_identifier) @type)

;; --- Qualified Paths ---
(qualified_value_path
  module: (_) @module
  function: (value_identifier) @function)

(qualified_type_path
  module: (_) @module
  type: (type_identifier) @type)

(qualified_value_path
  module: (value_identifier) @keyword.import
  (#any-of? @keyword.import "self" "package" "parent"))

(qualified_type_path
  module: (value_identifier) @keyword.import
  (#any-of? @keyword.import "self" "package" "parent"))

;; --- Functions ---
(function_definition
  name: (value_identifier) @function (#set! "priority" 105))

(call_expression
  function: (value_identifier) @function.call (#set! "priority" 105))

(call_expression
  function: (type_identifier) @constructor (#set! "priority" 105))

(call_expression
  function: (qualified_value_path
    function: (value_identifier) @function.call) (#set! "priority" 105))

;; --- Types & Definitions ---
(primitive_type) @type.builtin
(reference_type "&" @operator)
(simple_type (type_identifier) @type)
(generic_type (type_identifier) @type)
(struct_definition name: (type_identifier) @type)
(tuple_definition name: (type_identifier) @type)
(enum_definition name: (type_identifier) @type)
(struct_literal_expression
  name: (type_identifier) @type)
(struct_literal_expression
  name: (qualified_type_path
    type: (type_identifier) @type))
(struct_pattern (type_identifier) @type)
(struct_pattern
  (qualified_type_path
    type: (type_identifier) @type))

;; --- Enum Variants & Scoped Types ---
(enum_variant name: (type_identifier) @constructor)

(enum_variant_pattern
  path: (qualified_type_path
    type: (type_identifier) @constructor))

(scoped_type_expression
  path: (qualified_type_path
    type: (type_identifier) @constructor))

;; --- Fields & Properties ---
(field_definition name: (value_identifier) @property)
(field_access_expression field: [
  (value_identifier)
  (integer_literal)
] @property)
(struct_literal_fields name: (value_identifier) @property)
(field_pattern name: (value_identifier) @property)

;; --- Variables, Parameters & Patterns ---
(parameter name: (value_identifier) @variable.parameter)
(let_declaration name: (value_identifier) @variable)
(identifier_pattern) @variable
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
