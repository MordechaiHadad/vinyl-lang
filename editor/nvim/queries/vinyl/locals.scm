; --- Scopes ---

[
  (source_file)
  (function_definition)
  (block)
  (match_arm)
  (while_statement)
  (loop_statement)
  (if_expression)
] @local.scope

; --- Definitions ---

; Function definition
(function_definition
  name: (value_identifier) @local.definition.function)

; Types (Struct, Enum, Tuple definitions)
(struct_definition
  name: (type_identifier) @local.definition.type)

(enum_definition
  name: (type_identifier) @local.definition.type)

(tuple_definition
  name: (type_identifier) @local.definition.type)

; Variables and Parameters
(let_declaration
  name: (value_identifier) @local.definition.var)

(parameter
  name: (value_identifier) @local.definition.parameter)

(identifier_pattern
  (value_identifier) @local.definition.var)

; --- References ---

(value_identifier) @local.reference
(type_identifier) @local.reference
