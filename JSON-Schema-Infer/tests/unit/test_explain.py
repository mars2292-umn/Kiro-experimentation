"""Unit tests for explain rendering (task 17.3; Requirements 26.9, 26.12).

Covers ``core.provenance.render_record`` and ``ExplainError``, and
``io.explain.explain`` (re-exported as ``core.provenance.explain``): pointer
resolution against a schema and its explanation file, the plain-text
rendering of every record field, and the typed errors of 26.12.

The text is checked two ways: a golden rendering of a record that holds
every field, and a parser (:func:`parse_rendered`) that reads any rendering
back into the record's JSON form, so that records from real pipeline runs
(MDL, fallback, Baseline_Rule, branch, and relaxed records) are shown to be
rendered completely and exactly.
"""

from __future__ import annotations

import json
import pickle
import re
from pathlib import Path
from typing import Any

import pytest

from jsinfer.config.registry import REGISTRY
from jsinfer.config.run_config import RunConfiguration, technique_setting
from jsinfer.core import provenance
from jsinfer.core.provenance import (
    DECISION_MODEL_COMPARISON,
    EXPLAIN_CHECKS,
    EXPLAIN_INPUTS,
    MDL_COMPARISON,
    REMOVED,
    CandidateScore,
    Cutoff,
    ExplainError,
    FallbackInfo,
    ProvenanceRecord,
    RelaxationRecord,
    Resolution,
    SampleInfo,
    Support,
    explain,
    fallback_technique,
    render_record,
)
from jsinfer.core.schema_model import SchemaModel, SchemaNode
from jsinfer.io import explain as explain_module
from jsinfer.io.emit import ExplanationFile, SchemaEmitter
from jsinfer.io.load import load_explanation
from jsinfer.pipeline.infer import PipelineResult, infer_documents
from jsinfer.structure.analyzer import StructureAnalyzer

EMITTER = SchemaEmitter()
ONE_WORKER = RunConfiguration.resolve({"workers": 1})
NO_HYBRID = RunConfiguration.resolve({"workers": 1, "techniques.structure.hybrid_record_map": False})
ALL_OFF = RunConfiguration.resolve({"workers": 1, **{technique_setting(tid): False for tid in REGISTRY.ids()}})


# ---------------------------------------------------------------------------
# Reading a rendering back into the record's JSON form
# ---------------------------------------------------------------------------

_ITEM = re.compile(r"\[(\d+)\]( \(selected\))?")


def _tree(text: str) -> list[dict[str, Any]]:
    """The lines of a rendering as a tree of {label, value, index, selected, children} by indentation."""
    assert text.endswith("\n")
    lines = text[:-1].split("\n")
    assert text.splitlines() == lines  # no value hides a line break
    root: list[dict[str, Any]] = []
    stack: list[tuple[int, list[dict[str, Any]]]] = [(-1, root)]
    for line in lines:
        body = line.lstrip(" ")
        indent = len(line) - len(body)
        assert indent % 2 == 0, line
        depth = indent // 2
        node: dict[str, Any] = {"label": None, "value": None, "index": None, "selected": False, "children": []}
        item = _ITEM.fullmatch(body)
        if item is not None:
            node["index"], node["selected"] = int(item[1]), item[2] is not None
        else:
            if body.startswith('"'):  # a quoted label: a details key
                label, end = json.JSONDecoder().raw_decode(body)
                rest = body[end:]
            else:
                label, colon, after = body.partition(":")
                assert colon, line
                rest = ":" + after
            assert rest == ":" or rest.startswith(": "), line
            node["label"] = label
            node["value"] = rest[2:] if rest != ":" else None
        while stack[-1][0] >= depth:
            stack.pop()
        assert stack[-1][0] == depth - 1, f"bad indentation: {line!r}"
        stack[-1][1].append(node)
        stack.append((depth, node["children"]))
    return root


def _text(value: str) -> str:
    return json.loads(value) if value.startswith('"') else value


