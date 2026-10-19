"""Property 38: Field-name normalization (task 21.2).

For any token sequence rendered as camelCase, PascalCase, snake_case,
kebab-case, or SCREAMING_SNAKE_CASE, every rendering normalizes to the same
token tuple, which is the casefolded tokens themselves, and normalization is
idempotent: normalizing a rendering of the normalized tokens, or any single
normalized token, gives the tokens back.

Inputs
    Tokens are two to six characters: a lowercase letter, more lowercase
    letters, then optional trailing digits, from ASCII and a few non-ASCII
    letters (``é``, ``ü``, ``ß``, whose uppercase ``SS`` casefolds back to
    ``ss``). So a rendering can capitalize a token's first character, no
    token holds a separator or a case transition of its own, a capitalized
    token ends in a lowercase letter or digit (a one-letter token would
    leave ``AB`` in PascalCase, which no rule can split), and no digit is
    followed by a letter (``A0A`` in SCREAMING_SNAKE_CASE splits at the
    digit, as 24.2 requires). Sequences have one to five tokens. Besides the
    five styles, mixed renderings join tokens with a separator drawn per gap
    and capitalize per token at random.

**Validates: Requirements 24.2**
"""

# Feature: json-schema-inference, Property 38: Field-name normalization

from __future__ import annotations

from hypothesis import given
from hypothesis import strategies as st

from jsinfer.corpus.normalize import normalize_name

_LETTERS = "abcdefghijklmnopqrstuvwxyzéüß"
_DIGITS = "0123456789"

tokens = st.builds(
    lambda first, letters, digits: first + letters + digits,
    st.sampled_from(_LETTERS),
    st.text(_LETTERS, max_size=3),
    st.text(_DIGITS, max_size=2),
).filter(lambda token: len(token) >= 2)
token_sequences = st.lists(tokens, min_size=1, max_size=5)


def _capitalize(token: str) -> str:
    return token[0].upper() + token[1:]


def camel_case(seq: list[str]) -> str:
    return seq[0] + "".join(_capitalize(t) for t in seq[1:])


def pascal_case(seq: list[str]) -> str:
    return "".join(_capitalize(t) for t in seq)


def snake_case(seq: list[str]) -> str:
    return "_".join(seq)


def kebab_case(seq: list[str]) -> str:
    return "-".join(seq)


def screaming_snake_case(seq: list[str]) -> str:
    return "_".join(t.upper() for t in seq)


STYLES = (camel_case, pascal_case, snake_case, kebab_case, screaming_snake_case)


@st.composite
def mixed_renderings(draw: st.DrawFn, seq: list[str]) -> str:
    """Tokens joined by a per-gap separator (``_``, ``-``, or a case change) and capitalized at random."""
    out = ""
    for index, token in enumerate(seq):
        piece = _capitalize(token) if draw(st.booleans()) else token
        if index:
            separator = draw(st.sampled_from(["_", "-", "__", "-_", ""]))
            if separator == "" and not (out[-1].islower() or out[-1].isdigit()):
                separator = "_"  # "SS" + "Ab" has no lower-to-upper transition
            if separator == "" and not piece[0].isupper():
                piece = _capitalize(piece)  # a case change is the only separator in this gap
            out += separator
        out += piece
    return out


@given(seq=token_sequences, data=st.data())
def test_every_rendering_normalizes_to_the_casefolded_tokens(seq: list[str], data: st.DataObject) -> None:
    expected = tuple(t.casefold() for t in seq)
    for style in STYLES:
        assert normalize_name(style(seq)) == expected, style.__name__
    assert normalize_name(data.draw(mixed_renderings(seq), label="mixed")) == expected


@given(seq=token_sequences)
def test_normalization_is_idempotent(seq: list[str]) -> None:
    normalized = normalize_name(snake_case(seq))
    for style in STYLES:
        assert normalize_name(style(list(normalized))) == normalized, style.__name__
    for token in normalized:
        assert normalize_name(token) == (token,)
