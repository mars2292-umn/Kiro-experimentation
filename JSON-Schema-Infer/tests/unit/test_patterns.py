"""Unit tests for values/patterns.py (tasks 9.1, 10.2): safe regex AST, printer, parser, and matcher."""

from __future__ import annotations

import itertools
import pickle
import re

import pytest
import regress
from hypothesis import given

from jsinfer.core.determinism import canonical_bytes
from jsinfer.io.validator import Validator
from jsinfer.values.patterns import (
    ONE,
    PLUS,
    SYNTAX_CHARACTERS,
    Alternative,
    CharClass,
    Decomposition,
    Pattern,
    Quantifier,
    Token,
    escape,
)
from tests.strategies import safe_patterns

D, L, U, A, N = CharClass.DIGIT, CharClass.LOWER, CharClass.UPPER, CharClass.ALPHA, CharClass.ALNUM


def q(low: int, high: int | None) -> Quantifier:
    return Quantifier(low, high)


def alt(*tokens: Token) -> Alternative:
    return Alternative(tokens)


def pat(*alternatives: Alternative) -> Pattern:
    return Pattern(alternatives)


# ---------------------------------------------------------------------------
# Character classes
# ---------------------------------------------------------------------------


def test_class_sizes_and_membership() -> None:
    assert [c.size for c in (D, L, U, A, N)] == [10, 26, 26, 52, 62]
    for cls in CharClass:
        members = [chr(cp) for cp in range(128) if cls.contains(chr(cp))]
        assert len(members) == cls.size
        assert all(re.fullmatch(cls.value, ch) for ch in members)
    assert not N.contains("é") and not N.contains("_") and not N.contains("٣")


def test_base_of() -> None:
    assert [CharClass.base_of(ch) for ch in "7qQ-é"] == [D, L, U, None, None]


@pytest.mark.parametrize(
    ("a", "b", "overlap", "joined"),
    [
        (D, D, True, D),
        (D, L, False, N),
        (D, U, False, N),
        (L, U, False, A),
        (L, A, True, A),
        (U, A, True, A),
        (D, A, False, N),
        (D, N, True, N),
        (A, N, True, N),
    ],
)
def test_class_lattice(a: CharClass, b: CharClass, overlap: bool, joined: CharClass) -> None:
    assert a.overlaps(b) is overlap and b.overlaps(a) is overlap
    assert a.join(b) is joined and b.join(a) is joined
    # Overlap only by containment, so merging overlapping classes yields the larger one.
    if overlap:
        assert a.issubset(b) or b.issubset(a)


# ---------------------------------------------------------------------------
# Construction and validation
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("args", "error"),
    [
        ((0, 1), ValueError),
        ((0, None), ValueError),
        ((3, 2), ValueError),
        ((True, 1), TypeError),
        ((1, 2.0), TypeError),
    ],
)
def test_quantifier_rejects_bad_counts(args: tuple[object, object], error: type[Exception]) -> None:
    with pytest.raises(error):
        Quantifier(*args)  # type: ignore[arg-type]


def test_quantifier_addition_and_acceptance() -> None:
    assert q(1, 3) + q(2, 2) == q(3, 5)
    assert q(1, 3) + PLUS == q(2, None)
    assert Quantifier.exactly(4) == q(4, 4) and Quantifier.at_least(2) == q(2, None)
    assert [q(2, 4).accepts(k) for k in range(6)] == [False, False, True, True, True, False]
    assert PLUS.accepts(10**9) and not PLUS.accepts(0)


@pytest.mark.parametrize("atom", ["a", "Z", "5", "ab", "", "\ud800", "\udfff", 3])
def test_token_rejects_bad_atoms(atom: object) -> None:
    with pytest.raises((TypeError, ValueError)):
        Token(atom)  # type: ignore[arg-type]