def _fields(nodes: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    labels = [node["label"] for node in nodes]
    assert None not in labels and len(set(labels)) == len(labels), labels
    return {node["label"]: node for node in nodes}


def _items(node: dict[str, Any]) -> list[dict[str, Any]]:
    items = node["children"]
    assert node["value"] is None and [item["index"] for item in items] == list(range(len(items)))
    return items


def _candidate(item: dict[str, Any]) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for label, node in _fields(item["children"]).items():
        out[label] = _text(node["value"]) if label in ("technique", "outcome") else json.loads(node["value"])
    return out


def _resolution(node: dict[str, Any]) -> dict[str, Any]:
    fields = _fields(node["children"])
    assert set(fields) == {"method", "selected", "candidates"}
    selected = json.loads(fields["selected"]["value"])
    items = _items(fields["candidates"])
    assert [item["selected"] for item in items] == [index == selected for index in range(len(items))]
    return {
        "candidates": [_candidate(item) for item in items],
        "method": _text(fields["method"]["value"]),
        "selected": selected,
    }


def parse_rendered(text: str) -> tuple[str | None, dict[str, Any]]:
    """The pointer of the ``constraint`` line (or None) and the record's JSON form read back from the text."""
    nodes = _tree(text)
    pointer = None
    if nodes and nodes[0]["label"] == "constraint":
        pointer = _text(nodes.pop(0)["value"])
    fields = _fields(nodes)
    out: dict[str, Any] = {
        "technique": _text(fields.pop("technique")["value"]),
        "confidence": json.loads(fields.pop("confidence")["value"]),
        "support": {n["label"]: json.loads(n["value"]) for n in fields.pop("support")["children"]},
    }
    if "fallback" in fields:
        out["fallback"] = {n["label"]: _text(n["value"]) for n in fields.pop("fallback")["children"]}
    if "resolution" in fields:
        out["resolution"] = _resolution(fields.pop("resolution"))
    if "cutoff" in fields:
        cutoff: dict[str, Any] = {}
        for label, node in _fields(fields.pop("cutoff")["children"]).items():
            if label == "scores":
                cutoff[label] = [_candidate(item) for item in _items(node)]
            elif label == "value":
                cutoff[label] = json.loads(node["value"])
            else:
                cutoff[label] = _text(node["value"])
        out["cutoff"] = cutoff
    if "sample" in fields:
        out["sample"] = {n["label"]: json.loads(n["value"]) for n in fields.pop("sample")["children"]}
    if "changes" in fields:
        changes = []
        for item in _items(fields.pop("changes")):
            change: dict[str, Any] = {}
            for label, node in _fields(item["children"]).items():
                if label == "resolution":
                    change[label] = _resolution(node)
                elif label == "documents":
                    change[label] = [int(part) for part in node["value"].split(", ")]
                else:  # before, after, removed: JSON
                    change[label] = json.loads(node["value"])
            changes.append(change)
        out["changes"] = changes
    if "details" in fields:
        details = fields.pop("details")["children"]
        out["details"] = {n["label"]: json.loads(n["value"]) for n in details}
        assert list(out["details"]) == sorted(out["details"])  # code-point order
    assert not fields, f"unexpected fields {sorted(fields)}"
    return pointer, out


def exact(value: Any) -> str:
    """JSON text that tells 1 from 1.0 and true from 1, for exact comparisons."""
    return json.dumps(value, sort_keys=True, ensure_ascii=False)


def assert_renders_completely(record: ProvenanceRecord, text: str, pointer: str | None) -> None:
    parsed_pointer, parsed = parse_rendered(text)
    assert parsed_pointer == pointer
    assert exact(parsed) == exact(record.to_json())


# ---------------------------------------------------------------------------
# render_record
# ---------------------------------------------------------------------------

BIG = int(1.7976931348623157e308)

RICH = ProvenanceRecord(
    technique="values.bounds",
    support=Support(7, 9),
    confidence=0.875,
    fallback=FallbackInfo("decide.dm_closed_enum", "missing-model"),
    resolution=Resolution(
        "mdl",
        1,
        (
            CandidateScore("values.bounds", "maximum=omit", 40),
            CandidateScore(
                "values.bounds",
                "maximum=100",
                12.5,
                model_bits=2,
                data_bits=10.5,
                exception_bits=0.5,
                n_rejected=1,
                n_scored=9,
            ),
        ),
    ),
    cutoff=Cutoff(
        DECISION_MODEL_COMPARISON,
        scores=(CandidateScore(None, "closed", 0.75), CandidateScore(None, "open", 0.25)),
        artifact="dm.closed_enum.synthetic",
    ),
    sample=SampleInfo(5, 9),
    changes=(
        RelaxationRecord(
            {"maximum": 5},
            {"maximum": 100},
            (3, 1, 3),
            resolution=Resolution("priority", 0, (CandidateScore(None, "widen maximum", None),)),
        ),
        RelaxationRecord({"minimum": 1.0}, REMOVED, (2,)),
    ),
    details={
        "é😀": [2.0, -0.0, 5e-324, 1.7976931348623157e308, 2**70],
        "a: b": {"z": None, "a": True},
        "line\nbreak": "x\u2028y",
        "count": 3,
    },
)

RICH_TEXT = f"""\
constraint: /properties/a~1b/maximum
technique: values.bounds
support:
  consistent: 7
  total: 9
  ratio: 0.7777777777777778
confidence: 0.875
fallback:
  technique: decide.dm_closed_enum
  reason: missing-model
resolution:
  method: mdl
  selected: 1
  candidates:
    [0]
      technique: values.bounds
      outcome: maximum=omit
      score: 40.0
    [1] (selected)
      technique: values.bounds
      outcome: maximum=100
      score: 12.5
      model_bits: 2.0
      data_bits: 10.5
      exception_bits: 0.5
      n_rejected: 1
      n_scored: 9
cutoff:
  source: Decision_Model comparison
  scores:
    [0]
      outcome: closed
      score: 0.75
    [1]
      outcome: open
      score: 0.25
  artifact: dm.closed_enum.synthetic
sample:
  sampled: 5
  total: 9
changes:
  [0]
    before: {{"maximum": 5}}
    after: {{"maximum": 100}}
    documents: 1, 3
    resolution:
      method: priority
      selected: 0
      candidates:
        [0] (selected)
          outcome: widen maximum
  [1]
    before: {{"minimum": 1}}
    removed: true
    documents: 2
details:
  "a: b": {{"a": true, "z": null}}
  count: 3
  "line\\nbreak": "x\\u2028y"
  é😀: [2, 0, 5e-324, {BIG}, 1180591620717411303424]
"""


def test_every_field_of_a_rich_record_is_rendered() -> None:
    text = render_record(RICH, "/properties/a~1b/maximum")
    assert text == RICH_TEXT
    assert_renders_completely(RICH, text, "/properties/a~1b/maximum")
    assert text.encode("utf-8").decode("utf-8") == text  # strict UTF-8 writes it
    # Without a pointer, the constraint line is left out.
    assert render_record(RICH) == RICH_TEXT.split("\n", 1)[1]


def test_minimal_records_print_only_the_fields_they_hold() -> None:
    baseline = ProvenanceRecord.baseline("type", Support(0, 0))
    assert render_record(baseline, "/type") == (
        "constraint: /type\n"
        "technique: baseline:type\n"
        "support:\n"
        "  consistent: 0\n"
        "  total: 0\n"
        "  ratio: 1.0\n"
        "confidence: 0.5\n"
    )
    fallback = ProvenanceRecord.laplace(
        fallback_technique("record_or_map"),
        Support(3, 3),
        fallback=FallbackInfo("decide.dm_record_map", "disabled"),
        resolution=Resolution("fallback", 0, (CandidateScore(None, "record", None), CandidateScore(None, "map", None))),
    )
    assert render_record(fallback) == (
        "technique: fallback:record_or_map\n"
        "support:\n"
        "  consistent: 3\n"
        "  total: 3\n"
        "  ratio: 1.0\n"
        "confidence: 0.8\n"
        "fallback:\n"
        "  technique: decide.dm_record_map\n"
        "  reason: disabled\n"
        "resolution:\n"
        "  method: fallback\n"
        "  selected: 0\n"
        "  candidates:\n"
        "    [0] (selected)\n"
        "      outcome: record\n"
        "    [1]\n"
        "      outcome: map\n"
    )
    for record in (baseline, fallback):
        assert_renders_completely(record, render_record(record, "/x"), "/x")


def test_numbers_print_exactly_as_the_explanation_file_writes_them() -> None:
    huge = 2**70
    record = ProvenanceRecord(
        "rules.cooccurrence",
        Support(huge, 3 * huge),
        1,  # stored as 1.0
        cutoff=Cutoff("cooccur.min_support", value=-0.0),  # -0.0 is stored as 0.0
        resolution=Resolution("mdl", 0, (CandidateScore("rules.cooccurrence", "rule", 1e-300, data_bits=1e16),)),
    )
    text = render_record(record)
    assert f"  consistent: {huge}\n  total: {3 * huge}\n  ratio: 0.3333333333333333\n" in text
    assert "\nconfidence: 1.0\n" in text
    assert "\n  value: 0.0\n" in text and "-0" not in text
    assert "score: 1e-300\n" in text and "data_bits: 1e+16\n" in text
    assert_renders_completely(record, text, None)
    # Every number is the one the explanation file holds, spelled the same way.
    data = ExplanationFile({"/x": record}).dumps().decode("utf-8")
    for number in (str(huge), str(3 * huge), "0.3333333333333333", "1.0", "0.0", "1e-300", "1e+16"):
        assert number in data


@pytest.mark.parametrize(
    ("text", "rendered"),
    [
        ("decide.mdl", "decide.mdl"),
        ("maximum=100", "maximum=100"),
        ("a: b", "a: b"),  # a value may hold ': '; only keys are split there
        ("é😀 naïve", "é😀 naïve"),  # non-ASCII prints as is
        ("", '""'),
        (" padded", '" padded"'),
        ("trailing ", '"trailing "'),
        ('"quoted"', '"\\"quoted\\""'),
        ("two\nlines", '"two\\nlines"'),
        ("tab\there", '"tab\\there"'),
        ("nbsp\u00a0x", '"nbsp\u00a0x"'),  # not printable as plain text, but JSON leaves it as is
        ("ls\u2028ps\u2029nel\x85", '"ls\\u2028ps\\u2029nel\\u0085"'),
        ("lone\udcff", '"lone\\udcff"'),
    ],
)
def test_text_fields_print_plainly_unless_ambiguous(text: str, rendered: str) -> None:
    record = ProvenanceRecord(
        "values.bounds",
        Support(1, 1),
        0.5,
        resolution=Resolution("priority", 0, (CandidateScore(None, text, None),)),
    )
    lines = render_record(record).split("\n")
    assert f"      outcome: {rendered}" in lines
    output = render_record(record)
    output.encode("utf-8")  # always encodable, lone surrogates included
    assert output.splitlines() == output[:-1].split("\n")
    _, parsed = parse_rendered(output)
    assert parsed["resolution"]["candidates"][0]["outcome"] == text


def test_details_keys_are_quoted_when_they_hold_a_colon() -> None:
    record = ProvenanceRecord("values.bounds", Support(1, 1), 0.5, details={"a:b": 1, "plain_key": "v", "": None})
    lines = render_record(record).split("\n")
    assert lines[-5:] == ["details:", '  "": null', '  "a:b": 1', '  plain_key: "v"', ""]
    assert_renders_completely(record, render_record(record), None)


def test_render_record_checks_its_arguments() -> None:
    with pytest.raises(TypeError, match="ProvenanceRecord"):
        render_record(RICH.to_json())  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="pointer"):
        render_record(RICH, 3)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# ExplainError
