"""Primitive MDL codes: universal integers, KT categorical, strings, numbers, bounds (Requirement 19.1).

Every function returns a code length in bits as a finite, non-negative
``float``. The MDL_Scorer builds model bits and data bits from these codes
(see "Key Algorithms > MDL encoding" in the design).

Integers
    ``LN(n)``, n ≥ 1, is Rissanen's universal code for the positive integers:
    ``log2 c0 + log2 n + log2 log2 n + …`` over the positive terms, with
    ``c0 = 2.865064`` (:data:`RISSANEN_C0`). ``LN0(n) = LN(n + 1)`` codes
    n ≥ 0, and ``LZ(z) = 1 + LN0(|z|)`` codes any integer with a sign bit.
    ``UNI(k) = log2 k`` picks one of k equally likely options (0 when
    k ≤ 1), and ``SUB(r, p) = LN0(r) + log2 C(p, r)`` codes a subset of size
    r out of p. Integer arguments may be arbitrarily large Python ints or
    numpy integers; ``bool`` is rejected, as are floats, even integral ones.
    ``math.log2`` and ``math.log`` take ints of any size without converting
    them to float, so huge arguments do not overflow. ``log2 C(p, r)`` is
    evaluated from logarithms (``math.comb`` is too slow for p near 10^6)
    with a relative error near float precision; see ``_log2_binomial``.

Categorical sequences
    ``CAT(counts)`` is the Krichevsky–Trofimov (Dirichlet(½)) code length of
    any sequence with the given category counts c₁…c_k, n = Σ cᵢ:
    ``log2 Γ(n + k/2) − log2 Γ(k/2) − Σᵢ [log2 Γ(cᵢ + ½) − log2 Γ(½)]``.
    It takes one iterable (or a 1-D integer numpy array) of counts. Zero
    counts are categories that cost probability mass without being used.
    No categories, one category, or n = 0 cost 0 bits.

Strings and numbers
    ``STR(s) = LN0(b) + 8b`` for the UTF-8 byte length b. Lone surrogates
    count 3 bytes each (``surrogatepass``), so the code is total over
    ``str``.

    ``NUM(x)`` follows the JSON value model of :mod:`jsinfer.core.jsonvalue`:
    a number with a zero fractional part (``1``, ``1.0``, ``1e3``, ``-0.0``)
    is an integer and costs ``LZ(int(x))``, with the exact value of the
    float. A non-integral number is written as the shortest decimal
    ``m·10^e`` (m without trailing zeros) that reads back as the same
    double, which is what ``repr`` prints, and costs ``LZ(m) + LZ(e)``.
    :func:`decimal_parts` returns that ``(m, e)``. Booleans are not numbers.

Bounds
    ``BOUND(v)`` codes a bound value as a ``UNI(3)`` family flag plus the
    cheapest family part that represents v: decimal ``LZ(m) + LZ(e)`` for the
    minimal ``m·10^e`` (every number), zero ``0`` (v = 0), or an integer type
    limit ``UNI(16)`` (v in :data:`INTEGER_LIMITS`). Round numbers and 0 are
    therefore cheap.

Universal values
    ``U(v)`` codes any JSON value: a ``UNI(7)`` type tag (the seven JTypes)
    plus 0 for null, 1 for a boolean, ``NUM`` for a number, ``STR`` for a
    string, ``LN0(len) + Σ U(e)`` for an array, and
    ``LN0(#keys) + Σ (STR(k) + U(val))`` for an object. Tuples are arrays.
    The walk is iterative, so deep nesting does not hit the recursion limit.

Determinism
    Every sum of three or more terms uses ``math.fsum``, which rounds the
    exact sum once, so a result does not depend on the order of its terms:
    ``CAT`` is bit-identical for any permutation of the counts, and ``U``
    for any key order of an object (Requirement 19.6). Non-JSON values raise
    ``TypeError``, and NaN or infinities ``ValueError``, as in
    :mod:`jsinfer.core.jsonvalue`. Out-of-domain integers (``LN(0)``,
    ``SUB(3, 2)``, negative counts) raise ``ValueError``. No code returns NaN
    or infinity: ``SUB`` with a subset size near 10^308 and ``CAT`` with a
    total count beyond about 10^305 (the range of ``math.lgamma``) raise
    ``OverflowError`` instead.
"""