def test_containers_reject_bad_members() -> None:
    with pytest.raises(TypeError):
        Token(D, (1, 1))  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        Alternative(["-"])  # type: ignore[list-item]
    with pytest.raises(TypeError):
        Alternative(Token("-"))  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        Pattern([Token("-")])  # type: ignore[list-item]
    with pytest.raises(ValueError):
        Pattern(())


# ---------------------------------------------------------------------------
# Normal form
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("tokens", "merged"),
    [
        ((Token(D, q(1, 3)), Token(D, q(2, 2))), (Token(D, q(3, 5)),)),
        ((Token(L, q(2, 2)), Token(A, PLUS)), (Token(A, q(3, None)),)),
        ((Token(D), Token(N)), (Token(N, q(2, 2)),)),
        # Merging cascades: [A-Z] and [A-Za-z] merge, and the result now overlaps [a-z].
        ((Token(L), Token(U), Token(A)), (Token(A, q(3, 3)),)),
        ((Token("-"), Token("-", PLUS)), (Token("-", q(2, None)),)),
        # Disjoint neighbours stay apart.
        ((Token(L), Token(U)), (Token(L), Token(U))),
        ((Token(D), Token(L), Token(D)), (Token(D), Token(L), Token(D))),
        ((Token(D), Token("-"), Token(D)), (Token(D), Token("-"), Token(D))),
        ((Token("-"), Token(".")), (Token("-"), Token("."))),
        ((), ()),
    ],
)
def test_adjacent_overlapping_tokens_merge(tokens: tuple[Token, ...], merged: tuple[Token, ...]) -> None:
    alternative = Alternative(tokens)
    assert alternative.tokens == merged
    assert Alternative(alternative.tokens) == alternative  # normal form is a fixed point
    assert all(not a.overlaps(b) for a, b in itertools.pairwise(alternative.tokens))


def test_merge_of_disjoint_tokens_is_an_error() -> None:
    with pytest.raises(ValueError):
        Token(D).merge(Token(L))
    with pytest.raises(ValueError):
        Token(D).merge(Token("-"))


def test_pattern_drops_duplicate_alternatives_keeping_order() -> None:
    a, b = alt(Token(D, PLUS)), alt(Token(L, PLUS))
    # [0-9][0-9] normalizes to [0-9]{2}, so it duplicates the third alternative.
    p = pat(a, b, alt(Token(D), Token(D)), a, alt(Token(D, q(2, 2))))
    assert p.alternatives == (a, b, alt(Token(D, q(2, 2))))


# ---------------------------------------------------------------------------
# Printer
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("quantifier", "text"),
    [(ONE, ""), (q(3, 3), "{3}"), (q(2, 4), "{2,4}"), (PLUS, "+"), (q(2, None), "{2,}")],
)
def test_quantifier_and_class_printing(quantifier: Quantifier, text: str) -> None:
    assert quantifier.source == text
    assert Token(D, quantifier).source == "[0-9]" + text
    assert Token(N, quantifier).source == "[0-9A-Za-z]" + text


def test_only_ecma_syntax_characters_are_escaped() -> None:
    for ch in SYNTAX_CHARACTERS:
        assert escape(ch) == "\\" + ch
        assert Token(ch).source == "\\" + ch
    for ch in "-_#&~ ,:;<>=!@%'\"`\t\n\r\x00\u2028é中😀":
        assert escape(ch) == ch
        assert Token(ch).source == ch


def test_quantified_literals_print_as_groups() -> None:
    assert Token("😀", q(2, 2)).source == "(?:😀){2}"
    assert Token("😀", PLUS).source == "(?:😀)+"
    assert Token("-", q(2, 3)).source == "(?:-){2,3}"
    assert Token(".", q(2, None)).source == "(?:\\.){2,}"
    assert Token("😀").source == "😀"