# ---------------------------------------------------------------------------


def test_explain_error_fields_message_and_pickling() -> None:
    error = ExplainError("schema", "read", "the schema file 'a.json'", "cannot be read: No such file or directory")
    assert str(error) == "the schema file 'a.json': cannot be read: No such file or directory"
    assert isinstance(error, ValueError)
    again = pickle.loads(pickle.dumps(error))
    assert (again.input, again.check, again.name, again.reason, str(again)) == (
        "schema",
        "read",
        "the schema file 'a.json'",
        "cannot be read: No such file or directory",
        str(error),
    )
    assert EXPLAIN_INPUTS == ("schema", "explanation", "pointer")
    assert EXPLAIN_CHECKS == ("read", "parse", "syntax", "resolve", "entry")
    with pytest.raises(ValueError, match="input"):
        ExplainError("bundle", "read", "x", "y")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="check of the schema"):
        ExplainError("schema", "entry", "x", "y")
    with pytest.raises(ValueError, match="check of the pointer"):
        ExplainError("pointer", "read", "x", "y")
    with pytest.raises(ValueError, match="reason"):
        ExplainError("pointer", "syntax", "x", "")


def test_explain_is_re_exported_from_core_provenance() -> None:
    assert provenance.explain is explain_module.explain is explain
    assert "explain" in provenance.__all__ and "render_record" in provenance.__all__
    with pytest.raises(AttributeError, match="no attribute 'nope'"):
        provenance.nope  # noqa: B018


