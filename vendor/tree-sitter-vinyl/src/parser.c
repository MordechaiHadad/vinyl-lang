#include <tree_sitter/parser.h>

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 13
#define STATE_COUNT 54
#define LARGE_STATE_COUNT 6
#define SYMBOL_COUNT 42
#define ALIAS_COUNT 0
#define TOKEN_COUNT 23
#define EXTERNAL_TOKEN_COUNT 0
#define FIELD_COUNT 8
#define MAX_ALIAS_SEQUENCE_LENGTH 5
#define PRODUCTION_ID_COUNT 7

enum {
  anon_sym_EQ = 1,
  anon_sym_SEMI = 2,
  sym_primitive_type = 3,
  sym_implicit_type = 4,
  sym_void_type = 5,
  anon_sym_LBRACK = 6,
  anon_sym_RBRACK = 7,
  sym_integer_literal = 8,
  sym_floating_point_literal = 9,
  anon_sym_DQUOTE = 10,
  aux_sym_string_literal_token1 = 11,
  anon_sym_SQUOTE = 12,
  aux_sym_char_literal_token1 = 13,
  anon_sym_true = 14,
  anon_sym_false = 15,
  sym_identifier = 16,
  anon_sym_new = 17,
  anon_sym_LPAREN = 18,
  anon_sym_COMMA = 19,
  anon_sym_RPAREN = 20,
  anon_sym_LBRACE = 21,
  anon_sym_RBRACE = 22,
  sym_source_file = 23,
  sym__statement = 24,
  sym__declaration = 25,
  sym_variable_declaration = 26,
  sym_function_declaration = 27,
  sym__type = 28,
  sym_array_type = 29,
  sym__literal = 30,
  sym_string_literal = 31,
  sym_char_literal = 32,
  sym_bool_literal = 33,
  sym__expression = 34,
  sym_array_creation_expression = 35,
  sym_parameters = 36,
  sym_parameter = 37,
  sym_block = 38,
  aux_sym_source_file_repeat1 = 39,
  aux_sym_string_literal_repeat1 = 40,
  aux_sym_parameters_repeat1 = 41,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [anon_sym_EQ] = "=",
  [anon_sym_SEMI] = ";",
  [sym_primitive_type] = "primitive_type",
  [sym_implicit_type] = "implicit_type",
  [sym_void_type] = "void_type",
  [anon_sym_LBRACK] = "[",
  [anon_sym_RBRACK] = "]",
  [sym_integer_literal] = "integer_literal",
  [sym_floating_point_literal] = "floating_point_literal",
  [anon_sym_DQUOTE] = "\"",
  [aux_sym_string_literal_token1] = "string_literal_token1",
  [anon_sym_SQUOTE] = "'",
  [aux_sym_char_literal_token1] = "char_literal_token1",
  [anon_sym_true] = "true",
  [anon_sym_false] = "false",
  [sym_identifier] = "identifier",
  [anon_sym_new] = "new",
  [anon_sym_LPAREN] = "(",
  [anon_sym_COMMA] = ",",
  [anon_sym_RPAREN] = ")",
  [anon_sym_LBRACE] = "{",
  [anon_sym_RBRACE] = "}",
  [sym_source_file] = "source_file",
  [sym__statement] = "_statement",
  [sym__declaration] = "_declaration",
  [sym_variable_declaration] = "variable_declaration",
  [sym_function_declaration] = "function_declaration",
  [sym__type] = "_type",
  [sym_array_type] = "array_type",
  [sym__literal] = "_literal",
  [sym_string_literal] = "string_literal",
  [sym_char_literal] = "char_literal",
  [sym_bool_literal] = "bool_literal",
  [sym__expression] = "_expression",
  [sym_array_creation_expression] = "array_creation_expression",
  [sym_parameters] = "parameters",
  [sym_parameter] = "parameter",
  [sym_block] = "block",
  [aux_sym_source_file_repeat1] = "source_file_repeat1",
  [aux_sym_string_literal_repeat1] = "string_literal_repeat1",
  [aux_sym_parameters_repeat1] = "parameters_repeat1",
};

static const TSSymbol ts_symbol_map[] = {
  [ts_builtin_sym_end] = ts_builtin_sym_end,
  [anon_sym_EQ] = anon_sym_EQ,
  [anon_sym_SEMI] = anon_sym_SEMI,
  [sym_primitive_type] = sym_primitive_type,
  [sym_implicit_type] = sym_implicit_type,
  [sym_void_type] = sym_void_type,
  [anon_sym_LBRACK] = anon_sym_LBRACK,
  [anon_sym_RBRACK] = anon_sym_RBRACK,
  [sym_integer_literal] = sym_integer_literal,
  [sym_floating_point_literal] = sym_floating_point_literal,
  [anon_sym_DQUOTE] = anon_sym_DQUOTE,
  [aux_sym_string_literal_token1] = aux_sym_string_literal_token1,
  [anon_sym_SQUOTE] = anon_sym_SQUOTE,
  [aux_sym_char_literal_token1] = aux_sym_char_literal_token1,
  [anon_sym_true] = anon_sym_true,
  [anon_sym_false] = anon_sym_false,
  [sym_identifier] = sym_identifier,
  [anon_sym_new] = anon_sym_new,
  [anon_sym_LPAREN] = anon_sym_LPAREN,
  [anon_sym_COMMA] = anon_sym_COMMA,
  [anon_sym_RPAREN] = anon_sym_RPAREN,
  [anon_sym_LBRACE] = anon_sym_LBRACE,
  [anon_sym_RBRACE] = anon_sym_RBRACE,
  [sym_source_file] = sym_source_file,
  [sym__statement] = sym__statement,
  [sym__declaration] = sym__declaration,
  [sym_variable_declaration] = sym_variable_declaration,
  [sym_function_declaration] = sym_function_declaration,
  [sym__type] = sym__type,
  [sym_array_type] = sym_array_type,
  [sym__literal] = sym__literal,
  [sym_string_literal] = sym_string_literal,
  [sym_char_literal] = sym_char_literal,
  [sym_bool_literal] = sym_bool_literal,
  [sym__expression] = sym__expression,
  [sym_array_creation_expression] = sym_array_creation_expression,
  [sym_parameters] = sym_parameters,
  [sym_parameter] = sym_parameter,
  [sym_block] = sym_block,
  [aux_sym_source_file_repeat1] = aux_sym_source_file_repeat1,
  [aux_sym_string_literal_repeat1] = aux_sym_string_literal_repeat1,
  [aux_sym_parameters_repeat1] = aux_sym_parameters_repeat1,
};