def test_anchoring() -> None:
    p = pat(alt(Token(U, q(2, 2)), Token("-"), Token(D, q(1, 4))), alt(Token(L, PLUS)))
    assert p.body == "[A-Z]{2}-[0-9]{1,4}|[a-z]+"
    assert p.source == "^(?:[A-Z]{2}-[0-9]{1,4}|[a-z]+)$(?!\\n)"
    assert str(p) == p.source
    assert pat(alt()).source == "^(?:)$(?!\\n)"
    assert pat(alt(), alt(Token(D))).source == "^(?:|[0-9])$(?!\\n)"


def test_printed_pattern_requires_a_whole_string_match_in_every_engine() -> None:
    p = pat(alt(Token(U, q(2, 2)), Token("/"), Token("😀", q(2, 2))), alt())
    validator = Validator()
    cases = {"AB/😀😀": True, "": True, "AB/😀😀\n": False, "\n": False, "xAB/😀😀": False, "AB/😀": False}
    for s, expected in cases.items():
        assert (re.search(p.source, s) is not None) is expected
        assert (regress.Regex(p.source).find(s) is not None) is expected
        assert (regress.Regex(p.source, "u").find(s) is not None) is expected
        assert validator.matches_pattern(p.source, s) is expected
        assert p.matches(s) is expected


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------


_QUANTIFIERS = (ONE, q(3, 3), q(2, 4), PLUS, q(2, None))


def test_parser_inverts_the_printer_for_every_token_shape() -> None:
    atoms: list[CharClass | str] = [*CharClass, "-", "😀", "\n", "é", *SYNTAX_CHARACTERS]
    for atom, quantifier in itertools.product(atoms, _QUANTIFIERS):
        p = pat(alt(Token(atom, quantifier)), alt(), alt(Token("_"), Token(D, PLUS), Token("|")))
        assert Pattern.from_source(p.source) == p
    for p in (
        pat(alt()),
        pat(alt(), alt(Token(D))),
        pat(alt(Token(U, q(2, 2)), Token("-"), Token(D, q(1, 4))), alt(Token(L, PLUS))),
        pat(alt(Token("\\", q(2, 2)), Token("(", PLUS), Token(")"), Token("{"), Token(N, q(10, 12)))),
        pat(alt(Token(A, q(1, 10**30)))),
    ):
        assert Pattern.from_source(p.source) == p
        assert Pattern.from_source(p.source).source == p.source


@given(safe_patterns())
def test_parser_round_trips_generated_patterns(p: Pattern) -> None:
    assert Pattern.from_source(p.source) == p


@pytest.mark.parametrize(
    "body",
    [
        "[a-z][a-z]",  # not in normal form: prints as [a-z]{2}
        "[0-9]{1}",  # ONE prints as nothing
        "[0-9]{1,}",  # prints as +
        "[0-9]{02}",
        "[0-9]{3,3}",
        "[0-9]{0}",
        "[0-9]{3,2}",
        "[0-9]{,3}",
        "[0-9]{a}",
        "[0-9]{3",
        "[0-9]*",
        "[0-8]",
        "a",  # letters and digits belong to the classes
        "\\-",  # needless escape
        "\\d",
        "-\\",
        ".",  # unescaped syntax character
        "(?:-)",  # a group needs a quantifier
        "(?:-",
        "(?:--){2}",
        "(?:[a-z]){2}",
        "--",  # prints as (?:-){2}
        "-|-",  # duplicate alternatives
        "\ud800",
    ],
)
def test_parser_rejects_bodies_the_printer_never_writes(body: str) -> None:
    with pytest.raises(ValueError):
        Pattern.from_source(f"^(?:{body})$(?!\\n)")


@pytest.mark.parametrize("source", ["", "^(?:-)$", "(?:-)$(?!\\n)", "^-$(?!\\n)", "^(?:)$(?!\\n)x", "^(?:$(?!\\n)"])
def test_parser_requires_the_anchors(source: str) -> None:
    with pytest.raises(ValueError):
        Pattern.from_source(source)


def test_parser_rejects_non_strings() -> None:
    with pytest.raises(TypeError):
        Pattern.from_source(b"^(?:)$(?!\\n)")  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Matcher