# ---------------------------------------------------------------------------
# Resolution: escapes, empty names, and what a non-Constraint pointer names
# ---------------------------------------------------------------------------

# Property names that need escapes ("a/b", "m~n"), the empty name, a property
# named like the inline keyword, a schema object without Constraints, a
# boolean subschema, and Constraints inside $defs.
SCHEMA: dict[str, Any] = {
    "type": "object",
    "properties": {
        "": {"type": "string", "minLength": 1},
        "a/b": {"type": "integer", "minimum": 0, "maximum": 9},
        "m~n": {"enum": ["y", "x"]},
        "x-provenance": {"type": "boolean"},
        "pair": {"type": "array", "prefixItems": [{"type": "string"}, True], "items": False},
        "map": {"type": "object", "additionalProperties": {"type": "integer"}, "propertyNames": {"format": "uuid"}},
        "node": {"$ref": "#/$defs/node"},
        "free": {"title": "no Constraint here"},
    },
    "additionalProperties": False,
    "required": ["a/b", ""],
    "dependentRequired": {"a/b": ["m~n"], "": ["a/b"]},
    "anyOf": [{"required": ["pair"]}, True],
    "$defs": {"node": {"type": "object", "properties": {"next": {"$ref": "#/$defs/node"}}, "x-latent-type": "tree"}},
}


def schema_files(schema: Any, tmp_path: Path, *, inline: bool = False) -> tuple[Path, Path, dict[str, ProvenanceRecord]]:
    """Write ``schema`` as the Schema_Emitter would, with one distinct record per Constraint, and its explanation file."""
    m = SchemaModel(SchemaNode.from_json(schema))
    _, pointers = EMITTER.layout(m)
    for index, cid in enumerate(pointers):
        support = Support(index, index + 1)
        if cid.keyword in ("type", "required"):
            m.provenance[cid] = ProvenanceRecord.baseline(cid.keyword, support)
        else:
            m.provenance[cid] = ProvenanceRecord("values.bounds", support, 0.25, details={"index": index})
    schema_path, explanation_path = tmp_path / "schema.json", tmp_path / "explanation.json"
    schema_path.write_bytes(EMITTER.emit(m, inline_provenance=inline))
    explanation_path.write_bytes(EMITTER.explanation(m, pointers))
    return schema_path, explanation_path, {pointer: m.provenance[cid] for cid, pointer in pointers.items()}


ESCAPED_POINTERS = [
    "/properties//type",
    "/properties//minLength",
    "/properties/a~1b/maximum",
    "/properties/m~0n/enum",
    "/properties/x-provenance/type",
    "/properties/pair/prefixItems",
    "/properties/map/propertyNames",
    "/properties/map/propertyNames/format",
    "/properties/map/additionalProperties",
    "/properties/node/$ref",
    "/required/0",
    "/required/1",
    "/dependentRequired/",
    "/dependentRequired/a~1b",
    "/anyOf/0",
    "/anyOf/1",
    "/anyOf/0/required/0",
    "/additionalProperties",
    "/$defs/node/x-latent-type",
    "/$defs/node/properties/next/$ref",
]