from __future__ import annotations

import math
import operator
from collections.abc import Iterable
from decimal import Decimal
from typing import Any, Final

import numpy as np

from jsinfer.core.determinism import (
    RANK_ARRAY,
    RANK_BOOLEAN,
    RANK_INTEGER,
    RANK_NULL,
    RANK_NUMBER,
    RANK_STRING,
    json_type_rank,
)
from jsinfer.core.jsonvalue import JType

__all__ = [
    "BOUND",
    "CAT",
    "INTEGER_LIMITS",
    "LN",
    "LN0",
    "LZ",
    "NUM",
    "RISSANEN_C0",
    "STR",
    "SUB",
    "U",
    "UNI",
    "decimal_parts",
    "utf8_length",
]

# Rissanen's normalizing constant: Σ_{n≥1} 2^(−log2* n) = c0, so LN is a
# complete prefix code over the positive integers.
RISSANEN_C0: Final = 2.865064
_LOG2_C0: Final = math.log2(RISSANEN_C0)

_LN2: Final = math.log(2.0)
_LGAMMA_HALF: Final = math.lgamma(0.5)  # ln Γ(½) = ln √π

_INTEGER_WIDTHS: Final = (8, 16, 32, 64)

# The 16 signed and unsigned 8/16/32/64-bit integer limits that BOUND codes
# as UNI(16), in table order: the signed minimum and maximum for each width,
# then the unsigned minimum and maximum for each width. Every unsigned
# minimum is 0, so 13 values are distinct; BOUND codes 0 through the cheaper
# zero family. Bound selection (task 12.4) proposes the same limits.
INTEGER_LIMITS: Final[tuple[int, ...]] = (
    *(limit for width in _INTEGER_WIDTHS for limit in (-(2 ** (width - 1)), 2 ** (width - 1) - 1)),
    *(limit for width in _INTEGER_WIDTHS for limit in (0, 2**width - 1)),
)
_INTEGER_LIMIT_SET: Final = frozenset(INTEGER_LIMITS)

# Family flag sizes and type-tag alphabets of BOUND and U.
_BOUND_FAMILIES: Final = 3  # decimal, zero, integer limit
_N_JTYPES: Final = len(JType)


def _as_int(value: Any, what: str) -> int:
    """Return ``value`` as a Python ``int``; rejects bools, floats, and other non-integers."""
    if isinstance(value, (bool, np.bool_)):
        raise TypeError(f"{what} must be an integer, not a boolean")
    try:
        return operator.index(value)
    except TypeError:
        raise TypeError(f"{what} must be an integer, got {type(value).__name__}") from None


# ---------------------------------------------------------------------------
# Integers


def LN(n: int) -> float:
    """Rissanen's universal code length of a positive integer ``n``.

    ``log2 c0 + log2 n + log2 log2 n + …``, summing the positive terms only.
    ``LN(1) = log2 c0 ≈ 1.519``.
    """
    k = _as_int(n, "n")
    if k < 1:
        raise ValueError(f"LN codes integers n >= 1, got {k}")
    terms = [_LOG2_C0]
    t = math.log2(k)  # no float conversion, so ints of any size work
    while t > 0.0:
        terms.append(t)
        t = math.log2(t)
    return math.fsum(terms)


def LN0(n: int) -> float:
    """Universal code length of a non-negative integer: ``LN(n + 1)``."""
    k = _as_int(n, "n")
    if k < 0:
        raise ValueError(f"LN0 codes integers n >= 0, got {k}")
    return LN(k + 1)


def LZ(z: int) -> float:
    """Universal code length of any integer: a sign bit plus ``LN0(|z|)``."""
    return 1.0 + LN0(abs(_as_int(z, "z")))


def UNI(k: int) -> float:
    """Uniform code length for one of ``k`` options: ``log2 k``, or 0 when ``k <= 1``."""
    m = _as_int(k, "k")
    return math.log2(m) if m > 1 else 0.0


# Switch point between the two numerical methods of _log2_binomial. It picks
# how log2 C(p, r) is evaluated, never what is decided.
_DIRECT_SUM_TERMS: Final = 2**16


