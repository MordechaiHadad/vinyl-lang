; Containers that push interior lines right (+1 indent)
[
  (block)
  (struct_definition)
  (enum_definition)
  (match_expression)
  (match_statement)
  (struct_literal_fields)
  (parameters)
  (arguments)
  (array_expression)
  (array_type)
  (tuple_expression)
  (tuple_type)
  (tuple_definition)
] @indent.begin

; Closing delimiters snap back to match their parent container's start column
[
  "}"
  ")"
  "]"
] @indent.branch

; Control flow branches match their statement root
[
  "else"
] @indent.branch