@pytest.mark.parametrize("inline", [False, True], ids=["plain", "inline-provenance"])
def test_pointers_with_escapes_and_empty_names_resolve(tmp_path: Path, inline: bool) -> None:
    schema_path, explanation_path, records = schema_files(SCHEMA, tmp_path, inline=inline)
    assert set(ESCAPED_POINTERS) <= set(records)
    for pointer in records:
        text = explain(schema_path, explanation_path, pointer)
        assert text == render_record(records[pointer], pointer)
        assert_renders_completely(records[pointer], text, pointer)
    # required is emitted sorted, so /required/0 names the empty property name.
    assert json.loads(schema_path.read_bytes())["required"] == ["", "a/b"]


@pytest.mark.parametrize(
    ("pointer", "reason"),
    [
        # Schema objects, which are not Constraints; the reason lists their Constraints.
        ("", "it names the root schema object, which is not a Constraint; the Constraints it holds are '/type', "),
        ("/properties/", "the schema object at '/properties/', which is not a Constraint; the Constraints it holds "
         "are '/properties//type', '/properties//minLength'"),
        ("/properties/a~1b", "the schema object at '/properties/a~1b'"),
        ("/properties/x-provenance", "the schema object at '/properties/x-provenance', which is not a Constraint; "
         "the Constraints it holds are '/properties/x-provenance/type'"),
        ("/properties/free", "the schema object at '/properties/free', which is not a Constraint and holds none"),
        ("/properties/pair/prefixItems/0", "the schema object at '/properties/pair/prefixItems/0'"),
        ("/properties/pair/prefixItems/1", "the boolean schema true, which holds no Constraint"),
        ("/properties/pair/items", None),  # items: false is a Constraint
        ("/$defs/node", "the Constraints it holds are '/$defs/node/type', '/$defs/node/properties', "
         "'/$defs/node/x-latent-type'"),
        # Keywords that are not Constraints, or whose entries are.
        ("/$schema", "the keyword '$schema' at '/$schema' is not a Constraint"),
        ("/$defs", "the keyword '$defs' at '/$defs' is not a Constraint"),
        ("/properties/free/title", "the keyword 'title' at '/properties/free/title' is not a Constraint"),
        ("/required", "each entry of the keyword 'required' at '/required' is its own Constraint: '/required/0', "
         "'/required/1'"),
        ("/dependentRequired", "is its own Constraint: '/dependentRequired/', '/dependentRequired/a~1b'"),
        ("/anyOf", "each entry of the keyword 'anyOf' at '/anyOf' is its own Constraint: '/anyOf/0', '/anyOf/1'"),
        # Locations inside a Constraint's value.
        ("/properties/m~0n/enum/0", "it points inside the value of the keyword 'enum' at '/properties/m~0n/enum', "
         "which is one Constraint as a whole"),
        ("/dependentRequired/a~1b/0", "inside the value of the keyword 'dependentRequired' at '/dependentRequired', "
         "which belongs to the Constraint at '/dependentRequired/a~1b'"),
        # Locations that do not exist.
        ("/properties/a/b/type", "does not resolve to a location in the schema file "),
        ("/properties/zzz", "the location '/properties' has no member 'zzz'"),
        ("/nope", "the root has no member 'nope'"),
        ("/required/2", "the location '/required' is an array of length 2, which has no element '2'"),
        ("/required/01", "which has no element '01'"),
        ("/required/-", "which has no element '-'"),
        ("/required/0/x", "the location '/required/0' holds a string, which has no members"),
        ("/properties/pair/prefixItems/1/type", "the location '/properties/pair/prefixItems/1' holds a boolean"),
        ("/x-provenance", "the root has no member 'x-provenance'"),  # the schema has no inline provenance
    ],
)
def test_pointers_that_do_not_name_a_constraint(tmp_path: Path, pointer: str, reason: str | None) -> None:
    schema_path, explanation_path, records = schema_files(SCHEMA, tmp_path)
    if reason is None:
        assert pointer in records
        explain(schema_path, explanation_path, pointer)
        return
    assert pointer not in records
    with pytest.raises(ExplainError) as info:
        explain(schema_path, explanation_path, pointer)
    error = info.value
    assert (error.input, error.check) == ("pointer", "resolve")
    assert error.name == f"the JSON Pointer {pointer!r}"
    message = str(error)
    assert message.startswith(f"the JSON Pointer {pointer!r}: does not resolve to a ")
    assert f"the schema file {str(schema_path)!r}" in message  # names the schema it was resolved in
    assert reason in message


def test_inline_provenance_is_not_a_constraint(tmp_path: Path) -> None:
    schema_path, explanation_path, _ = schema_files(SCHEMA, tmp_path, inline=True)
    for pointer, verb in (
        ("/x-provenance", "names"),
        ("/x-provenance/~1type", "points inside"),
        ("/properties/a~1b/x-provenance/~1maximum/confidence", "points inside"),
    ):
        with pytest.raises(ExplainError, match=f"it {verb} the inline provenance annotation") as info:
            explain(schema_path, explanation_path, pointer)
        assert info.value.check == "resolve"
    # A property named like the keyword is a schema object like any other.
    explain(schema_path, explanation_path, "/properties/x-provenance/type")


