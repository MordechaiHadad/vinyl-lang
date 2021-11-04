#include <tree_sitter/parser.h>

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 13
#define STATE_COUNT 27
#define LARGE_STATE_COUNT 2
#define SYMBOL_COUNT 31
#define ALIAS_COUNT 0
#define TOKEN_COUNT 18
#define EXTERNAL_TOKEN_COUNT 0
#define FIELD_COUNT 4
#define MAX_ALIAS_SEQUENCE_LENGTH 5
#define PRODUCTION_ID_COUNT 6

enum {
  sym_primitive_type = 1,
  sym_implicit_type = 2,
  anon_sym_void = 3,
  anon_sym_LBRACK = 4,
  anon_sym_RBRACK = 5,
  sym_integer_literal = 6,
  sym_floating_point_literal = 7,
  anon_sym_DQUOTE = 8,
  aux_sym_string_literal_token1 = 9,
  anon_sym_SQUOTE = 10,
  aux_sym_char_literal_token1 = 11,
  anon_sym_true = 12,
  anon_sym_false = 13,
  sym_identifier = 14,
  anon_sym_EQ = 15,
  anon_sym_SEMI = 16,
  anon_sym_new = 17,
  sym_source_file = 18,
  sym__type = 19,
  sym_array_type = 20,
  sym__literal = 21,
  sym_string_literal = 22,
  sym_char_literal = 23,
  sym_bool_literal = 24,
  sym__declaration = 25,
  sym_variable_declaration = 26,
  sym__expression = 27,
  sym_array_creation_expression = 28,
  aux_sym_source_file_repeat1 = 29,
  aux_sym_string_literal_repeat1 = 30,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [sym_primitive_type] = "primitive_type",
  [sym_implicit_type] = "implicit_type",
  [anon_sym_void] = "void",
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
  [anon_sym_EQ] = "=",
  [anon_sym_SEMI] = ";",
  [anon_sym_new] = "new",
  [sym_source_file] = "source_file",
  [sym__type] = "_type",
  [sym_array_type] = "array_type",
  [sym__literal] = "_literal",
  [sym_string_literal] = "string_literal",
  [sym_char_literal] = "char_literal",
  [sym_bool_literal] = "bool_literal",
  [sym__declaration] = "_declaration",
  [sym_variable_declaration] = "variable_declaration",
  [sym__expression] = "_expression",
  [sym_array_creation_expression] = "array_creation_expression",
  [aux_sym_source_file_repeat1] = "source_file_repeat1",
  [aux_sym_string_literal_repeat1] = "string_literal_repeat1",
};

static const TSSymbol ts_symbol_map[] = {
  [ts_builtin_sym_end] = ts_builtin_sym_end,
  [sym_primitive_type] = sym_primitive_type,
  [sym_implicit_type] = sym_implicit_type,
  [anon_sym_void] = anon_sym_void,
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
  [anon_sym_EQ] = anon_sym_EQ,
  [anon_sym_SEMI] = anon_sym_SEMI,
  [anon_sym_new] = anon_sym_new,
  [sym_source_file] = sym_source_file,
  [sym__type] = sym__type,
  [sym_array_type] = sym_array_type,
  [sym__literal] = sym__literal,
  [sym_string_literal] = sym_string_literal,
  [sym_char_literal] = sym_char_literal,
  [sym_bool_literal] = sym_bool_literal,
  [sym__declaration] = sym__declaration,
  [sym_variable_declaration] = sym_variable_declaration,
  [sym__expression] = sym__expression,
  [sym_array_creation_expression] = sym_array_creation_expression,
  [aux_sym_source_file_repeat1] = aux_sym_source_file_repeat1,
  [aux_sym_string_literal_repeat1] = aux_sym_string_literal_repeat1,
};

