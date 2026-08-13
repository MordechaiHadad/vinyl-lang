#!/usr/bin/env python3
"""Drift guard: treesitter and TextMate lexical token sets must stay in step.

Extracts the overlapping lexical surface from the canonical treesitter sources
(editor/queries/vinyl/highlights.scm, grammar/grammar.js) and compares it
against the canonical TextMate grammar (editor/syntax/vinyl.tmLanguage.json).

Fails on any divergence so a keyword or primitive added in one place cannot
silently rot the other.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCM = ROOT / "editor/queries/vinyl/highlights.scm"
GRAMMAR = ROOT / "grammar/grammar.js"
TM = ROOT / "editor/syntax/vinyl.tmLanguage.json"

QUOTED = re.compile(r"(['\"])([^'\"]+)\1")

KEYWORD_CAPTURES = {"keyword", "keyword.modifier", "keyword.import", "keyword.operator"}

# Tokens TextMate legitimately classifies as operators instead of
# punctuation.brackets / punctuation.separator, so they never appear in its
# punctuation sections. New tokens added to the treesitter punctuation lists
# that are NOT in these exemptions must also appear in TextMate.
BRACKET_OPERATOR_TOKENS = {"<", ">"}
DELIMITER_OPERATOR_TOKENS = {"=", "::", "=>"}


def scm_capture_tokens(text, captures):
    tokens = set()
    for match in re.finditer(r"\[(.*?)\]\s*@([\w.]+)", text, re.DOTALL):
        if match.group(2) in captures:
            tokens.update(token for _, token in QUOTED.findall(match.group(1)))
    for match in re.finditer(r"(['\"])([^'\"]+)\1\s+@([\w.]+)", text):
        if match.group(3) in captures:
            tokens.add(match.group(2))
    for match in re.finditer(r"#any-of\?\s+@([\w.]+)\s+([^()]+)\)", text):
        if match.group(1) in captures:
            tokens.update(token for _, token in QUOTED.findall(match.group(2)))
    return tokens


def grammar_tokens(text, rule):
    block = re.search(re.escape(rule) + r"\s*:\s*\$ => choice\(([^()]*)\)", text, re.DOTALL)
    if not block:
        raise ValueError(f"rule {rule} not found in grammar.js")
    return {token for _, token in QUOTED.findall(block.group(1))}


def tm_pattern(tm, section_name, rule_name=None):
    for rule in tm["repository"][section_name]["patterns"]:
        if (rule_name is None or rule.get("name") == rule_name) and "match" in rule:
            return rule["match"]
    raise ValueError(f"match pattern {rule_name} not found in {section_name}")


def tm_words(tm, section_name, rule_name=None):
    pattern = tm_pattern(tm, section_name, rule_name)
    match = re.search(r"\\b\(([^)]+)\)\\b", pattern)
    if not match:
        raise ValueError(f"unexpected word pattern in {section_name}: {pattern}")
    return set(match.group(1).split("|"))


def tm_char_class(tm, section_name, rule_name):
    pattern = tm_pattern(tm, section_name, rule_name)
    match = re.search(r"\[(.*)\]", pattern)
    if not match:
        raise ValueError(f"unexpected char class in {rule_name}: {pattern}")
    chars = set()
    content = match.group(1)
    index = 0
    while index < len(content):
        if content[index] == "\\":
            chars.add(content[index + 1])
            index += 2
        else:
            chars.add(content[index])
            index += 1
    return chars


def check(label, expected, actual):
    missing = expected - actual
    extra = actual - expected
    if not missing and not extra:
        print(f"ok: {label}")
        return True
    if missing:
        print(f"MISMATCH {label}: in source but not TextMate: {sorted(missing)}")
    if extra:
        print(f"MISMATCH {label}: in TextMate but not source: {sorted(extra)}")
    return False


def main():
    scm = SCM.read_text()
    grammar = GRAMMAR.read_text()
    tm = json.loads(TM.read_text())

    checks = [
        (
            "keywords",
            scm_capture_tokens(scm, KEYWORD_CAPTURES),
            tm_words(tm, "keywords"),
        ),
        (
            "primitive types",
            grammar_tokens(grammar, "primitive_type"),
            tm_words(tm, "primitive-types"),
        ),
        (
            "booleans",
            grammar_tokens(grammar, "bool_literal"),
            tm_words(tm, "constants", "constant.language.boolean.vinyl"),
        ),
        (
            "brackets",
            scm_capture_tokens(scm, {"punctuation.bracket"}) - BRACKET_OPERATOR_TOKENS,
            tm_char_class(tm, "punctuation", "punctuation.brackets.vinyl"),
            # note: "<" ">" are operators in TextMate, not brackets
        ),
        (
            "delimiters",
            scm_capture_tokens(scm, {"punctuation.delimiter"}) - DELIMITER_OPERATOR_TOKENS,
            tm_char_class(tm, "punctuation", "punctuation.separator.vinyl"),
            # note: "=" "::" "=>" are operators in TextMate, not separators
        ),
    ]

    failures = 0
    for check_args in checks:
        if not check(*check_args[:3]):
            failures += 1

    if failures:
        sys.exit(f"{failures} lexical drift(s) between treesitter and TextMate")
    print("treesitter/TextMate lexical surface is in sync")


if __name__ == "__main__":
    main()