def test_keywords_the_loader_leaves_out_are_not_constraints() -> None:
    schema = {"type": "integer", "multipleOf": 2, "markdownDescription": "shown in editors"}
    explanation = ExplanationFile({"/type": ProvenanceRecord.baseline("type", Support(1, 1))})
    assert explain(schema, explanation, "/type").startswith("constraint: /type\n")
    with pytest.raises(ExplainError, match="does not support the keyword 'multipleOf' at '/multipleOf'"):
        explain(schema, explanation, "/multipleOf")
    with pytest.raises(ExplainError, match="ignores the keyword 'markdownDescription'"):
        explain(schema, explanation, "/markdownDescription")


def test_a_boolean_root_holds_no_constraint() -> None:
    with pytest.raises(ExplainError, match="the root schema object, the boolean schema true, which holds no") as info:
        explain(b"true\n", b'{"format": 1, "records": {}}', "")
    assert info.value.check == "resolve"
    with pytest.raises(ExplainError, match="the root holds a boolean, which has no members"):
        explain(True, ExplanationFile({}), "/type")


# ---------------------------------------------------------------------------
# Typed errors (26.12)
# ---------------------------------------------------------------------------

GOOD_SCHEMA = b'{"type": "integer"}'
GOOD_EXPLANATION = ExplanationFile({"/type": ProvenanceRecord.baseline("type", Support(2, 2))}).dumps()


@pytest.mark.parametrize(
    ("schema", "explanation", "pointer", "failed", "check", "match"),
    [
        (b"{", GOOD_EXPLANATION, "/type", "schema", "parse", r"invalid JSON: .*\(line 1, column 2\)"),
        (b'{"type": "integer"} {}', GOOD_EXPLANATION, "/type", "schema", "parse", r"Extra data"),
        (b"[]", GOOD_EXPLANATION, "/type", "schema", "parse", "the schema root is an array"),
        (b'{"type": 5}', GOOD_EXPLANATION, "/type", "schema", "parse", "meta-schema"),
        (b"\xff", GOOD_EXPLANATION, "/type", "schema", "parse", "cannot be parsed"),
        (GOOD_SCHEMA, b"{", "/type", "explanation", "parse", r"\(line 1, column 2\)"),
        (GOOD_SCHEMA, b"{}", "/type", "explanation", "parse", "missing member 'format'"),
        (GOOD_SCHEMA, b'{"format": 2, "records": {}}', "/type", "explanation", "parse", "unsupported .* format 2"),
        (GOOD_SCHEMA, b'{"format": 1, "records": {"/type": {}}}', "/type", "explanation", "parse", "/records/~1type"),
        (GOOD_SCHEMA, GOOD_EXPLANATION, "type", "pointer", "syntax", "empty or starts with '/'$"),
        (GOOD_SCHEMA, GOOD_EXPLANATION, "#/type", "pointer", "syntax", "without the '#' of the URI fragment form"),
        (GOOD_SCHEMA, GOOD_EXPLANATION, "/a~2", "pointer", "syntax", "the '~' at index 2 is not followed by"),
        (GOOD_SCHEMA, GOOD_EXPLANATION, "/a~", "pointer", "syntax", "the '~' at index 2 is not followed by"),
        (GOOD_SCHEMA, GOOD_EXPLANATION, "/maximum", "pointer", "resolve", "the root has no member 'maximum'"),
        (b'{"type": "integer", "maximum": 3}', GOOD_EXPLANATION, "/maximum", "pointer", "entry",
         "has no entry in the explanation file$"),
    ],
)
def test_failures_name_the_input_and_the_cause(
    schema: Any, explanation: Any, pointer: str, failed: str, check: str, match: str
) -> None:
    with pytest.raises(ExplainError, match=match) as info:
        explain(schema, explanation, pointer)
    error = info.value
    assert (error.input, error.check) == (failed, check)
    names = {"schema": "the schema", "explanation": "the explanation file", "pointer": f"the JSON Pointer {pointer!r}"}
    assert error.name == names[failed]
    assert str(error).startswith(f"{names[failed]}: ")
    assert check in EXPLAIN_CHECKS