static const TSSymbolMetadata ts_symbol_metadata[] = {
  [ts_builtin_sym_end] = {
    .visible = false,
    .named = true,
  },
  [anon_sym_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_SEMI] = {
    .visible = true,
    .named = false,
  },
  [sym_primitive_type] = {
    .visible = true,
    .named = true,
  },
  [sym_implicit_type] = {
    .visible = true,
    .named = true,
  },
  [sym_void_type] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_LBRACK] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_RBRACK] = {
    .visible = true,
    .named = false,
  },
  [sym_integer_literal] = {
    .visible = true,
    .named = true,
  },
  [sym_floating_point_literal] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_DQUOTE] = {
    .visible = true,
    .named = false,
  },
  [aux_sym_string_literal_token1] = {
    .visible = false,
    .named = false,
  },
  [anon_sym_SQUOTE] = {
    .visible = true,
    .named = false,
  },
  [aux_sym_char_literal_token1] = {
    .visible = false,
    .named = false,
  },
  [anon_sym_true] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_false] = {
    .visible = true,
    .named = false,
  },
  [sym_identifier] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_new] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LPAREN] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_COMMA] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_RPAREN] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LBRACE] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_RBRACE] = {
    .visible = true,
    .named = false,
  },
  [sym_source_file] = {
    .visible = true,
    .named = true,
  },
  [sym__statement] = {
    .visible = false,
    .named = true,
  },
  [sym__declaration] = {
    .visible = false,
    .named = true,
  },
  [sym_variable_declaration] = {
    .visible = true,
    .named = true,
  },
  [sym_function_declaration] = {
    .visible = true,
    .named = true,
  },
  [sym__type] = {
    .visible = false,
    .named = true,
  },
  [sym_array_type] = {
    .visible = true,
    .named = true,
  },
  [sym__literal] = {
    .visible = false,
    .named = true,
  },
  [sym_string_literal] = {
    .visible = true,
    .named = true,
  },
  [sym_char_literal] = {
    .visible = true,
    .named = true,
  },
  [sym_bool_literal] = {
    .visible = true,
    .named = true,
  },
  [sym__expression] = {
    .visible = false,
    .named = true,
  },
  [sym_array_creation_expression] = {
    .visible = true,
    .named = true,
  },
  [sym_parameters] = {
    .visible = true,
    .named = true,
  },
  [sym_parameter] = {
    .visible = true,
    .named = true,
  },
  [sym_block] = {
    .visible = true,
    .named = true,
  },
  [aux_sym_source_file_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_string_literal_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_parameters_repeat1] = {
    .visible = false,
    .named = false,
  },
};

enum {
  field_body = 1,
  field_expression = 2,
  field_identifier = 3,
  field_name = 4,
  field_parameters = 5,
  field_return_type = 6,
  field_size = 7,
  field_type = 8,
};

static const char * const ts_field_names[] = {
  [0] = NULL,
  [field_body] = "body",
  [field_expression] = "expression",
  [field_identifier] = "identifier",
  [field_name] = "name",
  [field_parameters] = "parameters",
  [field_return_type] = "return_type",
  [field_size] = "size",
  [field_type] = "type",
};

static const TSFieldMapSlice ts_field_map_slices[PRODUCTION_ID_COUNT] = {
  [1] = {.index = 0, .length = 1},
  [2] = {.index = 1, .length = 2},
  [3] = {.index = 3, .length = 1},
  [4] = {.index = 4, .length = 4},
  [5] = {.index = 8, .length = 2},
  [6] = {.index = 10, .length = 3},
};

static const TSFieldMapEntry ts_field_map_entries[] = {
  [0] =
    {field_type, 1},
  [1] =
    {field_name, 1},
    {field_type, 0},
  [3] =
    {field_type, 0},
  [4] =
    {field_body, 3},
    {field_identifier, 1},
    {field_parameters, 2},
    {field_return_type, 0},
  [8] =
    {field_size, 2},
    {field_type, 0},
  [10] =
    {field_expression, 3},
    {field_name, 1},
    {field_type, 0},
};

static const TSSymbol ts_alias_sequences[PRODUCTION_ID_COUNT][MAX_ALIAS_SEQUENCE_LENGTH] = {
  [0] = {0},
};

