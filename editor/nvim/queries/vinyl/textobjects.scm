; --- Functions ---

(function_definition) @function.outer
(function_definition
  body: (block) @function.inner)

; --- Classes / Data Structures (Structs, Enums, Tuples) ---

[
  (struct_definition)
  (enum_definition)
  (tuple_definition)
] @class.outer

(struct_definition
  (field_definition) @class.inner)

(enum_definition
  (enum_variant) @class.inner)

; --- Parameters & Arguments ---

(parameter) @parameter.inner
(parameters) @parameter.outer

(arguments) @parameter.outer

; --- Conditionals ---

(if_expression) @conditional.outer
(if_expression
  (block) @conditional.inner)

; --- Loops ---

[
  (while_statement)
  (loop_statement)
] @loop.outer

(while_statement
  (block) @loop.inner)

(loop_statement
  (block) @loop.inner)

; --- Blocks & Comments ---

(block) @block.inner
(block) @block.outer
(comment) @comment.outer
(call_expression) @call.outer