def test_file_errors_name_the_path(tmp_path: Path) -> None:
    schema_path, explanation_path, _ = schema_files({"type": "integer", "maximum": 3}, tmp_path)
    missing = tmp_path / "missing.json"
    bad = tmp_path / "bad é.json"
    bad.write_bytes(b'{"type": ')

    cases: list[tuple[Any, Any, str, str, str, str]] = [
        (missing, explanation_path, "/type", "schema", "read", "No such file or directory"),
        (tmp_path, explanation_path, "/type", "schema", "read", "cannot be read"),
        (bad, explanation_path, "/type", "schema", "parse", "cannot be parsed: invalid JSON"),
        (schema_path, missing, "/type", "explanation", "read", "No such file or directory"),
        (schema_path, tmp_path, "/type", "explanation", "read", "cannot be read"),
        (schema_path, bad, "/type", "explanation", "parse", "cannot be parsed"),
        (schema_path, schema_path, "/type", "explanation", "parse", "missing member 'format'"),
    ]
    for schema, explanation, pointer, failed, check, cause in cases:
        with pytest.raises(ExplainError) as info:
            explain(schema, explanation, pointer)
        error = info.value
        assert (error.input, error.check) == (failed, check)
        path = schema if failed == "schema" else explanation
        kind = "the schema file" if failed == "schema" else "the explanation file"
        assert error.name == f"{kind} {str(path)!r}"
        assert str(error).startswith(f"{kind} {str(path)!r}: ") and cause in str(error)

    # A Constraint without an entry: the message names the pointer and the explanation file.
    reduced = tmp_path / "reduced.json"
    full = load_explanation(explanation_path.read_bytes())
    reduced.write_bytes(ExplanationFile({k: v for k, v in full.records.items() if k != "/maximum"}).dumps())
    with pytest.raises(ExplainError) as info:
        explain(schema_path, reduced, "/maximum")
    assert (info.value.input, info.value.check) == ("pointer", "entry")
    assert str(info.value) == f"the JSON Pointer '/maximum': has no entry in the explanation file {str(reduced)!r}"
    assert explain(schema_path, reduced, "/type").startswith("constraint: /type\n")


def test_checks_run_in_argument_order(tmp_path: Path) -> None:
    missing = tmp_path / "missing.json"
    with pytest.raises(ExplainError) as info:
        explain(missing, missing, "not a pointer")
    assert (info.value.input, info.value.check) == ("schema", "read")
    with pytest.raises(ExplainError) as info:
        explain(GOOD_SCHEMA, missing, "not a pointer")
    assert (info.value.input, info.value.check) == ("explanation", "read")
    with pytest.raises(ExplainError) as info:
        explain(GOOD_SCHEMA, GOOD_EXPLANATION, "not a pointer")
    assert (info.value.input, info.value.check) == ("pointer", "syntax")


def test_python_types_outside_the_inputs_are_type_errors() -> None:
    with pytest.raises(TypeError, match="pointer must be a str"):
        explain(GOOD_SCHEMA, GOOD_EXPLANATION, b"/type")  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="schema must be a path"):
        explain(object(), GOOD_EXPLANATION, "/type")  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="explanation must be .* or an ExplanationFile"):
        explain(GOOD_SCHEMA, bytearray(GOOD_EXPLANATION), "/type")  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Records from pipeline runs
# ---------------------------------------------------------------------------

ORDERS: list[Any] = [
    {
        "id": i,
        "status": ["new", "paid", "sent"][i % 3],
        "lines": [{"sku": f"s{j}", "qty": j % 3 + 1} for j in range(i % 4)],
        "meta": {"tags": ["a", "b"][: i % 3], "source": "web" if i % 2 else "app"},
        **({"note": f"note {i}"} if i % 5 == 0 else {}),
    }
    for i in range(24)
]
# A Closed_Enum beside objects: anyOf branch records.
ENUM_AND_OBJECTS: list[Any] = ["red", "green"] * 4 + [{"a": 1}, {"a": 2}]
# A Map with key Constraints, mixed scalar types, and arrays.
MIXED: list[Any] = [
    *ORDERS[:6],
    *({f"user{3 * i + j}": {"score": i + j} for j in range(3)} for i in range(8)),
    *([i, f"n{i}"] for i in range(8)),
    1,
    2.5,
    "text",
    None,
    True,
]


def relaxed_run(monkeypatch: pytest.MonkeyPatch) -> PipelineResult:
    """A run whose inferred ``maximum`` of ``id`` is made over-tight, so the soundness loop relaxes it."""
    original = StructureAnalyzer.infer_model

    def over_tight(self: StructureAnalyzer, view: Any) -> SchemaModel:
        model = original(self, view)
        model.root.kw["properties"]["id"].kw["maximum"] = 5  # keeps its record; 18 documents exceed it
        return model

    monkeypatch.setattr(StructureAnalyzer, "infer_model", over_tight)
    try:
        return infer_documents(ORDERS, ONE_WORKER)
    finally:
        monkeypatch.undo()


def check_every_record(result: PipelineResult, tmp_path: Path) -> dict[str, ProvenanceRecord]:
    """Explain every Constraint of a run from files, and check that each record is rendered completely."""
    tmp_path.mkdir(parents=True, exist_ok=True)
    explanation = EMITTER.explanation_file(result.model, result.pointers)
    schema_path, explanation_path = tmp_path / "schema.json", tmp_path / "explanation.json"
    schema_path.write_bytes(result.schema)
    explanation_path.write_bytes(explanation.dumps())
    records = dict(explanation.records)
    assert sorted(records) == sorted(result.pointers.values())
    for pointer, record in records.items():
        text = explain(schema_path, explanation_path, pointer)
        assert text == render_record(record, pointer)
        assert_renders_completely(record, text, pointer)
    return records