static const TSSymbolMetadata ts_symbol_metadata[] = {
  [ts_builtin_sym_end] = {
    .visible = false,
    .named = true,
  },
  [sym_primitive_type] = {
    .visible = true,
    .named = true,
  },
  [sym_implicit_type] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_void] = {
    .visible = true,
    .named = false,
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
  [anon_sym_EQ] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_SEMI] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_new] = {
    .visible = true,
    .named = false,
  },
  [sym_source_file] = {
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
  [sym__declaration] = {
    .visible = false,
    .named = true,
  },
  [sym_variable_declaration] = {
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
  field_name = 2,
  field_size = 3,
  field_type = 4,
};

static const char * const ts_field_names[] = {
  [0] = NULL,
  [field_expression] = "expression",
  [field_name] = "name",
  [field_size] = "size",
  [field_type] = "type",
};

static const TSFieldMapSlice ts_field_map_slices[PRODUCTION_ID_COUNT] = {
  [1] = {.index = 0, .length = 1},
  [2] = {.index = 1, .length = 2},
  [3] = {.index = 3, .length = 2},
  [4] = {.index = 5, .length = 1},
  [5] = {.index = 6, .length = 3},
};

static const TSFieldMapEntry ts_field_map_entries[] = {
  [0] =
    {field_type, 0},
  [1] =
    {field_name, 1},
    {field_type, 0},
  [3] =
    {field_size, 2},
    {field_type, 0},
  [5] =
    {field_type, 1},
  [6] =
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
      if (lookahead == '"') ADVANCE(49);
      if (lookahead == '\'') ADVANCE(52);
      if (lookahead == ';') ADVANCE(58);
      if (lookahead == '=') ADVANCE(57);
      if (lookahead == '[') ADVANCE(45);
      if (lookahead == ']') ADVANCE(46);
      if (lookahead == 'b') ADVANCE(27);
      if (lookahead == 'c') ADVANCE(19);
      if (lookahead == 'f') ADVANCE(10);
      if (lookahead == 'i') ADVANCE(26);
      if (lookahead == 'n') ADVANCE(15);
      if (lookahead == 's') ADVANCE(37);
      if (lookahead == 't') ADVANCE(30);
      if (lookahead == 'u') ADVANCE(20);
      if (lookahead == 'v') ADVANCE(12);
      if (lookahead == '\t' ||
          lookahead == '\n' ||
          lookahead == '\r' ||
          lookahead == ' ') SKIP(0)
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(47);
      END_STATE();
    case 1:
      if (lookahead == '\n') SKIP(1)
      if (lookahead == '"') ADVANCE(49);
      if (lookahead == '\t' ||
          lookahead == '\r' ||
          lookahead == ' ') ADVANCE(50);
      if (lookahead != 0 &&
          lookahead != '\\') ADVANCE(51);
      END_STATE();
    case 2:
      if (lookahead == '1') ADVANCE(6);
      if (lookahead == '3') ADVANCE(4);
      if (lookahead == '6') ADVANCE(7);
      if (lookahead == '8') ADVANCE(42);
      END_STATE();
    case 3:
      if (lookahead == '1') ADVANCE(5);
      if (lookahead == '3') ADVANCE(4);
      if (lookahead == '6') ADVANCE(7);
      END_STATE();
    case 4:
      if (lookahead == '2') ADVANCE(42);
      END_STATE();
    case 5:
      if (lookahead == '2') ADVANCE(8);
      END_STATE();
    case 6:
      if (lookahead == '2') ADVANCE(8);
      if (lookahead == '6') ADVANCE(42);
      END_STATE();
    case 7:
      if (lookahead == '4') ADVANCE(42);
      END_STATE();
    case 8:
      if (lookahead == '8') ADVANCE(42);
      END_STATE();
    case 9:
      if (lookahead == ';') ADVANCE(58);
      if (lookahead == '[') ADVANCE(45);
      if (lookahead == '\t' ||
          lookahead == '\n' ||
          lookahead == '\r' ||
          lookahead == ' ') SKIP(9)
      if (('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          lookahead == 181 ||
          (913 <= lookahead && lookahead <= 937) ||
          (945 <= lookahead && lookahead <= 969)) ADVANCE(56);
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
      if (lookahead == 'd') ADVANCE(44);
      END_STATE();
    case 15:
      if (lookahead == 'e') ADVANCE(39);
      END_STATE();
    case 16:
      if (lookahead == 'e') ADVANCE(54);
      END_STATE();
    case 17:
      if (lookahead == 'e') ADVANCE(55);
      END_STATE();
    case 18:
      if (lookahead == 'g') ADVANCE(42);
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
      if (lookahead == 'l') ADVANCE(42);
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
      if (lookahead == 'r') ADVANCE(43);
      END_STATE();
    case 32:
      if (lookahead == 'r') ADVANCE(42);
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
          lookahead != '\\') ADVANCE(53);
      END_STATE();
    case 41:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 42:
      ACCEPT_TOKEN(sym_primitive_type);
      END_STATE();
    case 43:
      ACCEPT_TOKEN(sym_implicit_type);
      END_STATE();
    case 44:
      ACCEPT_TOKEN(anon_sym_void);
      END_STATE();
    case 45:
      ACCEPT_TOKEN(anon_sym_LBRACK);
      END_STATE();
    case 46:
      ACCEPT_TOKEN(anon_sym_RBRACK);
      END_STATE();
    case 47:
      ACCEPT_TOKEN(sym_integer_literal);
      if (lookahead == '.') ADVANCE(48);
      if (('0' <= lookahead && lookahead <= '9') ||
          lookahead == '_') ADVANCE(47);
      END_STATE();
    case 48:
      ACCEPT_TOKEN(sym_floating_point_literal);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(48);
      END_STATE();
    case 49:
      ACCEPT_TOKEN(anon_sym_DQUOTE);
      END_STATE();
    case 50:
      ACCEPT_TOKEN(aux_sym_string_literal_token1);
      if (lookahead == '\t' ||
          lookahead == '\r' ||
          lookahead == ' ') ADVANCE(50);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '"' &&
          lookahead != '\\') ADVANCE(51);
      END_STATE();
    case 51:
      ACCEPT_TOKEN(aux_sym_string_literal_token1);
      if (lookahead != 0 &&
          lookahead != '\n' &&
          lookahead != '"' &&
          lookahead != '\\') ADVANCE(51);
      END_STATE();
    case 52:
      ACCEPT_TOKEN(anon_sym_SQUOTE);
      END_STATE();
    case 53:
      ACCEPT_TOKEN(aux_sym_char_literal_token1);
      END_STATE();
    case 54:
      ACCEPT_TOKEN(anon_sym_true);
      END_STATE();
    case 55:
      ACCEPT_TOKEN(anon_sym_false);
      END_STATE();
    case 56:
      ACCEPT_TOKEN(sym_identifier);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z') ||
          lookahead == 181 ||
          (913 <= lookahead && lookahead <= 937) ||
          (945 <= lookahead && lookahead <= 969)) ADVANCE(56);
      END_STATE();
    case 57:
      ACCEPT_TOKEN(anon_sym_EQ);
      END_STATE();
    case 58:
      ACCEPT_TOKEN(anon_sym_SEMI);
      END_STATE();
    case 59:
      ACCEPT_TOKEN(anon_sym_new);
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
  [6] = {.lex_state = 1},
  [7] = {.lex_state = 1},
  [8] = {.lex_state = 0},
  [9] = {.lex_state = 1},
  [10] = {.lex_state = 9},
  [11] = {.lex_state = 0},
  [12] = {.lex_state = 9},
  [13] = {.lex_state = 0},
  [14] = {.lex_state = 0},
  [15] = {.lex_state = 0},
  [16] = {.lex_state = 9},
  [17] = {.lex_state = 40},
  [18] = {.lex_state = 0},
  [19] = {.lex_state = 0},
  [20] = {.lex_state = 0},
  [21] = {.lex_state = 0},
  [22] = {.lex_state = 0},
  [23] = {.lex_state = 0},
  [24] = {.lex_state = 0},
  [25] = {.lex_state = 0},
  [26] = {.lex_state = 0},
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [ts_builtin_sym_end] = ACTIONS(1),
    [sym_primitive_type] = ACTIONS(1),
    [sym_implicit_type] = ACTIONS(1),
    [anon_sym_void] = ACTIONS(1),
    [anon_sym_LBRACK] = ACTIONS(1),
    [anon_sym_RBRACK] = ACTIONS(1),
    [sym_integer_literal] = ACTIONS(1),
    [sym_floating_point_literal] = ACTIONS(1),
    [anon_sym_DQUOTE] = ACTIONS(1),
    [anon_sym_SQUOTE] = ACTIONS(1),
    [anon_sym_true] = ACTIONS(1),
    [anon_sym_false] = ACTIONS(1),
    [anon_sym_EQ] = ACTIONS(1),
    [anon_sym_SEMI] = ACTIONS(1),
    [anon_sym_new] = ACTIONS(1),
  },
  [1] = {
    [sym_source_file] = STATE(19),
    [sym__type] = STATE(16),
    [sym_array_type] = STATE(16),
    [sym__declaration] = STATE(3),
    [sym_variable_declaration] = STATE(3),
    [aux_sym_source_file_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(3),
    [sym_primitive_type] = ACTIONS(5),
    [sym_implicit_type] = ACTIONS(5),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 7,
    ACTIONS(7), 1,
      sym_integer_literal,
    ACTIONS(9), 1,
      sym_floating_point_literal,
    ACTIONS(11), 1,
      anon_sym_DQUOTE,
    ACTIONS(13), 1,
      anon_sym_SQUOTE,
    ACTIONS(17), 1,
      anon_sym_new,
    ACTIONS(15), 2,
      anon_sym_true,
      anon_sym_false,
    STATE(20), 6,
      sym__literal,
      sym_string_literal,
      sym_char_literal,
      sym_bool_literal,
      sym__expression,
      sym_array_creation_expression,
  [28] = 4,
    ACTIONS(19), 1,
      ts_builtin_sym_end,
    ACTIONS(5), 2,
      sym_primitive_type,
      sym_implicit_type,
    STATE(16), 2,
      sym__type,
      sym_array_type,
    STATE(4), 3,
      sym__declaration,
      sym_variable_declaration,
      aux_sym_source_file_repeat1,
  [45] = 4,
    ACTIONS(21), 1,
      ts_builtin_sym_end,
    ACTIONS(23), 2,
      sym_primitive_type,
      sym_implicit_type,
    STATE(16), 2,
      sym__type,
      sym_array_type,
    STATE(4), 3,
      sym__declaration,
      sym_variable_declaration,
      aux_sym_source_file_repeat1,
  [62] = 3,
    STATE(14), 1,
      sym_array_type,
    STATE(24), 1,
      sym__type,
    ACTIONS(26), 2,
      sym_primitive_type,
      sym_implicit_type,
  [73] = 3,
    ACTIONS(28), 1,
      anon_sym_DQUOTE,
    ACTIONS(30), 1,
      aux_sym_string_literal_token1,
    STATE(9), 1,
      aux_sym_string_literal_repeat1,
  [83] = 3,
    ACTIONS(32), 1,
      anon_sym_DQUOTE,
    ACTIONS(34), 1,
      aux_sym_string_literal_token1,
    STATE(7), 1,
      aux_sym_string_literal_repeat1,
  [93] = 1,
    ACTIONS(37), 3,
      ts_builtin_sym_end,
      sym_primitive_type,
      sym_implicit_type,
  [99] = 3,
    ACTIONS(39), 1,
      anon_sym_DQUOTE,
    ACTIONS(41), 1,
      aux_sym_string_literal_token1,
    STATE(7), 1,
      aux_sym_string_literal_repeat1,
  [109] = 1,
    ACTIONS(43), 3,
      anon_sym_LBRACK,
      sym_identifier,
      anon_sym_SEMI,
  [115] = 1,
    ACTIONS(45), 3,
      ts_builtin_sym_end,
      sym_primitive_type,
      sym_implicit_type,
  [121] = 1,
    ACTIONS(47), 3,
      anon_sym_LBRACK,
      sym_identifier,
      anon_sym_SEMI,
  [127] = 2,
    ACTIONS(49), 1,
      anon_sym_EQ,
    ACTIONS(51), 1,
      anon_sym_SEMI,
  [134] = 2,
    ACTIONS(53), 1,
      anon_sym_LBRACK,
    ACTIONS(55), 1,
      anon_sym_SEMI,
  [141] = 2,
    ACTIONS(57), 1,
      anon_sym_RBRACK,
    ACTIONS(59), 1,
      sym_integer_literal,
  [148] = 2,
    ACTIONS(61), 1,
      anon_sym_LBRACK,
    ACTIONS(63), 1,
      sym_identifier,
  [155] = 1,
    ACTIONS(65), 1,
      aux_sym_char_literal_token1,
  [159] = 1,
    ACTIONS(67), 1,
      anon_sym_SEMI,
  [163] = 1,
    ACTIONS(69), 1,
      ts_builtin_sym_end,
  [167] = 1,
    ACTIONS(71), 1,
      anon_sym_SEMI,
  [171] = 1,
    ACTIONS(73), 1,
      anon_sym_SEMI,
  [175] = 1,
    ACTIONS(75), 1,
      anon_sym_RBRACK,
  [179] = 1,
    ACTIONS(77), 1,
      anon_sym_SQUOTE,
  [183] = 1,
    ACTIONS(61), 1,
      anon_sym_LBRACK,
  [187] = 1,
    ACTIONS(79), 1,
      anon_sym_SEMI,
  [191] = 1,
    ACTIONS(81), 1,
      anon_sym_SEMI,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(2)] = 0,
  [SMALL_STATE(3)] = 28,
  [SMALL_STATE(4)] = 45,
  [SMALL_STATE(5)] = 62,
  [SMALL_STATE(6)] = 73,
  [SMALL_STATE(7)] = 83,
  [SMALL_STATE(8)] = 93,
  [SMALL_STATE(9)] = 99,
  [SMALL_STATE(10)] = 109,
  [SMALL_STATE(11)] = 115,
  [SMALL_STATE(12)] = 121,
  [SMALL_STATE(13)] = 127,
  [SMALL_STATE(14)] = 134,
  [SMALL_STATE(15)] = 141,
  [SMALL_STATE(16)] = 148,
  [SMALL_STATE(17)] = 155,
  [SMALL_STATE(18)] = 159,
  [SMALL_STATE(19)] = 163,
  [SMALL_STATE(20)] = 167,
  [SMALL_STATE(21)] = 171,
  [SMALL_STATE(22)] = 175,
  [SMALL_STATE(23)] = 179,
  [SMALL_STATE(24)] = 183,
  [SMALL_STATE(25)] = 187,
  [SMALL_STATE(26)] = 191,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0),
  [5] = {.entry = {.count = 1, .reusable = true}}, SHIFT(16),
  [7] = {.entry = {.count = 1, .reusable = false}}, SHIFT(20),
  [9] = {.entry = {.count = 1, .reusable = true}}, SHIFT(20),
  [11] = {.entry = {.count = 1, .reusable = true}}, SHIFT(6),
  [13] = {.entry = {.count = 1, .reusable = true}}, SHIFT(17),
  [15] = {.entry = {.count = 1, .reusable = true}}, SHIFT(18),
  [17] = {.entry = {.count = 1, .reusable = true}}, SHIFT(5),
  [19] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1),
  [21] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2),
  [23] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(16),
  [26] = {.entry = {.count = 1, .reusable = true}}, SHIFT(24),
  [28] = {.entry = {.count = 1, .reusable = false}}, SHIFT(21),
  [30] = {.entry = {.count = 1, .reusable = true}}, SHIFT(9),
  [32] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym_string_literal_repeat1, 2),
  [34] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_string_literal_repeat1, 2), SHIFT_REPEAT(7),
  [37] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_variable_declaration, 5, .production_id = 5),
  [39] = {.entry = {.count = 1, .reusable = false}}, SHIFT(25),
  [41] = {.entry = {.count = 1, .reusable = true}}, SHIFT(7),
  [43] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_array_type, 3, .production_id = 1),
  [45] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_variable_declaration, 3, .production_id = 2),
  [47] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_array_type, 4, .production_id = 3),
  [49] = {.entry = {.count = 1, .reusable = true}}, SHIFT(2),
  [51] = {.entry = {.count = 1, .reusable = true}}, SHIFT(11),
  [53] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym__type, 1),
  [55] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_array_creation_expression, 2, .production_id = 4),
  [57] = {.entry = {.count = 1, .reusable = true}}, SHIFT(10),
  [59] = {.entry = {.count = 1, .reusable = true}}, SHIFT(22),
  [61] = {.entry = {.count = 1, .reusable = true}}, SHIFT(15),
  [63] = {.entry = {.count = 1, .reusable = true}}, SHIFT(13),
  [65] = {.entry = {.count = 1, .reusable = true}}, SHIFT(23),
  [67] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_bool_literal, 1),
  [69] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
  [71] = {.entry = {.count = 1, .reusable = true}}, SHIFT(8),
  [73] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_string_literal, 2),
  [75] = {.entry = {.count = 1, .reusable = true}}, SHIFT(12),
  [77] = {.entry = {.count = 1, .reusable = true}}, SHIFT(26),
  [79] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_string_literal, 3),
  [81] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_char_literal, 3),
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