def _log2_binomial(p: int, r: int) -> float:
    """``log2 C(p, r)`` for ``0 <= r <= p``, exactly 0 when ``r`` is 0 or ``p``.

    The exact ``math.comb`` takes seconds for p near 10^6, and
    ``ln Γ(p + 1) − ln Γ(p − r + 1)`` cancels catastrophically for large p,
    so with ``r`` replaced by ``min(r, p − r)`` (which makes the result
    exactly symmetric) the value is:

    - for r up to 2^16, ``Σ_{i<r} ln(p − i) − ln Γ(r + 1)``, where
      ``math.log`` takes ints of any size;
    - for larger r, Stirling's series ``r ln(p/r) + (p − r) ln(1 + r/(p − r))
      − ½ ln(2π r (p − r)/p) + (1/p − 1/r − 1/(p − r))/12``, whose truncation
      error is below ``1/(100 r³)`` nats.

    Both keep a relative error near the float precision for every p. Raises
    ``OverflowError`` only when the length itself exceeds the float range
    (``r`` beyond about 10^308).
    """
    r = min(r, p - r)
    if r == 0:
        return 0.0
    if r <= _DIRECT_SUM_TERMS:
        nats = math.fsum([*(math.log(p - i) for i in range(r)), -math.lgamma(r + 1)])
    else:
        x = r / (p - r)  # correctly rounded int division, in [0, 1]
        log1p_ratio = math.log1p(x) / x if x > 0.0 else 1.0  # → 1 as x → 0 (x underflows for huge p)
        log_r, log_q, log_p = math.log(r), math.log(p - r), math.log(p)
        nats = math.fsum(
            (
                float(r) * (log_p - log_r),
                float(r) * log1p_ratio,  # (p − r) ln(1 + x)
                -0.5 * math.fsum((math.log(2.0 * math.pi), log_r, log_q, -log_p)),
                math.fsum((1 / p, -1 / r, -1 / (p - r))) / 12.0,
            )
        )
    return max(0.0, nats / _LN2)


def SUB(r: int, p: int) -> float:
    """Code length of a subset of size ``r`` out of ``p`` items: ``LN0(r) + log2 C(p, r)``."""
    size = _as_int(r, "r")
    total = _as_int(p, "p")
    if not 0 <= size <= total:
        raise ValueError(f"SUB needs 0 <= r <= p, got r={size}, p={total}")
    return LN0(size) + _log2_binomial(total, size)


# ---------------------------------------------------------------------------
# Categorical sequences


def _counts(counts: Iterable[int] | np.ndarray) -> list[int]:
    if isinstance(counts, np.ndarray):
        if counts.ndim != 1:
            raise ValueError(f"counts must be one-dimensional, got shape {counts.shape}")
        if counts.dtype.kind not in "iu":
            raise TypeError(f"counts must be integers, got dtype {counts.dtype}")
        items: Iterable[Any] = counts.tolist()
    else:
        items = counts
    out = [_as_int(c, "a count") for c in items]
    for c in out:
        if c < 0:
            raise ValueError(f"counts must be non-negative, got {c}")
    return out


def CAT(counts: Iterable[int] | np.ndarray) -> float:
    """KT (Dirichlet(½)) code length of a sequence with category counts ``counts``.

    ``log2 Γ(n + k/2) − log2 Γ(k/2) − Σᵢ [log2 Γ(cᵢ + ½) − log2 Γ(½)]`` for k
    categories and n = Σ cᵢ. It depends only on the multiset of counts and is
    bit-identical for every order of ``counts``. ``CAT(())``, ``CAT((n,))``,
    and all-zero counts are 0.
    """
    cs = _counts(counts)
    k = len(cs)
    n = sum(cs)  # exact integer sum
    if k <= 1 or n == 0:
        return 0.0
    terms = [math.lgamma(n + k / 2), -math.lgamma(k / 2)]
    for c in cs:
        if c:  # a zero count contributes lgamma(½) − lgamma(½) = 0
            terms.append(-math.lgamma(c + 0.5))
            terms.append(_LGAMMA_HALF)
    return max(0.0, math.fsum(terms) / _LN2)


# ---------------------------------------------------------------------------
# Strings and numbers


