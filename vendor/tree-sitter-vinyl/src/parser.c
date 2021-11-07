#include <tree_sitter/parser.h>

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 13
#define STATE_COUNT 43
#define LARGE_STATE_COUNT 4
#define SYMBOL_COUNT 38
#define ALIAS_COUNT 0
#define TOKEN_COUNT 22
#define EXTERNAL_TOKEN_COUNT 0
#define FIELD_COUNT 7
#define MAX_ALIAS_SEQUENCE_LENGTH 5
#define PRODUCTION_ID_COUNT 8

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
  anon_sym_RPAREN = 19,
  anon_sym_LBRACE = 20,
  anon_sym_RBRACE = 21,
  sym_source_file = 22,
  sym__statement = 23,
  sym__declaration = 24,
  sym_variable_declaration = 25,
  sym_function_declaration = 26,
  sym__type = 27,
  sym_array_type = 28,
  sym__literal = 29,
  sym_string_literal = 30,
  sym_char_literal = 31,
  sym_bool_literal = 32,
  sym__expression = 33,
  sym_array_creation_expression = 34,
  sym_parameters = 35,
  aux_sym_source_file_repeat1 = 36,
  aux_sym_string_literal_repeat1 = 37,
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
  [aux_sym_source_file_repeat1] = "source_file_repeat1",
  [aux_sym_string_literal_repeat1] = "string_literal_repeat1",
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
  [aux_sym_source_file_repeat1] = aux_sym_source_file_repeat1,
  [aux_sym_string_literal_repeat1] = aux_sym_string_literal_repeat1,
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
  [aux_sym_source_file_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_string_literal_repeat1] = {
    .visible = false,
    .named = false,
  },
};

enum {
  field_expression = 1,
  field_identifier = 2,
  field_name = 3,
  field_parameters = 4,
  field_return_type = 5,
  field_size = 6,
  field_type = 7,
};

static const char * const ts_field_names[] = {
  [0] = NULL,
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
  [3] = {.index = 3, .length = 3},
  [4] = {.index = 6, .length = 1},
  [5] = {.index = 7, .length = 2},
  [6] = {.index = 9, .length = 3},
  [7] = {.index = 12, .length = 2},
};