def test_every_record_of_pipeline_runs_is_rendered(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    runs = {
        "orders": infer_documents(ORDERS, ONE_WORKER),
        "all-off": infer_documents(ORDERS, ALL_OFF),
        "enum-and-objects": infer_documents(ENUM_AND_OBJECTS, ONE_WORKER),
        "mixed": infer_documents(MIXED, NO_HYBRID),  # the user Records stay a Map with propertyNames
        "hybrid": infer_documents(MIXED, ONE_WORKER),  # the hybrid partition (4.5) puts them under patternProperties
        "relaxed": relaxed_run(monkeypatch),
    }
    seen: dict[str, ProvenanceRecord] = {}
    for name, result in runs.items():
        for pointer, record in check_every_record(result, tmp_path / name).items():
            seen[f"{name}:{pointer}"] = record

    def having(test: Any) -> list[str]:
        return [key for key, record in seen.items() if test(key, record)]

    # The runs cover every kind of record the pipeline writes today.
    assert having(lambda _, r: r.technique == "baseline:type")
    assert having(lambda _, r: r.technique == "baseline:required")
    assert having(
        lambda _, r: r.technique.startswith("fallback:") and r.resolution is not None and r.resolution.method == "fallback"
    )
    assert having(lambda _, r: r.fallback is not None and r.resolution is not None and r.resolution.method == "mdl")
    assert having(lambda _, r: r.cutoff is not None and r.cutoff.source == MDL_COMPARISON)
    assert having(lambda k, r: k.startswith("enum-and-objects:/anyOf/") and r.details.get("branch"))
    assert having(lambda k, _: k.startswith("mixed:") and k.endswith("/propertyNames"))
    assert having(lambda k, r: k.startswith("hybrid:") and k.endswith("/patternProperties") and r.details.get("judgment") == "hybrid")
    relaxed = seen["relaxed:/properties/id/maximum"]
    assert relaxed.changes and relaxed.changes[0].resolution is not None
    text = explain(runs["relaxed"].schema, EMITTER.explanation(runs["relaxed"].model, runs["relaxed"].pointers),
                   "/properties/id/maximum")
    assert '\n    before: {"maximum": 5}\n' in text and "\n    documents: 6, 7, 8," in text


def test_explanations_do_not_depend_on_the_input_form(tmp_path: Path) -> None:
    result = infer_documents(ENUM_AND_OBJECTS, ONE_WORKER)
    explanation = EMITTER.explanation_file(result.model, result.pointers)
    data = explanation.dumps()
    inline = EMITTER.emit(result.model, inline_provenance=True)
    schema_path, explanation_path = tmp_path / "schema.json", tmp_path / "explanation.json"
    schema_path.write_bytes(result.schema)
    explanation_path.write_bytes(data)
    schemas: list[Any] = [schema_path, result.schema, result.schema.decode("utf-8"), json.loads(result.schema),
                          result.document, inline, json.loads(inline)]
    explanations: list[Any] = [explanation_path, data, data.decode("utf-8"), json.loads(data), explanation,
                               load_explanation(data)]
    pointers = list(explanation.records)
    assert {"/anyOf/0", "/anyOf/1"} <= set(pointers)
    for index, pointer in enumerate(pointers):
        expected = explain(schema_path, explanation_path, pointer)
        assert expected.startswith(f"constraint: {pointer}\n")
        assert explain(schema_path, explanation_path, pointer) == expected  # repeatable
        # Every pairing of input forms for a few pointers, and the schema forms for all of them.
        for schema in schemas:
            for source in explanations if index < 3 or pointer.startswith("/anyOf/") else explanations[:1]:
                assert explain(schema, source, pointer) == expected


def test_explanations_are_reproducible_across_runs(tmp_path: Path) -> None:
    first = check_every_record(infer_documents(MIXED, ONE_WORKER), tmp_path / "first")
    second = check_every_record(infer_documents(MIXED, ONE_WORKER), tmp_path / "second")
    assert list(first) == list(second)
    for pointer in first:
        assert render_record(first[pointer], pointer) == render_record(second[pointer], pointer)


def test_records_inside_shared_definitions_and_branches_resolve() -> None:
    schema = {
        "anyOf": [{"$ref": "#/$defs/item"}, {"type": "null"}],
        "$defs": {"item": {"type": "object", "properties": {"k": {"const": "v"}}, "required": ["k"]}},
    }
    m = SchemaModel(SchemaNode.from_json(schema))
    _, pointers = EMITTER.layout(m)
    m.provenance = {cid: ProvenanceRecord.laplace("decide.mdl", Support(1, 2)) for cid in pointers}
    explanation = EMITTER.explanation_file(m, pointers)
    expected = {"/anyOf/0", "/anyOf/0/$ref", "/anyOf/1", "/anyOf/1/type", "/$defs/item/type",
                "/$defs/item/properties", "/$defs/item/properties/k/const", "/$defs/item/required/0"}
    assert set(explanation.records) == expected
    for pointer in sorted(expected):
        assert explain(schema, explanation, pointer) == render_record(explanation.records[pointer], pointer)