static const uint16_t ts_non_terminal_alias_map[] = {
  0,
};

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(41);
      if (lookahead == '"') ADVANCE(51);
      if (lookahead == '\'') ADVANCE(54);
      if (lookahead == '(') ADVANCE(60);
      if (lookahead == ')') ADVANCE(62);
      if (lookahead == ',') ADVANCE(61);
      if (lookahead == ';') ADVANCE(43);
      if (lookahead == '=') ADVANCE(42);
      if (lookahead == '[') ADVANCE(47);
      if (lookahead == ']') ADVANCE(48);
      if (lookahead == 'b') ADVANCE(27);
      if (lookahead == 'c') ADVANCE(19);
      if (lookahead == 'f') ADVANCE(10);
      if (lookahead == 'i') ADVANCE(26);
      if (lookahead == 'n') ADVANCE(15);
      if (lookahead == 's') ADVANCE(37);
      if (lookahead == 't') ADVANCE(30);
      if (lookahead == 'u') ADVANCE(20);
      if (lookahead == 'v') ADVANCE(12);
      if (lookahead == '{') ADVANCE(63);
      if (lookahead == '}') ADVANCE(64);
      if (lookahead == '\t' ||
          lookahead == '\n' ||
          lookahead == '\r' ||
          lookahead == ' ') SKIP(0)
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(49);
      END_STATE();
    case 1:
      if (lookahead == '\n') SKIP(1)
      if (lookahead == '"') ADVANCE(51);
      if (lookahead == '\t' ||
          lookahead == '\r' ||
          lookahead == ' ') ADVANCE(52);
      if (lookahead != 0 &&
          lookahead != '\\') ADVANCE(53);
      END_STATE();
    case 2:
      if (lookahead == '1') ADVANCE(6);
      if (lookahead == '3') ADVANCE(4);
      if (lookahead == '6') ADVANCE(7);
      if (lookahead == '8') ADVANCE(44);
      END_STATE();
    case 3:
      if (lookahead == '1') ADVANCE(5);
      if (lookahead == '3') ADVANCE(4);
      if (lookahead == '6') ADVANCE(7);
      END_STATE();
    case 4:
      if (lookahead == '2') ADVANCE(44);
      END_STATE();
    case 5:
      if (lookahead == '2') ADVANCE(8);
      END_STATE();
    case 6:
      if (lookahead == '2') ADVANCE(8);
      if (lookahead == '6') ADVANCE(44);
      END_STATE();
    case 7:
      if (lookahead == '4') ADVANCE(44);
      END_STATE();
    case 8:
      if (lookahead == '8') ADVANCE(44);
      END_STATE();
    case 9:
      if (lookahead == ';') ADVANCE(43);
      if (lookahead == '[') ADVANCE(47);
      if (lookahead == '\t' ||
          lookahead == '\n' ||
          lookahead == '\r' ||
          lookahead == ' ') SKIP(9)
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(58);
      END_STATE();
    case 10:
      if (lookahead == 'a') ADVANCE(23);
      if (lookahead == 'l') ADVANCE(28);
      END_STATE();
    case 11:
      if (lookahead == 'a') ADVANCE(36);
      END_STATE();
    case 12:
      if (lookahead == 'a') ADVANCE(31);
      if (lookahead == 'o') ADVANCE(21);
      END_STATE();
    case 13:
      if (lookahead == 'a') ADVANCE(32);
      END_STATE();
    case 14:
      if (lookahead == 'd') ADVANCE(46);
      END_STATE();
    case 15:
      if (lookahead == 'e') ADVANCE(39);
      END_STATE();
    case 16:
      if (lookahead == 'e') ADVANCE(56);
      END_STATE();
    case 17:
      if (lookahead == 'e') ADVANCE(57);
      END_STATE();
    case 18:
      if (lookahead == 'g') ADVANCE(44);
      END_STATE();
    case 19:
      if (lookahead == 'h') ADVANCE(13);
      END_STATE();
    case 20:
      if (lookahead == 'i') ADVANCE(26);
      END_STATE();
    case 21:
      if (lookahead == 'i') ADVANCE(14);
      END_STATE();
    case 22:
      if (lookahead == 'i') ADVANCE(25);
      END_STATE();
    case 23:
      if (lookahead == 'l') ADVANCE(34);
      END_STATE();
    case 24:
      if (lookahead == 'l') ADVANCE(44);
      END_STATE();
    case 25:
      if (lookahead == 'n') ADVANCE(18);
      END_STATE();
    case 26:
      if (lookahead == 'n') ADVANCE(35);
      END_STATE();
    case 27:
      if (lookahead == 'o') ADVANCE(29);
      END_STATE();
    case 28:
      if (lookahead == 'o') ADVANCE(11);
      END_STATE();
    case 29:
      if (lookahead == 'o') ADVANCE(24);
      END_STATE();
    case 30:
      if (lookahead == 'r') ADVANCE(38);
      END_STATE();
    case 31:
      if (lookahead == 'r') ADVANCE(45);
      END_STATE();
    case 32:
      if (lookahead == 'r') ADVANCE(44);
      END_STATE();
    case 33:
      if (lookahead == 'r') ADVANCE(22);
      END_STATE();
    case 34:
      if (lookahead == 's') ADVANCE(17);
      END_STATE();
    case 35:
      if (lookahead == 't') ADVANCE(2);
      END_STATE();
    case 36:
      if (lookahead == 't') ADVANCE(3);
      END_STATE();
    case 37:
      if (lookahead == 't') ADVANCE(33);
      END_STATE();
    case 38:
      if (lookahead == 'u') ADVANCE(16);
      END_STATE();
    case 39:
      if (lookahead == 'w') ADVANCE(59);
      END_STATE();
    case 40:
      if (lookahead != 0 &&
          lookahead != '\'' &&
          lookahead != '\\') ADVANCE(55);
      END_STATE();
    case 41:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 42:
      ACCEPT_TOKEN(anon_sym_EQ);
      END_STATE();
    case 43:
      ACCEPT_TOKEN(anon_sym_SEMI);
      END_STATE();
    case 44:
      ACCEPT_TOKEN(sym_primitive_type);
      END_STATE();
    case 45:
      ACCEPT_TOKEN(sym_implicit_type);
      END_STATE();
    case 46:
      ACCEPT_TOKEN(sym_void_type);
      END_STATE();
    case 47:
      ACCEPT_TOKEN(anon_sym_LBRACK);
      END_STATE();
    case 48:
      ACCEPT_TOKEN(anon_sym_RBRACK);
      END_STATE();
    case 49:
      ACCEPT_TOKEN(sym_integer_literal);
      if (lookahead == '.') ADVANCE(50);
      if (('0' <= lookahead && lookahead <= '9') ||
          lookahead == '_') ADVANCE(49);
      END_STATE();
    case 50:
      ACCEPT_TOKEN(sym_floating_point_literal);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(50);
      END_STATE();
    case 51:
      ACCEPT_TOKEN(anon_sym_DQUOTE);
      END_STATE();
    case 52:
      ACCEPT_TOKEN(aux_sym_string_literal_token1);
      if (lookahead == '\t' ||
          lookahead == '\r' ||
          lookahead == ' ') ADVANCE(52);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '"' &&
          lookahead != '\\') ADVANCE(53);
      END_STATE();
    case 53:
      ACCEPT_TOKEN(aux_sym_string_literal_token1);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '"' &&
          lookahead != '\\') ADVANCE(53);
      END_STATE();
    case 54:
      ACCEPT_TOKEN(anon_sym_SQUOTE);
      END_STATE();
    case 55:
      ACCEPT_TOKEN(aux_sym_char_literal_token1);
      END_STATE();
    case 56:
      ACCEPT_TOKEN(anon_sym_true);
      END_STATE();
    case 57:
      ACCEPT_TOKEN(anon_sym_false);
      END_STATE();
    case 58:
      ACCEPT_TOKEN(sym_identifier);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(58);
      END_STATE();
    case 59:
      ACCEPT_TOKEN(anon_sym_new);
      END_STATE();
    case 60:
      ACCEPT_TOKEN(anon_sym_LPAREN);
      END_STATE();
    case 61:
      ACCEPT_TOKEN(anon_sym_COMMA);
      END_STATE();
    case 62:
      ACCEPT_TOKEN(anon_sym_RPAREN);
      END_STATE();
    case 63:
      ACCEPT_TOKEN(anon_sym_LBRACE);
      END_STATE();
    case 64:
      ACCEPT_TOKEN(anon_sym_RBRACE);
      END_STATE();
    default:
      return false;
  }
}