static const TSFieldMapEntry ts_field_map_entries[] = {
  [0] =
    {field_type, 1},
  [1] =
    {field_name, 1},
    {field_type, 0},
  [3] =
    {field_identifier, 1},
    {field_parameters, 2},
    {field_return_type, 0},
  [6] =
    {field_type, 0},
  [7] =
    {field_size, 2},
    {field_type, 0},
  [9] =
    {field_expression, 3},
    {field_name, 1},
    {field_type, 0},
  [12] =
    {field_name, 2},
    {field_type, 1},
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
      if (lookahead == ')') ADVANCE(61);
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
      if (lookahead == '{') ADVANCE(62);
      if (lookahead == '}') ADVANCE(63);
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
      ACCEPT_TOKEN(anon_sym_RPAREN);
      END_STATE();
    case 62:
      ACCEPT_TOKEN(anon_sym_LBRACE);
      END_STATE();
    case 63:
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
  [19] = {.lex_state = 1},
  [20] = {.lex_state = 0},
  [21] = {.lex_state = 1},
  [22] = {.lex_state = 0},
  [23] = {.lex_state = 1},
  [24] = {.lex_state = 9},
  [25] = {.lex_state = 9},
  [26] = {.lex_state = 0},
  [27] = {.lex_state = 0},
  [28] = {.lex_state = 0},
  [29] = {.lex_state = 0},
  [30] = {.lex_state = 9},
  [31] = {.lex_state = 9},
  [32] = {.lex_state = 9},
  [33] = {.lex_state = 0},
  [34] = {.lex_state = 0},
  [35] = {.lex_state = 0},
  [36] = {.lex_state = 0},
  [37] = {.lex_state = 0},
  [38] = {.lex_state = 40},
  [39] = {.lex_state = 9},
  [40] = {.lex_state = 0},
  [41] = {.lex_state = 0},
  [42] = {.lex_state = 0},
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
    [anon_sym_RPAREN] = ACTIONS(1),
    [anon_sym_LBRACE] = ACTIONS(1),
    [anon_sym_RBRACE] = ACTIONS(1),
  },
  [1] = {
    [sym_source_file] = STATE(37),
    [sym__statement] = STATE(3),
    [sym__declaration] = STATE(3),
    [sym_variable_declaration] = STATE(3),
    [sym_function_declaration] = STATE(3),
    [sym__type] = STATE(30),
    [sym_array_type] = STATE(30),
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
    [sym__type] = STATE(30),
    [sym_array_type] = STATE(30),
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
  },
  [3] = {
    [sym__statement] = STATE(2),
    [sym__declaration] = STATE(2),
    [sym_variable_declaration] = STATE(2),
    [sym_function_declaration] = STATE(2),
    [sym__type] = STATE(30),
    [sym_array_type] = STATE(30),
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
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 3,
    ACTIONS(60), 1,
      anon_sym_LBRACK,
    ACTIONS(62), 1,
      sym_integer_literal,
    ACTIONS(58), 11,
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
  [20] = 7,
    ACTIONS(15), 1,
      anon_sym_DQUOTE,
    ACTIONS(17), 1,
      anon_sym_SQUOTE,
    ACTIONS(64), 1,
      sym_integer_literal,
    ACTIONS(66), 1,
      sym_floating_point_literal,
    ACTIONS(68), 1,
      anon_sym_new,
    ACTIONS(19), 2,
      anon_sym_true,
      anon_sym_false,
    STATE(34), 6,
      sym__literal,
      sym_string_literal,
      sym_char_literal,
      sym_bool_literal,
      sym__expression,
      sym_array_creation_expression,
  [48] = 2,
    ACTIONS(72), 1,
      sym_integer_literal,
    ACTIONS(70), 11,
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
  [65] = 2,
    ACTIONS(76), 1,
      sym_integer_literal,
    ACTIONS(74), 11,
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
  [82] = 2,
    ACTIONS(80), 1,
      sym_integer_literal,
    ACTIONS(78), 11,
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
  [99] = 2,
    ACTIONS(84), 1,
      sym_integer_literal,
    ACTIONS(82), 11,
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
  [116] = 2,
    ACTIONS(88), 1,
      sym_integer_literal,
    ACTIONS(86), 11,
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
  [133] = 2,
    ACTIONS(92), 1,
      sym_integer_literal,
    ACTIONS(90), 11,
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
  [150] = 2,
    ACTIONS(96), 1,
      sym_integer_literal,
    ACTIONS(94), 10,
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
  [166] = 2,
    ACTIONS(100), 1,
      sym_integer_literal,
    ACTIONS(98), 10,
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
  [182] = 2,
    ACTIONS(104), 1,
      sym_integer_literal,
    ACTIONS(102), 10,
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
  [198] = 2,
    ACTIONS(108), 1,
      sym_integer_literal,
    ACTIONS(106), 10,
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
  [214] = 2,
    ACTIONS(112), 1,
      sym_integer_literal,
    ACTIONS(110), 10,
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
  [230] = 4,
    ACTIONS(114), 1,
      anon_sym_EQ,
    ACTIONS(116), 1,
      anon_sym_SEMI,
    ACTIONS(118), 1,
      anon_sym_LPAREN,
    STATE(14), 1,
      sym_parameters,
  [243] = 3,
    ACTIONS(120), 1,
      sym_primitive_type,
    ACTIONS(122), 1,
      anon_sym_RPAREN,
    STATE(31), 2,
      sym__type,
      sym_array_type,
  [254] = 3,
    ACTIONS(124), 1,
      anon_sym_DQUOTE,
    ACTIONS(126), 1,
      aux_sym_string_literal_token1,
    STATE(23), 1,
      aux_sym_string_literal_repeat1,
  [264] = 3,
    ACTIONS(128), 1,
      sym_primitive_type,
    STATE(4), 1,
      sym_array_type,
    STATE(41), 1,
      sym__type,
  [274] = 3,
    ACTIONS(130), 1,
      anon_sym_DQUOTE,
    ACTIONS(132), 1,
      aux_sym_string_literal_token1,
    STATE(19), 1,
      aux_sym_string_literal_repeat1,
  [284] = 3,
    ACTIONS(134), 1,
      sym_primitive_type,
    STATE(4), 1,
      sym_array_type,
    STATE(42), 1,
      sym__type,
  [294] = 3,
    ACTIONS(136), 1,
      anon_sym_DQUOTE,
    ACTIONS(138), 1,
      aux_sym_string_literal_token1,
    STATE(23), 1,
      aux_sym_string_literal_repeat1,
  [304] = 1,
    ACTIONS(74), 3,
      anon_sym_SEMI,
      anon_sym_LBRACK,
      sym_identifier,
  [310] = 1,
    ACTIONS(70), 3,
      anon_sym_SEMI,
      anon_sym_LBRACK,
      sym_identifier,
  [316] = 2,
    ACTIONS(118), 1,
      anon_sym_LPAREN,
    STATE(14), 1,
      sym_parameters,
  [323] = 2,
    ACTIONS(141), 1,
      anon_sym_RBRACK,
    ACTIONS(143), 1,
      sym_integer_literal,
  [330] = 2,
    ACTIONS(145), 1,
      anon_sym_RBRACK,
    ACTIONS(147), 1,
      sym_integer_literal,
  [337] = 2,
    ACTIONS(114), 1,
      anon_sym_EQ,
    ACTIONS(116), 1,
      anon_sym_SEMI,
  [344] = 2,
    ACTIONS(149), 1,
      anon_sym_LBRACK,
    ACTIONS(151), 1,
      sym_identifier,
  [351] = 2,
    ACTIONS(149), 1,
      anon_sym_LBRACK,
    ACTIONS(153), 1,
      sym_identifier,
  [358] = 1,
    ACTIONS(155), 1,
      sym_identifier,
  [362] = 1,
    ACTIONS(157), 1,
      anon_sym_RBRACK,
  [366] = 1,
    ACTIONS(159), 1,
      anon_sym_SEMI,
  [370] = 1,
    ACTIONS(161), 1,
      anon_sym_RPAREN,
  [374] = 1,
    ACTIONS(163), 1,
      anon_sym_SQUOTE,
  [378] = 1,
    ACTIONS(165), 1,
      ts_builtin_sym_end,
  [382] = 1,
    ACTIONS(167), 1,
      aux_sym_char_literal_token1,
  [386] = 1,
    ACTIONS(169), 1,
      sym_identifier,
  [390] = 1,
    ACTIONS(171), 1,
      anon_sym_RBRACK,
  [394] = 1,
    ACTIONS(149), 1,
      anon_sym_LBRACK,
  [398] = 1,
    ACTIONS(173), 1,
      anon_sym_LBRACK,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(4)] = 0,
  [SMALL_STATE(5)] = 20,
  [SMALL_STATE(6)] = 48,
  [SMALL_STATE(7)] = 65,
  [SMALL_STATE(8)] = 82,
  [SMALL_STATE(9)] = 99,
  [SMALL_STATE(10)] = 116,
  [SMALL_STATE(11)] = 133,
  [SMALL_STATE(12)] = 150,
  [SMALL_STATE(13)] = 166,
  [SMALL_STATE(14)] = 182,
  [SMALL_STATE(15)] = 198,
  [SMALL_STATE(16)] = 214,
  [SMALL_STATE(17)] = 230,
  [SMALL_STATE(18)] = 243,
  [SMALL_STATE(19)] = 254,
  [SMALL_STATE(20)] = 264,
  [SMALL_STATE(21)] = 274,
  [SMALL_STATE(22)] = 284,
  [SMALL_STATE(23)] = 294,
  [SMALL_STATE(24)] = 304,
  [SMALL_STATE(25)] = 310,
  [SMALL_STATE(26)] = 316,
  [SMALL_STATE(27)] = 323,
  [SMALL_STATE(28)] = 330,
  [SMALL_STATE(29)] = 337,
  [SMALL_STATE(30)] = 344,
  [SMALL_STATE(31)] = 351,
  [SMALL_STATE(32)] = 358,
  [SMALL_STATE(33)] = 362,
  [SMALL_STATE(34)] = 366,
  [SMALL_STATE(35)] = 370,
  [SMALL_STATE(36)] = 374,
  [SMALL_STATE(37)] = 378,
  [SMALL_STATE(38)] = 382,
  [SMALL_STATE(39)] = 386,
  [SMALL_STATE(40)] = 390,
  [SMALL_STATE(41)] = 394,
  [SMALL_STATE(42)] = 398,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0),
  [5] = {.entry = {.count = 1, .reusable = true}}, SHIFT(30),
  [7] = {.entry = {.count = 1, .reusable = true}}, SHIFT(32),
  [9] = {.entry = {.count = 1, .reusable = true}}, SHIFT(39),
  [11] = {.entry = {.count = 1, .reusable = false}}, SHIFT(3),
  [13] = {.entry = {.count = 1, .reusable = true}}, SHIFT(3),
  [15] = {.entry = {.count = 1, .reusable = true}}, SHIFT(21),
  [17] = {.entry = {.count = 1, .reusable = true}}, SHIFT(38),
  [19] = {.entry = {.count = 1, .reusable = true}}, SHIFT(8),
  [21] = {.entry = {.count = 1, .reusable = true}}, SHIFT(22),
  [23] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2),
  [25] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(30),
  [28] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(32),
  [31] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(39),
  [34] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(2),
  [37] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(2),
  [40] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(21),
  [43] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(38),
  [46] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(8),
  [49] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(22),
  [52] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1),
  [54] = {.entry = {.count = 1, .reusable = false}}, SHIFT(2),
  [56] = {.entry = {.count = 1, .reusable = true}}, SHIFT(2),
  [58] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_array_creation_expression, 2, .production_id = 1),
  [60] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__type, 1),
  [62] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_array_creation_expression, 2, .production_id = 1),
  [64] = {.entry = {.count = 1, .reusable = false}}, SHIFT(34),
  [66] = {.entry = {.count = 1, .reusable = true}}, SHIFT(34),
  [68] = {.entry = {.count = 1, .reusable = true}}, SHIFT(20),
  [70] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_array_type, 4, .production_id = 5),
  [72] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_array_type, 4, .production_id = 5),
  [74] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_array_type, 3, .production_id = 4),
  [76] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_array_type, 3, .production_id = 4),
  [78] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_bool_literal, 1),
  [80] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_bool_literal, 1),
  [82] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_char_literal, 3),
  [84] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_char_literal, 3),
  [86] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_string_literal, 3),
  [88] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_string_literal, 3),
  [90] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_string_literal, 2),
  [92] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_string_literal, 2),
  [94] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_variable_declaration, 3, .production_id = 2),
  [96] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_variable_declaration, 3, .production_id = 2),
  [98] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_variable_declaration, 5, .production_id = 6),
  [100] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_variable_declaration, 5, .production_id = 6),
  [102] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_function_declaration, 3, .production_id = 3),
  [104] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_function_declaration, 3, .production_id = 3),
  [106] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_parameters, 4, .production_id = 7),
  [108] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_parameters, 4, .production_id = 7),
  [110] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_parameters, 2),
  [112] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_parameters, 2),
  [114] = {.entry = {.count = 1, .reusable = true}}, SHIFT(5),
  [116] = {.entry = {.count = 1, .reusable = true}}, SHIFT(12),
  [118] = {.entry = {.count = 1, .reusable = true}}, SHIFT(18),
  [120] = {.entry = {.count = 1, .reusable = true}}, SHIFT(31),
  [122] = {.entry = {.count = 1, .reusable = true}}, SHIFT(16),
  [124] = {.entry = {.count = 1, .reusable = false}}, SHIFT(10),
  [126] = {.entry = {.count = 1, .reusable = true}}, SHIFT(23),
  [128] = {.entry = {.count = 1, .reusable = true}}, SHIFT(41),
  [130] = {.entry = {.count = 1, .reusable = false}}, SHIFT(11),
  [132] = {.entry = {.count = 1, .reusable = true}}, SHIFT(19),
  [134] = {.entry = {.count = 1, .reusable = true}}, SHIFT(42),
  [136] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym_string_literal_repeat1, 2),
  [138] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_string_literal_repeat1, 2), SHIFT_REPEAT(23),
  [141] = {.entry = {.count = 1, .reusable = true}}, SHIFT(7),
  [143] = {.entry = {.count = 1, .reusable = true}}, SHIFT(40),
  [145] = {.entry = {.count = 1, .reusable = true}}, SHIFT(24),
  [147] = {.entry = {.count = 1, .reusable = true}}, SHIFT(33),
  [149] = {.entry = {.count = 1, .reusable = true}}, SHIFT(28),
  [151] = {.entry = {.count = 1, .reusable = true}}, SHIFT(17),
  [153] = {.entry = {.count = 1, .reusable = true}}, SHIFT(35),
  [155] = {.entry = {.count = 1, .reusable = true}}, SHIFT(29),
  [157] = {.entry = {.count = 1, .reusable = true}}, SHIFT(25),
  [159] = {.entry = {.count = 1, .reusable = true}}, SHIFT(13),
  [161] = {.entry = {.count = 1, .reusable = true}}, SHIFT(15),
  [163] = {.entry = {.count = 1, .reusable = true}}, SHIFT(9),
  [165] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
  [167] = {.entry = {.count = 1, .reusable = true}}, SHIFT(36),
  [169] = {.entry = {.count = 1, .reusable = true}}, SHIFT(26),
  [171] = {.entry = {.count = 1, .reusable = true}}, SHIFT(6),
  [173] = {.entry = {.count = 1, .reusable = true}}, SHIFT(27),
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