def utf8_length(s: str) -> int:
    """UTF-8 byte length of ``s``; a lone surrogate counts 3 bytes."""
    if not isinstance(s, str):
        raise TypeError(f"expected a str, got {type(s).__name__}")
    return len(s.encode("utf-8", "surrogatepass"))


def STR(s: str) -> float:
    """Literal string code length: ``LN0(b) + 8b`` for the UTF-8 byte length b."""
    b = utf8_length(s)
    return LN0(b) + 8.0 * b


def _number_rank(x: Any) -> int:
    rank = json_type_rank(x)  # TypeError for non-JSON values, ValueError for NaN/inf
    if rank not in (RANK_INTEGER, RANK_NUMBER):
        raise TypeError(f"expected a JSON number, got {JType(rank).keyword}")
    return rank


def decimal_parts(x: int | float) -> tuple[int, int]:
    """Return the minimal ``(m, e)`` with ``x == m·10^e`` and ``m`` free of trailing zeros.

    Integral numbers use their exact value (``1e3`` → ``(1, 3)``, and
    ``1e23`` → ``(99999999999999991611392, 0)``, the double's value).
    Non-integral numbers use the shortest decimal that reads back as the
    same double (``0.1`` → ``(1, -1)``). Zero is ``(0, 0)``.
    """
    if _number_rank(x) == RANK_INTEGER:
        m, e = int(x), 0
    else:
        sign, digits, exponent = Decimal(repr(float(x))).as_tuple()
        if not isinstance(exponent, int):  # unreachable: json_type_rank rejected NaN and inf
            raise ValueError(f"non-finite number has no decimal form: {x!r}")
        m = int("".join(map(str, digits)))
        m, e = (-m if sign else m), exponent
    if m == 0:
        return (0, 0)
    while m % 10 == 0:
        m //= 10
        e += 1
    return (m, e)


def NUM(x: int | float) -> float:
    """Number literal code length: ``LZ(x)`` for integers, ``LZ(m) + LZ(e)`` otherwise.

    Integers are numbers with a zero fractional part, however written.
    Non-integral numbers use the shortest decimal ``m·10^e`` of
    :func:`decimal_parts`.
    """
    if _number_rank(x) == RANK_INTEGER:
        return LZ(int(x))
    m, e = decimal_parts(x)
    return LZ(m) + LZ(e)


def BOUND(v: int | float) -> float:
    """Bound value code length: ``UNI(3)`` family flag plus the cheapest family part.

    The families are decimal (``LZ(m) + LZ(e)`` for the minimal ``m·10^e``),
    zero (0 bits, for v = 0), and integer limit (``UNI(16)``, for v in
    :data:`INTEGER_LIMITS`).
    """
    m, e = decimal_parts(v)  # validates v as a JSON number
    if m == 0:
        part = 0.0
    else:
        part = LZ(m) + LZ(e)
        # e >= 0 exactly when v is an integer, and m·10^e is then its exact value.
        if e >= 0 and m * 10**e in _INTEGER_LIMIT_SET:
            part = min(part, UNI(len(INTEGER_LIMITS)))
    return UNI(_BOUND_FAMILIES) + part


# ---------------------------------------------------------------------------
# Universal values


def U(value: Any) -> float:
    """Universal code length of any JSON value (see the module docstring).

    The result is bit-identical for every key order of every object.
    """
    tag = UNI(_N_JTYPES)
    terms: list[float] = []
    pending: list[Any] = [value]
    while pending:
        v = pending.pop()
        rank = json_type_rank(v)
        terms.append(tag)
        if rank == RANK_NULL:
            continue
        if rank == RANK_BOOLEAN:
            terms.append(1.0)
        elif rank in (RANK_INTEGER, RANK_NUMBER):
            terms.append(NUM(v))
        elif rank == RANK_STRING:
            terms.append(STR(v))
        elif rank == RANK_ARRAY:
            terms.append(LN0(len(v)))
            pending.extend(v)
        else:
            terms.append(LN0(len(v)))
            for key, item in v.items():
                if not isinstance(key, str):
                    raise TypeError(f"object keys must be str, got {type(key).__name__}")
                terms.append(STR(key))
                pending.append(item)
    return math.fsum(terms)