# ---------------------------------------------------------------------------


def _all_decompositions(alternative: Alternative, s: str) -> list[tuple[int, ...]]:
    """Every split of ``s`` into token runs that the tokens accept (brute force)."""
    found: list[tuple[int, ...]] = []
    tokens = alternative.tokens

    def walk(i: int, pos: int, counts: tuple[int, ...]) -> None:
        if i == len(tokens):
            if pos == len(s):
                found.append(counts)
            return
        token = tokens[i]
        for end in range(pos + 1, len(s) + 1):
            if not token.accepts(s[end - 1]):
                break
            if token.quantifier.accepts(end - pos):
                walk(i + 1, end, (*counts, end - pos))

    walk(0, 0, ())
    return found


def test_decomposition_spans_and_counts() -> None:
    p = pat(alt(Token(U, q(2, 2)), Token("-"), Token(D, q(1, 4))))
    assert p.match("AB-123") == Decomposition(0, ((0, 2), (2, 3), (3, 6)))
    assert p.match("AB-123").counts == (2, 1, 3)  # type: ignore[union-attr]
    for s in ("AB-12345", "AB-", "ABC-1", "Ab-1", "AB-123\n", "AB_1", ""):
        assert p.match(s) is None


def test_first_matching_alternative_is_reported() -> None:
    p = pat(alt(Token(D, PLUS)), alt(Token(N, PLUS)), alt())
    assert p.match("123") == Decomposition(0, ((0, 3),))
    assert p.match("a1") == Decomposition(1, ((0, 2),))
    assert p.match("") == Decomposition(2, ())
    assert p.match("a-1") is None


def test_counts_are_code_points() -> None:
    p = pat(alt(Token("😀", q(2, 3)), Token(L, PLUS)))
    assert p.match("😀😀ab") == Decomposition(0, ((0, 2), (2, 4)))
    assert p.match("😀ab") is None
    assert pat(alt(Token(L, PLUS))).match("é") is None


@pytest.mark.parametrize(
    ("tokens", "strings"),
    [
        ((Token(D, q(1, 3)), Token(D, q(1, 3))), ["1", "12", "123456", "1234567"]),
        ((Token(L, PLUS), Token(A, PLUS), Token(D, q(1, 2))), ["aB1", "abcD12", "ab", "a123"]),
        ((Token(D, PLUS), Token(L, PLUS), Token(D, PLUS)), ["1a1", "12ab345", "1a", "a1"]),
        ((Token("-", q(1, 2)), Token("-", q(1, 2)), Token(N)), ["--x", "----9", "-x", "-----x"]),
        ((Token(U), Token(".", PLUS), Token(N, q(2, None))), ["A.ab", "A...a1b2", "A.a", "Aab"]),
    ],
)
def test_decomposition_is_unique_and_agrees_with_full_match(tokens: tuple[Token, ...], strings: list[str]) -> None:
    alternative = alt(*tokens)
    p = pat(alternative)
    for s in strings:
        decompositions = _all_decompositions(alternative, s)
        assert len(decompositions) <= 1, (s, decompositions)
        found = p.match(s)
        assert (found is not None) is (re.fullmatch(p.body, s) is not None)
        assert (found.counts if found is not None else None) == (decompositions[0] if decompositions else None)


# ---------------------------------------------------------------------------
# Value semantics
# ---------------------------------------------------------------------------


def test_patterns_pickle_and_hash_by_content() -> None:
    p = pat(alt(Token(U, q(2, 2)), Token("-"), Token(D, PLUS)), alt(Token("😀", q(2, 2))))
    clone = pickle.loads(pickle.dumps(p))
    assert clone == p and clone.source == p.source
    assert canonical_bytes(p) == canonical_bytes(p.source)
    same = pat(alt(Token(U), Token(U), Token("-"), Token(D, PLUS)), alt(Token("😀"), Token("😀")))
    assert same == p and {same: 1}[p] == 1
