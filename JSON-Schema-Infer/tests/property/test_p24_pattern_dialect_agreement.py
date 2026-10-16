"""Property 24: Pattern anchoring and dialect agreement (task 9.2).

For any pattern from the safe regex AST and any string (including non-ASCII,
astral, LF, CR, and U+2028 characters, and trailing line terminators), the
printed pattern gets the same verdict from Python ``re.search``, from the
ECMA-262 engine ``regress`` (without and with the ``u`` flag), and from the
Validator (jsonschema-rs, as the ``pattern`` keyword). That verdict is true
exactly when the unanchored body matches the entire string, and the in-house
matcher, which the MDL_Scorer's data bits rest on, agrees with it.

Strings come from :func:`tests.strategies.pattern_strings`: mostly strings
built from an alternative (so matches are common), then maybe edited with
line terminators and adversarial characters, and otherwise arbitrary text.

**Validates: Requirements 14.4, 14.5**
"""

# Feature: json-schema-inference, Property 24: Pattern anchoring and dialect agreement

from __future__ import annotations

import re

import regress
from hypothesis import event, given

from jsinfer.io.validator import Validator
from jsinfer.values.patterns import Pattern
from tests.strategies import LINE_TERMINATORS, patterns_with_strings

# One Validator for the module, so each printed pattern compiles once.
VALIDATOR = Validator()


@given(patterns_with_strings())
def test_every_engine_matches_exactly_the_strings_the_body_fully_matches(case: tuple[Pattern, list[str]]) -> None:
    pattern, strings = case
    source = pattern.source
    python = re.compile(source)
    body = re.compile(pattern.body)
    ecma = regress.Regex(source)
    ecma_unicode = regress.Regex(source, "u")
    for s in strings:
        expected = body.fullmatch(s) is not None
        verdicts = {
            "Python re.search": python.search(s) is not None,
            "regress": ecma.find(s) is not None,
            "regress (u flag)": ecma_unicode.find(s) is not None,
            "Validator": VALIDATOR.matches_pattern(source, s),
            "Pattern.matches": pattern.matches(s),
        }
        disagreeing = {engine: verdict for engine, verdict in verdicts.items() if verdict is not expected}
        assert not disagreeing, f"pattern {source!r} on {s!r}: full match of the body is {expected}, but {disagreeing}"

        event("match" if expected else "no match")
        if s.endswith(LINE_TERMINATORS):
            event(f"trailing line terminator, {'match' if expected else 'no match'}")
        if any(ord(ch) > 0xFFFF for ch in s):
            event("astral character in string")
    if any(ord(ch) > 0xFFFF for ch in source):
        event("astral character in pattern")