static const TSLexMode ts_lex_modes[STATE_COUNT] = {
  [0] = {.lex_state = 0},
  [1] = {.lex_state = 0},
  [2] = {.lex_state = 0},
  [3] = {.lex_state = 0},
  [4] = {.lex_state = 0},
  [5] = {.lex_state = 0},
  [6] = {.lex_state = 0},
  [7] = {.lex_state = 0},
  [8] = {.lex_state = 0},
  [9] = {.lex_state = 0},
  [10] = {.lex_state = 0},
  [11] = {.lex_state = 0},
  [12] = {.lex_state = 0},
  [13] = {.lex_state = 0},
  [14] = {.lex_state = 0},
  [15] = {.lex_state = 0},
  [16] = {.lex_state = 0},
  [17] = {.lex_state = 0},
  [18] = {.lex_state = 0},
  [19] = {.lex_state = 0},
  [20] = {.lex_state = 0},
  [21] = {.lex_state = 0},
  [22] = {.lex_state = 1},
  [23] = {.lex_state = 9},
  [24] = {.lex_state = 0},
  [25] = {.lex_state = 1},
  [26] = {.lex_state = 1},
  [27] = {.lex_state = 0},
  [28] = {.lex_state = 9},
  [29] = {.lex_state = 0},
  [30] = {.lex_state = 0},
  [31] = {.lex_state = 0},
  [32] = {.lex_state = 0},
  [33] = {.lex_state = 0},
  [34] = {.lex_state = 9},
  [35] = {.lex_state = 0},
  [36] = {.lex_state = 0},
  [37] = {.lex_state = 0},
  [38] = {.lex_state = 0},
  [39] = {.lex_state = 9},
  [40] = {.lex_state = 0},
  [41] = {.lex_state = 0},
  [42] = {.lex_state = 0},
  [43] = {.lex_state = 0},
  [44] = {.lex_state = 0},
  [45] = {.lex_state = 0},
  [46] = {.lex_state = 0},
  [47] = {.lex_state = 40},
  [48] = {.lex_state = 9},
  [49] = {.lex_state = 9},
  [50] = {.lex_state = 0},
  [51] = {.lex_state = 0},
  [52] = {.lex_state = 0},
  [53] = {.lex_state = 0},
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [ts_builtin_sym_end] = ACTIONS(1),
    [anon_sym_EQ] = ACTIONS(1),
    [anon_sym_SEMI] = ACTIONS(1),
    [sym_primitive_type] = ACTIONS(1),
    [sym_implicit_type] = ACTIONS(1),
    [sym_void_type] = ACTIONS(1),
    [anon_sym_LBRACK] = ACTIONS(1),
    [anon_sym_RBRACK] = ACTIONS(1),
    [sym_integer_literal] = ACTIONS(1),
    [sym_floating_point_literal] = ACTIONS(1),
    [anon_sym_DQUOTE] = ACTIONS(1),
    [anon_sym_SQUOTE] = ACTIONS(1),
    [anon_sym_true] = ACTIONS(1),
    [anon_sym_false] = ACTIONS(1),
    [anon_sym_new] = ACTIONS(1),
    [anon_sym_LPAREN] = ACTIONS(1),
    [anon_sym_COMMA] = ACTIONS(1),
    [anon_sym_RPAREN] = ACTIONS(1),
    [anon_sym_LBRACE] = ACTIONS(1),
    [anon_sym_RBRACE] = ACTIONS(1),
  },
  [1] = {
    [sym_source_file] = STATE(42),
    [sym__statement] = STATE(3),
    [sym__declaration] = STATE(3),
    [sym_variable_declaration] = STATE(3),
    [sym_function_declaration] = STATE(3),
    [sym__type] = STATE(39),
    [sym_array_type] = STATE(39),
    [sym__literal] = STATE(3),
    [sym_string_literal] = STATE(3),
    [sym_char_literal] = STATE(3),
    [sym_bool_literal] = STATE(3),
    [sym__expression] = STATE(3),
    [sym_array_creation_expression] = STATE(3),
    [aux_sym_source_file_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(3),
    [sym_primitive_type] = ACTIONS(5),
    [sym_implicit_type] = ACTIONS(7),
    [sym_void_type] = ACTIONS(9),
    [sym_integer_literal] = ACTIONS(11),
    [sym_floating_point_literal] = ACTIONS(13),
    [anon_sym_DQUOTE] = ACTIONS(15),
    [anon_sym_SQUOTE] = ACTIONS(17),
    [anon_sym_true] = ACTIONS(19),
    [anon_sym_false] = ACTIONS(19),
    [anon_sym_new] = ACTIONS(21),
  },
  [2] = {
    [sym__statement] = STATE(2),
    [sym__declaration] = STATE(2),
    [sym_variable_declaration] = STATE(2),
    [sym_function_declaration] = STATE(2),
    [sym__type] = STATE(39),
    [sym_array_type] = STATE(39),
    [sym__literal] = STATE(2),
    [sym_string_literal] = STATE(2),
    [sym_char_literal] = STATE(2),
    [sym_bool_literal] = STATE(2),
    [sym__expression] = STATE(2),
    [sym_array_creation_expression] = STATE(2),
    [aux_sym_source_file_repeat1] = STATE(2),
    [ts_builtin_sym_end] = ACTIONS(23),
    [sym_primitive_type] = ACTIONS(25),
    [sym_implicit_type] = ACTIONS(28),
    [sym_void_type] = ACTIONS(31),
    [sym_integer_literal] = ACTIONS(34),
    [sym_floating_point_literal] = ACTIONS(37),
    [anon_sym_DQUOTE] = ACTIONS(40),
    [anon_sym_SQUOTE] = ACTIONS(43),
    [anon_sym_true] = ACTIONS(46),
    [anon_sym_false] = ACTIONS(46),
    [anon_sym_new] = ACTIONS(49),
    [anon_sym_RBRACE] = ACTIONS(23),
  },
  [3] = {
    [sym__statement] = STATE(2),
    [sym__declaration] = STATE(2),
    [sym_variable_declaration] = STATE(2),
    [sym_function_declaration] = STATE(2),
    [sym__type] = STATE(39),
    [sym_array_type] = STATE(39),
    [sym__literal] = STATE(2),
    [sym_string_literal] = STATE(2),
    [sym_char_literal] = STATE(2),
    [sym_bool_literal] = STATE(2),
    [sym__expression] = STATE(2),
    [sym_array_creation_expression] = STATE(2),
    [aux_sym_source_file_repeat1] = STATE(2),
    [ts_builtin_sym_end] = ACTIONS(52),
    [sym_primitive_type] = ACTIONS(5),
    [sym_implicit_type] = ACTIONS(7),
    [sym_void_type] = ACTIONS(9),
    [sym_integer_literal] = ACTIONS(54),
    [sym_floating_point_literal] = ACTIONS(56),
    [anon_sym_DQUOTE] = ACTIONS(15),
    [anon_sym_SQUOTE] = ACTIONS(17),
    [anon_sym_true] = ACTIONS(19),
    [anon_sym_false] = ACTIONS(19),
    [anon_sym_new] = ACTIONS(21),
  },
  [4] = {
    [sym__statement] = STATE(2),
    [sym__declaration] = STATE(2),
    [sym_variable_declaration] = STATE(2),
    [sym_function_declaration] = STATE(2),
    [sym__type] = STATE(39),
    [sym_array_type] = STATE(39),
    [sym__literal] = STATE(2),
    [sym_string_literal] = STATE(2),
    [sym_char_literal] = STATE(2),
    [sym_bool_literal] = STATE(2),
    [sym__expression] = STATE(2),
    [sym_array_creation_expression] = STATE(2),
    [aux_sym_source_file_repeat1] = STATE(2),
    [sym_primitive_type] = ACTIONS(5),
    [sym_implicit_type] = ACTIONS(7),
    [sym_void_type] = ACTIONS(9),
    [sym_integer_literal] = ACTIONS(54),
    [sym_floating_point_literal] = ACTIONS(56),
    [anon_sym_DQUOTE] = ACTIONS(15),
    [anon_sym_SQUOTE] = ACTIONS(17),
    [anon_sym_true] = ACTIONS(19),
    [anon_sym_false] = ACTIONS(19),
    [anon_sym_new] = ACTIONS(21),
    [anon_sym_RBRACE] = ACTIONS(58),
  },
  [5] = {
    [sym__statement] = STATE(4),
    [sym__declaration] = STATE(4),
    [sym_variable_declaration] = STATE(4),
    [sym_function_declaration] = STATE(4),
    [sym__type] = STATE(39),
    [sym_array_type] = STATE(39),
    [sym__literal] = STATE(4),
    [sym_string_literal] = STATE(4),
    [sym_char_literal] = STATE(4),
    [sym_bool_literal] = STATE(4),
    [sym__expression] = STATE(4),
    [sym_array_creation_expression] = STATE(4),
    [aux_sym_source_file_repeat1] = STATE(4),
    [sym_primitive_type] = ACTIONS(5),
    [sym_implicit_type] = ACTIONS(7),
    [sym_void_type] = ACTIONS(9),
    [sym_integer_literal] = ACTIONS(60),
    [sym_floating_point_literal] = ACTIONS(62),
    [anon_sym_DQUOTE] = ACTIONS(15),
    [anon_sym_SQUOTE] = ACTIONS(17),
    [anon_sym_true] = ACTIONS(19),
    [anon_sym_false] = ACTIONS(19),
    [anon_sym_new] = ACTIONS(21),
    [anon_sym_RBRACE] = ACTIONS(64),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 3,
    ACTIONS(68), 1,
      anon_sym_LBRACK,
    ACTIONS(70), 1,
      sym_integer_literal,
    ACTIONS(66), 12,
      ts_builtin_sym_end,
      anon_sym_SEMI,
      sym_primitive_type,
      sym_implicit_type,
      sym_void_type,
      sym_floating_point_literal,
      anon_sym_DQUOTE,
      anon_sym_SQUOTE,
      anon_sym_true,
      anon_sym_false,
      anon_sym_new,
      anon_sym_RBRACE,
  [21] = 2,
    ACTIONS(74), 1,
      sym_integer_literal,
    ACTIONS(72), 12,
      ts_builtin_sym_end,
      anon_sym_SEMI,
      sym_primitive_type,
      sym_implicit_type,
      sym_void_type,
      sym_floating_point_literal,
      anon_sym_DQUOTE,
      anon_sym_SQUOTE,
      anon_sym_true,
      anon_sym_false,
      anon_sym_new,
      anon_sym_RBRACE,
  [39] = 2,
    ACTIONS(78), 1,
      sym_integer_literal,
    ACTIONS(76), 12,
      ts_builtin_sym_end,
      anon_sym_SEMI,
      sym_primitive_type,
      sym_implicit_type,
      sym_void_type,
      sym_floating_point_literal,
      anon_sym_DQUOTE,
      anon_sym_SQUOTE,
      anon_sym_true,
      anon_sym_false,
      anon_sym_new,
      anon_sym_RBRACE,
  [57] = 2,
    ACTIONS(82), 1,
      sym_integer_literal,
    ACTIONS(80), 12,
      ts_builtin_sym_end,
      sym_primitive_type,
      sym_implicit_type,
      sym_void_type,
      anon_sym_LBRACK,
      sym_floating_point_literal,
      anon_sym_DQUOTE,
      anon_sym_SQUOTE,
      anon_sym_true,
      anon_sym_false,
      anon_sym_new,
      anon_sym_RBRACE,
  [75] = 2,
    ACTIONS(86), 1,
      sym_integer_literal,
    ACTIONS(84), 12,
      ts_builtin_sym_end,
      anon_sym_SEMI,
      sym_primitive_type,
      sym_implicit_type,
      sym_void_type,
      sym_floating_point_literal,
      anon_sym_DQUOTE,
      anon_sym_SQUOTE,
      anon_sym_true,
      anon_sym_false,
      anon_sym_new,
      anon_sym_RBRACE,
  [93] = 2,
    ACTIONS(90), 1,
      sym_integer_literal,
    ACTIONS(88), 12,
      ts_builtin_sym_end,
      anon_sym_SEMI,
      sym_primitive_type,
      sym_implicit_type,
      sym_void_type,
      sym_floating_point_literal,
      anon_sym_DQUOTE,
      anon_sym_SQUOTE,
      anon_sym_true,
      anon_sym_false,
      anon_sym_new,
      anon_sym_RBRACE,
  [111] = 2,
    ACTIONS(94), 1,
      sym_integer_literal,
    ACTIONS(92), 12,
      ts_builtin_sym_end,
      sym_primitive_type,
      sym_implicit_type,
      sym_void_type,
      anon_sym_LBRACK,
      sym_floating_point_literal,
      anon_sym_DQUOTE,
      anon_sym_SQUOTE,
      anon_sym_true,
      anon_sym_false,
      anon_sym_new,
      anon_sym_RBRACE,
  [129] = 7,
    ACTIONS(15), 1,
      anon_sym_DQUOTE,
    ACTIONS(17), 1,
      anon_sym_SQUOTE,
    ACTIONS(96), 1,
      sym_integer_literal,
    ACTIONS(98), 1,
      sym_floating_point_literal,
    ACTIONS(100), 1,
      anon_sym_new,
    ACTIONS(19), 2,
      anon_sym_true,
      anon_sym_false,
    STATE(41), 6,
      sym__literal,
      sym_string_literal,
      sym_char_literal,
      sym_bool_literal,
      sym__expression,
      sym_array_creation_expression,
  [157] = 2,
    ACTIONS(104), 1,
      sym_integer_literal,
    ACTIONS(102), 11,
      ts_builtin_sym_end,
      sym_primitive_type,
      sym_implicit_type,
      sym_void_type,
      sym_floating_point_literal,
      anon_sym_DQUOTE,
      anon_sym_SQUOTE,
      anon_sym_true,
      anon_sym_false,
      anon_sym_new,
      anon_sym_RBRACE,
  [174] = 2,
    ACTIONS(108), 1,
      sym_integer_literal,
    ACTIONS(106), 11,
      ts_builtin_sym_end,
      sym_primitive_type,
      sym_implicit_type,
      sym_void_type,
      sym_floating_point_literal,
      anon_sym_DQUOTE,
      anon_sym_SQUOTE,
      anon_sym_true,
      anon_sym_false,
      anon_sym_new,
      anon_sym_RBRACE,
  [191] = 2,
    ACTIONS(112), 1,
      sym_integer_literal,
    ACTIONS(110), 11,
      ts_builtin_sym_end,
      sym_primitive_type,
      sym_implicit_type,
      sym_void_type,
      sym_floating_point_literal,
      anon_sym_DQUOTE,
      anon_sym_SQUOTE,
      anon_sym_true,
      anon_sym_false,
      anon_sym_new,
      anon_sym_RBRACE,
  [208] = 2,
    ACTIONS(116), 1,
      sym_integer_literal,
    ACTIONS(114), 11,
      ts_builtin_sym_end,
      sym_primitive_type,
      sym_implicit_type,
      sym_void_type,
      sym_floating_point_literal,
      anon_sym_DQUOTE,
      anon_sym_SQUOTE,
      anon_sym_true,
      anon_sym_false,
      anon_sym_new,
      anon_sym_RBRACE,
  [225] = 2,
    ACTIONS(120), 1,
      sym_integer_literal,
    ACTIONS(118), 11,
      ts_builtin_sym_end,
      sym_primitive_type,
      sym_implicit_type,
      sym_void_type,
      sym_floating_point_literal,
      anon_sym_DQUOTE,
      anon_sym_SQUOTE,
      anon_sym_true,
      anon_sym_false,
      anon_sym_new,
      anon_sym_RBRACE,
  [242] = 4,
    ACTIONS(122), 1,
      sym_primitive_type,
    ACTIONS(124), 1,
      anon_sym_RPAREN,
    STATE(31), 1,
      sym_parameter,
    STATE(34), 2,
      sym__type,
      sym_array_type,
  [256] = 3,
    ACTIONS(122), 1,
      sym_primitive_type,
    STATE(32), 1,
      sym_parameter,
    STATE(34), 2,
      sym__type,
      sym_array_type,
  [267] = 4,
    ACTIONS(126), 1,
      anon_sym_EQ,
    ACTIONS(128), 1,
      anon_sym_SEMI,
    ACTIONS(130), 1,
      anon_sym_LPAREN,
    STATE(35), 1,
      sym_parameters,
  [280] = 3,
    ACTIONS(132), 1,
      anon_sym_DQUOTE,
    ACTIONS(134), 1,
      aux_sym_string_literal_token1,
    STATE(26), 1,
      aux_sym_string_literal_repeat1,
  [290] = 1,
    ACTIONS(80), 3,
      anon_sym_SEMI,
      anon_sym_LBRACK,
      sym_identifier,
  [296] = 3,
    ACTIONS(136), 1,
      sym_primitive_type,
    STATE(6), 1,
      sym_array_type,
    STATE(52), 1,
      sym__type,
  [306] = 3,
    ACTIONS(138), 1,
      anon_sym_DQUOTE,
    ACTIONS(140), 1,
      aux_sym_string_literal_token1,
    STATE(22), 1,
      aux_sym_string_literal_repeat1,
  [316] = 3,
    ACTIONS(142), 1,
      anon_sym_DQUOTE,
    ACTIONS(144), 1,
      aux_sym_string_literal_token1,
    STATE(26), 1,
      aux_sym_string_literal_repeat1,
  [326] = 3,
    ACTIONS(147), 1,
      anon_sym_COMMA,
    ACTIONS(150), 1,
      anon_sym_RPAREN,
    STATE(27), 1,
      aux_sym_parameters_repeat1,
  [336] = 1,
    ACTIONS(92), 3,
      anon_sym_SEMI,
      anon_sym_LBRACK,
      sym_identifier,
  [342] = 3,
    ACTIONS(152), 1,
      sym_primitive_type,
    STATE(6), 1,
      sym_array_type,
    STATE(44), 1,
      sym__type,
  [352] = 3,
    ACTIONS(154), 1,
      anon_sym_COMMA,
    ACTIONS(156), 1,
      anon_sym_RPAREN,
    STATE(27), 1,
      aux_sym_parameters_repeat1,
  [362] = 3,
    ACTIONS(154), 1,
      anon_sym_COMMA,
    ACTIONS(158), 1,
      anon_sym_RPAREN,
    STATE(30), 1,
      aux_sym_parameters_repeat1,
  [372] = 1,
    ACTIONS(150), 2,
      anon_sym_COMMA,
      anon_sym_RPAREN,
  [377] = 2,
    ACTIONS(160), 1,
      anon_sym_RBRACK,
    ACTIONS(162), 1,
      sym_integer_literal,
  [384] = 2,
    ACTIONS(164), 1,
      anon_sym_LBRACK,
    ACTIONS(166), 1,
      sym_identifier,
  [391] = 2,
    ACTIONS(168), 1,
      anon_sym_LBRACE,
    STATE(18), 1,
      sym_block,
  [398] = 2,
    ACTIONS(130), 1,
      anon_sym_LPAREN,
    STATE(35), 1,
      sym_parameters,
  [405] = 2,
    ACTIONS(126), 1,
      anon_sym_EQ,
    ACTIONS(128), 1,
      anon_sym_SEMI,
  [412] = 1,
    ACTIONS(170), 2,
      anon_sym_COMMA,
      anon_sym_RPAREN,
  [417] = 2,
    ACTIONS(164), 1,
      anon_sym_LBRACK,
    ACTIONS(172), 1,
      sym_identifier,
  [424] = 2,
    ACTIONS(174), 1,
      anon_sym_RBRACK,
    ACTIONS(176), 1,
      sym_integer_literal,
  [431] = 1,
    ACTIONS(178), 1,
      anon_sym_SEMI,
  [435] = 1,
    ACTIONS(180), 1,
      ts_builtin_sym_end,
  [439] = 1,
    ACTIONS(182), 1,
      anon_sym_LBRACE,
  [443] = 1,
    ACTIONS(184), 1,
      anon_sym_LBRACK,
  [447] = 1,
    ACTIONS(186), 1,
      anon_sym_LBRACE,
  [451] = 1,
    ACTIONS(188), 1,
      anon_sym_RBRACK,
  [455] = 1,
    ACTIONS(190), 1,
      aux_sym_char_literal_token1,
  [459] = 1,
    ACTIONS(192), 1,
      sym_identifier,
  [463] = 1,
    ACTIONS(194), 1,
      sym_identifier,
  [467] = 1,
    ACTIONS(196), 1,
      anon_sym_SQUOTE,
  [471] = 1,
    ACTIONS(198), 1,
      anon_sym_RBRACK,
  [475] = 1,
    ACTIONS(164), 1,
      anon_sym_LBRACK,
  [479] = 1,
    ACTIONS(200), 1,
      anon_sym_LBRACE,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(6)] = 0,
  [SMALL_STATE(7)] = 21,
  [SMALL_STATE(8)] = 39,
  [SMALL_STATE(9)] = 57,
  [SMALL_STATE(10)] = 75,
  [SMALL_STATE(11)] = 93,
  [SMALL_STATE(12)] = 111,
  [SMALL_STATE(13)] = 129,
  [SMALL_STATE(14)] = 157,
  [SMALL_STATE(15)] = 174,
  [SMALL_STATE(16)] = 191,
  [SMALL_STATE(17)] = 208,
  [SMALL_STATE(18)] = 225,
  [SMALL_STATE(19)] = 242,
  [SMALL_STATE(20)] = 256,
  [SMALL_STATE(21)] = 267,
  [SMALL_STATE(22)] = 280,
  [SMALL_STATE(23)] = 290,
  [SMALL_STATE(24)] = 296,
  [SMALL_STATE(25)] = 306,
  [SMALL_STATE(26)] = 316,
  [SMALL_STATE(27)] = 326,
  [SMALL_STATE(28)] = 336,
  [SMALL_STATE(29)] = 342,
  [SMALL_STATE(30)] = 352,
  [SMALL_STATE(31)] = 362,
  [SMALL_STATE(32)] = 372,
  [SMALL_STATE(33)] = 377,
  [SMALL_STATE(34)] = 384,
  [SMALL_STATE(35)] = 391,
  [SMALL_STATE(36)] = 398,
  [SMALL_STATE(37)] = 405,
  [SMALL_STATE(38)] = 412,
  [SMALL_STATE(39)] = 417,
  [SMALL_STATE(40)] = 424,
  [SMALL_STATE(41)] = 431,
  [SMALL_STATE(42)] = 435,
  [SMALL_STATE(43)] = 439,
  [SMALL_STATE(44)] = 443,
  [SMALL_STATE(45)] = 447,
  [SMALL_STATE(46)] = 451,
  [SMALL_STATE(47)] = 455,
  [SMALL_STATE(48)] = 459,
  [SMALL_STATE(49)] = 463,
  [SMALL_STATE(50)] = 467,
  [SMALL_STATE(51)] = 471,
  [SMALL_STATE(52)] = 475,
  [SMALL_STATE(53)] = 479,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0),
  [5] = {.entry = {.count = 1, .reusable = true}}, SHIFT(39),
  [7] = {.entry = {.count = 1, .reusable = true}}, SHIFT(48),
  [9] = {.entry = {.count = 1, .reusable = true}}, SHIFT(49),
  [11] = {.entry = {.count = 1, .reusable = false}}, SHIFT(3),
  [13] = {.entry = {.count = 1, .reusable = true}}, SHIFT(3),
  [15] = {.entry = {.count = 1, .reusable = true}}, SHIFT(25),
  [17] = {.entry = {.count = 1, .reusable = true}}, SHIFT(47),
  [19] = {.entry = {.count = 1, .reusable = true}}, SHIFT(7),
  [21] = {.entry = {.count = 1, .reusable = true}}, SHIFT(29),
  [23] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2),
  [25] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(39),
  [28] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(48),
  [31] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(49),
  [34] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(2),
  [37] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(2),
  [40] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(25),
  [43] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(47),
  [46] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(7),
  [49] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(29),
  [52] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1),
  [54] = {.entry = {.count = 1, .reusable = false}}, SHIFT(2),
  [56] = {.entry = {.count = 1, .reusable = true}}, SHIFT(2),
  [58] = {.entry = {.count = 1, .reusable = true}}, SHIFT(14),
  [60] = {.entry = {.count = 1, .reusable = false}}, SHIFT(4),
  [62] = {.entry = {.count = 1, .reusable = true}}, SHIFT(4),
  [64] = {.entry = {.count = 1, .reusable = true}}, SHIFT(15),
  [66] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_array_creation_expression, 2, .production_id = 1),
  [68] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__type, 1),
  [70] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_array_creation_expression, 2, .production_id = 1),
  [72] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_bool_literal, 1),
  [74] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_bool_literal, 1),
  [76] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_string_literal, 2),
  [78] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_string_literal, 2),
  [80] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_array_type, 4, .production_id = 5),
  [82] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_array_type, 4, .production_id = 5),
  [84] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_char_literal, 3),
  [86] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_char_literal, 3),
  [88] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_string_literal, 3),
  [90] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_string_literal, 3),
  [92] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_array_type, 3, .production_id = 3),
  [94] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_array_type, 3, .production_id = 3),
  [96] = {.entry = {.count = 1, .reusable = false}}, SHIFT(41),
  [98] = {.entry = {.count = 1, .reusable = true}}, SHIFT(41),
  [100] = {.entry = {.count = 1, .reusable = true}}, SHIFT(24),
  [102] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_block, 3),
  [104] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_block, 3),
  [106] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_block, 2),
  [108] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_block, 2),
  [110] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_variable_declaration, 5, .production_id = 6),
  [112] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_variable_declaration, 5, .production_id = 6),
  [114] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_variable_declaration, 3, .production_id = 2),
  [116] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_variable_declaration, 3, .production_id = 2),
  [118] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_function_declaration, 4, .production_id = 4),
  [120] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_function_declaration, 4, .production_id = 4),
  [122] = {.entry = {.count = 1, .reusable = true}}, SHIFT(34),
  [124] = {.entry = {.count = 1, .reusable = true}}, SHIFT(43),
  [126] = {.entry = {.count = 1, .reusable = true}}, SHIFT(13),
  [128] = {.entry = {.count = 1, .reusable = true}}, SHIFT(17),
  [130] = {.entry = {.count = 1, .reusable = true}}, SHIFT(19),
  [132] = {.entry = {.count = 1, .reusable = false}}, SHIFT(11),
  [134] = {.entry = {.count = 1, .reusable = true}}, SHIFT(26),
  [136] = {.entry = {.count = 1, .reusable = true}}, SHIFT(52),
  [138] = {.entry = {.count = 1, .reusable = false}}, SHIFT(8),
  [140] = {.entry = {.count = 1, .reusable = true}}, SHIFT(22),
  [142] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym_string_literal_repeat1, 2),
  [144] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_string_literal_repeat1, 2), SHIFT_REPEAT(26),
  [147] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_parameters_repeat1, 2), SHIFT_REPEAT(20),
  [150] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_parameters_repeat1, 2),
  [152] = {.entry = {.count = 1, .reusable = true}}, SHIFT(44),
  [154] = {.entry = {.count = 1, .reusable = true}}, SHIFT(20),
  [156] = {.entry = {.count = 1, .reusable = true}}, SHIFT(45),
  [158] = {.entry = {.count = 1, .reusable = true}}, SHIFT(53),
  [160] = {.entry = {.count = 1, .reusable = true}}, SHIFT(28),
  [162] = {.entry = {.count = 1, .reusable = true}}, SHIFT(46),
  [164] = {.entry = {.count = 1, .reusable = true}}, SHIFT(33),
  [166] = {.entry = {.count = 1, .reusable = true}}, SHIFT(38),
  [168] = {.entry = {.count = 1, .reusable = true}}, SHIFT(5),
  [170] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_parameter, 2, .production_id = 2),
  [172] = {.entry = {.count = 1, .reusable = true}}, SHIFT(21),
  [174] = {.entry = {.count = 1, .reusable = true}}, SHIFT(12),
  [176] = {.entry = {.count = 1, .reusable = true}}, SHIFT(51),
  [178] = {.entry = {.count = 1, .reusable = true}}, SHIFT(16),
  [180] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
  [182] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_parameters, 2),
  [184] = {.entry = {.count = 1, .reusable = true}}, SHIFT(40),
  [186] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_parameters, 4),
  [188] = {.entry = {.count = 1, .reusable = true}}, SHIFT(23),
  [190] = {.entry = {.count = 1, .reusable = true}}, SHIFT(50),
  [192] = {.entry = {.count = 1, .reusable = true}}, SHIFT(37),
  [194] = {.entry = {.count = 1, .reusable = true}}, SHIFT(36),
  [196] = {.entry = {.count = 1, .reusable = true}}, SHIFT(10),
  [198] = {.entry = {.count = 1, .reusable = true}}, SHIFT(9),
  [200] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_parameters, 3),
};

#ifdef __cplusplus
extern "C" {
#endif
#ifdef _WIN32
#define extern __declspec(dllexport)
#endif

extern const TSLanguage *tree_sitter_vinyl(void) {
  static const TSLanguage language = {
    .version = LANGUAGE_VERSION,
    .symbol_count = SYMBOL_COUNT,
    .alias_count = ALIAS_COUNT,
    .token_count = TOKEN_COUNT,
    .external_token_count = EXTERNAL_TOKEN_COUNT,
    .state_count = STATE_COUNT,
    .large_state_count = LARGE_STATE_COUNT,
    .production_id_count = PRODUCTION_ID_COUNT,
    .field_count = FIELD_COUNT,
    .max_alias_sequence_length = MAX_ALIAS_SEQUENCE_LENGTH,
    .parse_table = &ts_parse_table[0][0],
    .small_parse_table = ts_small_parse_table,
    .small_parse_table_map = ts_small_parse_table_map,
    .parse_actions = ts_parse_actions,
    .symbol_names = ts_symbol_names,
    .field_names = ts_field_names,
    .field_map_slices = ts_field_map_slices,
    .field_map_entries = ts_field_map_entries,
    .symbol_metadata = ts_symbol_metadata,
    .public_symbol_map = ts_symbol_map,
    .alias_map = ts_non_terminal_alias_map,
    .alias_sequences = &ts_alias_sequences[0][0],
    .lex_modes = ts_lex_modes,
    .lex_fn = ts_lex,
  };
  return &language;
}
#ifdef __cplusplus
}
#endif
