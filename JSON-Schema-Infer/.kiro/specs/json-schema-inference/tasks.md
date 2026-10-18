# Implementation Plan: json-schema-inference

## Overview

`jsinfer` is a Python 3.11+ package (`src/jsinfer/`, uv with `==` pins and `uv.lock`, pytest + Hypothesis). The plan follows the design's milestones: M1 Core, M2 Decisions, M3 Learning, M4 Harness, M5 Should, M6 Could. M1–M4 hold the Must tier and form the critical path.

Conventions used by every task:

- Each Technique task sets its registry entry to `implemented=True`, registers its Learned_Thresholds and their validation objectives with the Trainer, and wires itself into the pipeline in the same task. Unimplemented Techniques stay registered, and enabling one is a `ConfigError`.
- No decision cut-off is hard-coded. Cut-offs come from MDL comparisons, BIC, or Learned_Thresholds. Runtime knobs are Resource_Parameters. Tests may build stub bundles with hand-set thresholds.
- Randomness goes through `core/determinism.rng(purpose, *keys)`. Emitted JSON goes through the canonical encoder.
- Property tests use the Hypothesis `ci` profile (at least 100 examples) and carry the tag `# Feature: json-schema-inference, Property N: <title>`. The shared `run_configs()` strategy draws toggles only from implemented Techniques and respects prerequisites. Full-run properties therefore pick up each Technique as soon as it is implemented. Clauses about unimplemented Techniques are gated on the registry.

## Tasks

### M1: Core

- [x] 1. Set up the project and determinism utilities
  - [x] 1.1 Create the project scaffolding
    - `pyproject.toml`: Python ≥ 3.11; `==` pins for orjson, jsonschema-rs, joblib, numpy, scipy, scikit-learn (≥ 1.6 line), datasketch, ruptures, tomli-w, psutil, threadpoolctl; dev group with pytest, hypothesis, regress; console script `jsinfer = jsinfer.cli:main`. Generate `uv.lock`
    - Package skeleton for every subpackage in the design layout; `tests/{unit,property,integration,bench_smoke,fixtures,perf}`
    - `tests/conftest.py` with Hypothesis profiles `ci` (`max_examples=100`, `derandomize=True`) and `nightly` (500); empty `tests/strategies.py`
    - _Requirements: 29.1, 29.5_
  - [x] 1.2 Implement determinism and sampling utilities
    - `core/determinism.py`: `rng(purpose, *keys)` from `SeedSequence` with 32-bit BLAKE2b keys and default seed 0; canonical code-point and value ordering helpers; the canonical encoder (`json.dumps(indent=2, ensure_ascii=False, allow_nan=False) + "\n"`); atomic file and directory writers (temp, then rename)
    - `core/sampling.py`: `sample_rows(rows, k, rng)` returns sorted distinct rows when n > k and all rows otherwise
    - _Requirements: 2.3, 27.2, 27.5, 29.8_
  - [x] 1.3 Write smoke and lint tests
    - Lock file pins exactly one version per package; `src/` imports no validator other than `jsonschema_rs`; no `set` iteration without `sorted`; no `hash()` on output paths
    - _Requirements: 27.5, 29.3, 29.5_

- [x] 2. Implement the core data models
  - [x] 2.1 Implement Path and Step (`core/paths.py`)
    - Key, Elem (`[*]`), and MapVal (`{*}`) steps ordered Key < Elem < MapVal; rendering (`$`, `.name`, `["name"]`); parsing of configured Path strings; step-tuple sorting
    - _Requirements: 4.3, 7.2, 18.3_
  - [x] 2.2 Implement the JSON value model and interning (`core/jsonvalue.py`)
    - `JType`, with INTEGER covering every zero-fraction number (`1.0`, `1e3`); canonical key `(type_rank, value)` with booleans kept apart from numbers; canonical value order (containers by canonical bytes); JSON Schema instance and `enum` equality
    - _Requirements: 12.1, 13.2, 15.1_
  - [x] 2.3 Implement the ObservationStore, views, and stats (`core/observations.py`, `core/views.py`, `core/stats.py`)
    - `flatten_chunk` and `merge_partials` over shape-path columns, with keys and values re-interned in canonical order so ids do not depend on chunking or document order
    - `View` with `child_key`, `child_map`, `elements(pos)`, `subset`, and multi-part union views; child views via `np.searchsorted`
    - `FlagSet` excludes flagged documents, flagged items, and their descendants. `ViewStats` holds type counts, value-id counts, length histograms, and presence bitsets, cached by (view digest, flag epoch)
    - _Requirements: 17.4, 19.6, 27.6_
  - [x] 2.4 Implement the Schema_Model (`core/schema_model.py`)
    - `SchemaNode` (boolean or keyword dict, stable `nid`); `SchemaModel` with `ignored`, `unsupported`, and `dialect_assumed`; `ConstraintId`
    - Equality per 2.7: `required`, `enum`, `type`, and `dependentRequired` compared as sets; `$schema`, provenance, `x-provenance`, and loader records excluded
    - _Requirements: 2.7_
  - [x] 2.5 Implement the provenance data model (`core/provenance.py`)
    - `Support`, `CandidateScore`, `ProvenanceRecord`, `FallbackInfo` (disabled, missing-threshold, missing-model), `Cutoff`, `SampleInfo`, `RelaxationRecord`; Laplace default Confidence; fallback and Baseline_Rule markers (`baseline:type`, `baseline:required`); JSON serialization and deserialization
    - _Requirements: 26.2, 26.3, 26.4, 26.10, 26.13_
  - [x] 2.6 Implement the Inference_Report builder (`core/report.py`)
    - Sections from the design's report table; per-stage timer keyed by component name; `status` of ok, or failed with the stage and error; canonical serialization
    - _Requirements: 20.5, 27.8, 27.10_
  - [x] 2.7 Write property test for seeded sampling
    - **Property 50: Seeded uniform sampling**
    - **Validates: Requirements 26.4, 29.8**

- [x] 3. Implement configuration, the Technique registry, and the bundle reader
  - [x] 3.1 Implement the Technique registry (`config/registry.py`)
    - All 35 `TechniqueSpec` entries from the design table: id, area, tier, default state, `implemented=False`, served kinds, priority per kind, prerequisites (only `structure.hybrid_record_map` → `decide.mdl`), superlinear flag, sample parameter, thresholds read
    - Every `DecisionPointSpec` with its constant Fallback_Behavior (no Technique invoked) and any mandated tie rule
    - _Requirements: 28.1, 28.3, 29.8_
  - [x] 3.2 Implement Learned_Threshold and Resource_Parameter specs (`config/thresholds.py`, `config/resources.py`)
    - `ThresholdSpec` (technique, Decision_Point, objective name, admissible range, integer flag, grid) and `ThresholdRecord`; validation that flags missing, non-finite, and out-of-range values
    - `SettingSpec` tables with types, ranges, and the design's proposed Resource_Parameter defaults (`workers` defaults to `os.cpu_count()`)
    - _Requirements: 20.1, 20.2, 20.4, 20.6, 29.4, 29.11_
  - [x] 3.3 Implement the Run_Configuration (`config/run_config.py`)
    - `resolve(flags, file)` with flag > file > default and an origin per setting
    - One `ConfigError` lists every unknown setting, wrong type, out-of-range value (including sizes < 1), unregistered Technique id (with the registered list), enabled Technique with a disabled prerequisite, and enabled unimplemented Technique
    - `to_toml` with tomli-w and `from_toml` with tomllib
    - _Requirements: 27.2, 27.3, 27.4, 27.11, 28.2, 28.8, 28.9, 29.11_
  - [x] 3.4 Implement the Model_Bundle reader (`learn/bundle.py`)
    - Load the manifest, `thresholds.json`, and model files by identifier or path. Validate thresholds against their specs and return substitutions for the report. An unloadable bundle raises `ConfigError`
    - Ship an empty default bundle at the default identifier, so a default run completes with every fallback recorded
    - _Requirements: 20.6, 21.10, 27.2, 27.11_
  - [x] 3.5 Write property test for configuration round trip and precedence
    - **Property 44: Configuration round trip and precedence**
    - **Validates: Requirements 27.3, 27.4**
  - [x] 3.6 Write unit tests for registry completeness
    - 35 unique ids covering Table 1; one Fallback_Behavior per served Decision_Point; prerequisite and unimplemented-Technique errors
    - _Requirements: 28.1, 28.3, 28.8_

- [x] 4. Implement the Ingestor
  - [x] 4.1 Implement `parse_document` (`io/ingest.py`)
    - Strip one UTF-8 BOM, then try orjson. Fall back to stdlib `json` when orjson raises or returns an integral float with |x| ≥ 2^63
    - The stdlib path decodes strict UTF-8 and uses `parse_constant` to reject NaN and ±Infinity. Overflowing doubles and unpaired surrogates fail with explicit reasons
    - _Requirements: 1.4, 1.6, 1.11, 29.2_
  - [x] 4.2 Implement enumeration, line splitting, and the DocumentStore (`io/ingest.py`)
    - Inputs in argument order; `os.walk(followlinks=False)`, skipping symlinked files and keeping dotfiles; relative paths sorted by code point; extensions compared after `casefold()`
    - `.jsonl`/`.ndjson` split on LF with a trailing CR stripped and blank lines skipped; array unwrapping; `DocOrigin`
    - Record parse errors (path, line, reason), unreadable files, and the skipped count. Rejected supplied paths raise `ConfigError` before ingestion. Zero documents raise `NoDocumentsError`
    - `DocumentStore` of canonical bytes plus offsets; fixed-size chunks parsed with joblib, collected in submission order
    - _Requirements: 1.1, 1.2, 1.3, 1.5, 1.7, 1.8, 1.9, 1.10, 29.4_
  - [x] 4.3 Write property test for ingestion
    - **Property 1: Ingestion matches the reference model**
    - **Validates: Requirements 1.1, 1.2, 1.3, 1.5, 1.7, 1.9, 1.11**
  - [x] 4.4 Write property test for parsing fidelity
    - **Property 2: Parsing fidelity**
    - **Validates: Requirements 1.4, 1.6**

- [x] 5. Implement the Validator wrapper
  - [x] 5.1 Implement `Validator` (`io/validator.py`)
    - Draft class from the declared or assumed dialect; `validate_formats=True`; default regex engine; compiled validators cached by canonical-bytes digest; `CompileError` names the schema and the cause
    - `errors()` converts evaluation paths to absolute keyword locations by following `$ref` hops. Also `meta_errors`, plus `satisfies_format` and `matches_pattern` cached per distinct value
    - _Requirements: 3.2, 13.4, 29.3, 29.10_
  - [x] 5.2 Verify jsonschema-rs edge cases (open decision, M1)
    - Smoke tests (`tests/unit/test_validator_edge_cases.py`) cover integers beyond 64 bits as instances and in `enum`/`const`, overflowing doubles, and unpaired surrogates
    - Keep or revise the `parse_document` failure policy based on the results, and record the outcome in the test module docstring
    - _Requirements: 1.6, 1.11, 29.3_

- [x] 6. Implement the Schema_Emitter and Schema_Loader
  - [x] 6.1 Implement the Schema_Emitter (`io/emit.py`)
    - Fixed keyword rank order. `properties`, `patternProperties`, `$defs`, and `dependentRequired` keys, and `required`/`dependentRequired` values, in code-point order. Fixed `type` order and canonical `enum` order
    - Integral floats become `int`; `$schema` at the root only; check that every non-vocabulary keyword has the `x-` prefix
    - `layout()` returns the document plus the ConstraintId → JSON Pointer map, following the design's granularity table. `dumps()` uses the canonical encoder
    - _Requirements: 2.1, 2.2, 2.3, 26.5_
  - [x] 6.2 Implement the Schema_Loader (`io/load.py`)
    - Parse with `parse_document` and check the root kind. Detect the dialect, normalizing http/https and a trailing `#`, and record 2020-12 as assumed when needed. Meta-validate and raise `SchemaLoadError(check, line, col)` on failure
    - Load supported keywords, boolean schemas, and local `$ref`, including recursive refs. Keep annotations, the root `$id`, and `x-` keywords
    - Record ignored unknown keywords and unsupported constructs with their pointers, and mark the model partially supported when needed
    - _Requirements: 2.4, 2.6, 2.10, 2.11, 2.12_
  - [x] 6.3 Implement draft translation (`io/load.py`)
    - Rewrite pass with a pointer-rewrite map: `definitions`→`$defs`, array-form `items`→`prefixItems`, `additionalItems`→`items`, array `dependencies`→`dependentRequired`, draft-04 `id`→`$id`, and draft-04 boolean exclusive bounds
    - Rewrite local `$ref` through the map. Record as ignored the `$ref` siblings in drafts 04–07 and any `additionalItems` without array-form `items`
    - _Requirements: 2.5_
  - [x] 6.4 Write property test for schema emit/load round trip
    - **Property 3: Schema emit/load round trip** (generated Schema_Models now; Inferrer outputs added in 16.10, corpus schemas in 25.7)
    - **Validates: Requirements 2.4, 2.7, 2.8**
  - [x] 6.5 Write property test for emitted schema well-formedness
    - **Property 4: Emitted schemas are well-formed and canonical**
    - **Validates: Requirements 2.1, 2.2, 2.3**
  - [x] 6.6 Write property test for draft translation
    - **Property 5: Draft translation preserves verdicts** (generated older-draft schemas now; SchemaStore test documents added in 25.7)
    - **Validates: Requirements 2.5, 2.9, 34.5**
  - [x] 6.7 Write property test for loader accounting
    - **Property 6: Loader accounting**
    - **Validates: Requirements 2.6, 2.10, 2.11**
  - [x] 6.8 Write unit tests for loader errors
    - Non-JSON input with line and column, bad root kind, meta-schema failure
    - _Requirements: 2.12_

- [x] 7. Implement Naive_Merge
  - [x] 7.1 Implement `pipeline/naive_merge.py`
    - Union of types per view with the integer rule, union of keys under `properties`, one merged `items`, and `required` = keys present in every object; only `$schema`, `type`, `properties`, `items`, and `required`
    - Expose per-view Naive_Merge subschemas, which MDL local scoring and relaxation reuse
    - _Requirements: 28.5, 34.4_
  - [x] 7.2 Write unit tests for Naive_Merge
    - Keyword whitelist, the integer rule, and acceptance of every input document
    - _Requirements: 34.4_

- [x] 8. Checkpoint (M1): ensure all tests pass, ask the user if questions arise.

### M2: Decisions

- [x] 9. Implement the safe regex AST
  - [x] 9.1 Implement the AST, printer, and matcher (`values/patterns.py`)
    - Nodes for the five ASCII classes, literals, and quantifiers that apply only to classes or `(?:…)` groups; adjacent overlapping classes merged
    - Printer: escape only ECMA syntax characters, emit non-ASCII raw, write astral repeats as `(?:c){n}`, and anchor as `^(?:alt1|…|altk)$(?!\n)`
    - In-house matcher that returns the unique decomposition the MDL data bits need
    - _Requirements: 14.4, 14.5_
  - [x] 9.2 Write property test for pattern anchoring and dialect agreement
    - **Property 24: Pattern anchoring and dialect agreement** (Python `re`, `regress`, Validator)
    - **Validates: Requirements 14.4, 14.5**

- [x] 10. Implement the MDL_Scorer
  - [x] 10.1 Implement the primitive codes (`mdl/codes.py`)
    - `LN`, `LN0`, `LZ`, `UNI`, `SUB`, `CAT` (KT via `math.lgamma`), `STR`, `NUM`, `BOUND`, and the universal value code `U`
    - _Requirements: 19.1_
  - [x] 10.2 Implement model bits (`mdl/scorer.py`)
    - `M(node)` from the per-keyword table, plus `M_re` for safe-regex ASTs
    - _Requirements: 19.1_
  - [x] 10.3 Implement data bits and local scoring (`mdl/scorer.py`)
    - Exception cost `EXC`, type choice, `enum`/`const`, bounded integers and decimals, strings (length, Pattern tokens, per-Format alphabet constants, raw), booleans, Lists and Tuples, and `anyOf`/`oneOf` branch counts
    - Records: optional presence, `dependentRequired` conditioning, Field_Group presence, and closed or open extras. Maps: key count, keys, and values
    - Sums use `math.fsum` over per-distinct-value costs. Descendants are represented by their Naive_Merge subschemas. Returns `Score` with model, data, and exception bits, `n_rejected`, and `n_scored`
    - _Requirements: 19.1, 19.2, 19.6_
  - [x] 10.4 Write property test for the MDL_Scorer
    - **Property 29: MDL_Scorer is well-formed and invariant**
    - **Validates: Requirements 19.1, 19.2, 19.6**

- [x] 11. Implement the DecisionEngine
  - [x] 11.1 Implement `DecisionEngine.resolve` (`decide/engine.py`)
    - Resolution order: an enabled Decision_Model (the WS model when `decide.weak_supervision` is on and one exists, otherwise the synthetic model; equal probabilities go to MDL), then MDL argmin, then registry priority, then the Fallback_Behavior. Record a substitution when a model is missing
    - Tie chain: mandated tie rule, then fewer model bits, then priority, then `order_key`. Offer exception-bearing candidates only when `robust.mdl_exceptions` is on, with ties going to widening
    - Confidence is the MDL posterior or the calibrated probability. Record the method, every candidate with its score, the cut-off source and value, and any fallback indicator and reason in provenance and the report. Disabled Techniques produce no candidates
    - _Requirements: 19.3, 19.4, 19.5, 19.7, 20.7, 22.12, 22.13, 22.14, 23.8, 23.9, 26.3, 26.10, 28.4, 28.6, 28.7, 28.11, 28.12_
  - [x] 11.2 Write property test for the DecisionEngine
    - **Property 30: DecisionEngine resolution order** (stub Decision_Models, forced ties)
    - **Validates: Requirements 5.3, 6.2, 7.8, 9.4, 10.3, 13.8, 14.2, 17.2, 17.3, 19.3, 19.4, 19.5, 19.7, 22.12, 22.13, 23.8, 23.9, 24.7, 26.3, 28.6, 28.7**
  - [x] 11.3 Add the exception check and the Decision_Model comparison cut-off (`decide/engine.py`)
    - After a Decision_Model or priority step picks the widening winner, compare it in bits against eligible exception-bearing candidates while `robust.mdl_exceptions` is on; the best one replaces it only on a strictly smaller DL. Record per the design (method `mdl`, MDL posterior, "MDL comparison" cut-off, Flagged_Items, kept fallback indicator if it wins; compared DLs in details if not). No check at the constant Fallback_Behavior step
    - A Decision_Model argmax records a "Decision_Model comparison" cut-off with the calibrated probabilities and the Decision_Model artifact; equal probabilities keep the "MDL comparison" cut-off; no-cut-off resolutions keep `cutoff=None`
    - Update the engine docstring, the unit tests, and the Property 30 reference model. `decide.mdl` stays `implemented=False` here; task 13.1, whose Judgment_Calls are the first Decision_Points it serves, sets it to `implemented=True`
    - _Requirements: 17.2, 17.3, 20.1, 20.7, 22.12_

- [x] 12. Implement the core of the Value_Inferrer
  - [x] 12.1 Implement types and latent types (`values/types.py`, `values/latent.py`)
    - `type` lists exactly the observed types, with `integer` replacing `number` when every number has zero fraction
    - Latent_Type classifier (mixed integer and decimal strings count as decimal); `x-latent-type`; a `date`/`date-time`/`time` Format candidate when satisfied, otherwise a safe-AST Pattern candidate
    - _Requirements: 13.1, 13.2, 13.3, 13.9_
  - [x] 12.2 Implement Format detection (`values/formats.py`)
    - One Validator check per distinct value for each of the ten Formats; candidates for Formats that at least one string satisfies
    - _Requirements: 13.4, 13.9_
  - [x] 12.3 Implement enum estimation (`values/enums.py`)
    - n, S_obs, f1, f2, Good–Turing, and Chao1 under `enum` equality
    - Closed-or-Open-Enum through the DecisionEngine: the Decision_Model only while `values.enum_estimation` is on; MDL with the Good–Turing exception charge and ties to open; f1 = S_obs forces open; fallback open
    - Emit `const` or `enum`. Record the statistics in provenance (closed) or the report (open)
    - _Requirements: 15.1, 15.2, 15.3, 15.4, 15.5, 15.6, 15.7, 15.8_
  - [x] 12.4 Implement bound selection (`values/bounds.py`)
    - Candidate ladder: omit, observed extreme, 0, 1- and 2-significant-digit outward roundings, and integer type limits. Drop violated candidates, and negative candidates for lengths and counts
    - Decide the minimum first, then the maximum given the minimum; ties go to the candidate nearest the extreme; fallback omit
    - `decide_bound` serves numbers, code-point string lengths (skipped with a selected Format or a Closed_Enum), item counts, and key counts
    - _Requirements: 16.1, 16.2, 16.3, 16.6, 16.7, 16.8_
  - [x] 12.5 Implement `ValueInferrer` orchestration (`values/inferrer.py`)
    - `infer_scalar` in design order: type, enum, string candidates, withholding of rejecting candidates (reported with counts), MDL selection with alternatives in provenance, then bounds. Steps for unimplemented Techniques are skipped by registry state
    - `key_constraint` emits a Map-key Format under `propertyNames`
    - `type` records name the Baseline_Rule marker `baseline:type`, with the Laplace Confidence and no resolution
    - _Requirements: 4.4, 13.6, 13.8, 13.9, 26.13_
  - [x] 12.6 Write property test for scalar typing
    - **Property 22: Scalar typing**
    - **Validates: Requirements 13.1, 13.2, 13.3, 13.4**
  - [x] 12.7 Write property test for enum statistics
    - **Property 25: Enum statistics and closure rules**
    - **Validates: Requirements 15.1, 15.3, 15.5**
  - [x] 12.8 Write property test for bounds
    - **Property 26: Bounds come from the candidate ladder**
    - **Validates: Requirements 16.2, 16.3, 16.4, 16.5, 16.7**
  - [x] 12.9 Write property test for value Constraint soundness
    - **Property 23: Value Constraints are sound** (scalar and key views under random `run_configs()`)
    - **Validates: Requirements 13.6, 13.8, 14.1, 14.3, 14.7, 15.3, 16.6, 24.5**

- [x] 13. Implement the Structure_Analyzer components
  - [x] 13.1 Implement Record-or-Map and Closed-or-Open-Record (`structure/record_map.py`)
    - MDL Record and Map candidates with ties to Record; fallback Record. A view with no keys becomes a Record with no `properties`, and provenance records that no keys were available
    - A Map emits one `additionalProperties` value schema, `propertyNames` from `key_constraint`, and `minProperties`/`maxProperties` from `decide_bound`
    - Closed-or-Open: MDL with the Good–Turing unseen-key charge; fallback open; closed emits `additionalProperties: false`
    - Set `decide.mdl` to `implemented=True` (enabled by default), so validated runs resolve Judgment_Calls by MDL instead of only by their Fallback_Behaviors
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.7, 4.8, 4.9, 4.10, 4.11, 16.5_
  - [x] 13.2 Implement Variant grouping (`structure/variants.py`)
    - Weighted, canonically sorted Object_Signatures
    - `minhash_lsh`: datasketch seeded from the rng, edges at τ = `lsh.keyset_jaccard`, connected components, then greedy merging while DL decreases along the links of the maximum spanning forest of the LSH-retrieved pairs, strongest first, in passes until one merges nothing. With `lsh.bands` = 1, pairs with equal sketches are the candidates
    - `dbscan` on the precomputed Jaccard matrix with `sample_weight` = signature counts, so `dbscan.min_samples` counts objects; `dbscan.eps` is the recorded cut-off and `min_samples` goes in the grouping details
    - `hdbscan` on the same matrix without weights (scikit-learn's `HDBSCAN.fit` takes no `sample_weight`), so `hdbscan.min_cluster_size` counts signatures
    - Both sample distinct signatures when superlinear (`sample.dbscan`, `sample.hdbscan`); each unsampled signature takes the label of its nearest sampled signature. Noise is grouped by signature into the lowest-DL group
    - `bernoulli_mixture`: weighted EM with BIC over k = 1…min(#signatures, `variants.max_components`)
    - Select by MDL or by priority; fallback one Variant
    - _Requirements: 5.1, 5.2, 5.7, 5.8, 5.9, 5.10, 29.8_
  - [x] 13.3 Implement Merge-or-Split and branch emission (`structure/variants.py`)
    - Merge-or-Split through the DecisionEngine. With both the Decision_Model and MDL disabled, keep the priority-selected grouping split
    - Merge-or-Split is binary: all candidate Variants merge into one, or all stay split. Partial merges are left to the grouping step
    - One `anyOf` branch per Variant listing only observed fields; `oneOf` when every pair of branches is disjoint per 5.5. A merged Variant gets the union of fields, the intersection as `required`, and the union of types
    - Disjointness reads only the branches' own keywords (design, Open Decisions); anything it cannot read counts as overlapping, so the branches stay under `anyOf`
    - _Requirements: 5.3, 5.4, 5.5, 5.6, 5.11_
  - [x] 13.4 Implement Tuple-or-List and array nodes (`structure/arrays.py`)
    - MDL Tuple vs List with ties to List (mandated tie rule, 7.10); fallback List. A List emits `items` over all elements. A Tuple emits `prefixItems` from position-filtered views, plus `items: false` when no array is longer
    - Empty-only Paths emit neither keyword and say so in provenance; `minItems`/`maxItems` come from `decide_bound`; the outcome and its Confidence go in provenance
    - Array lengths count flagged elements, so the prefix spans the longest array; a position whose elements are all flagged gets the subschema of an empty view. Arrays holding only flagged elements count as empty-only
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.9, 7.10, 16.4_
  - [x] 13.5 Write property test for Variant partitions
    - **Property 11: Variant partition invariant**
    - **Validates: Requirements 5.1, 5.2, 5.9**

- [x] 14. Implement the Dependency_Miner core
  - [x] 14.1 Implement presence and required fields (`deps/presence.py`)
    - Packed presence bitsets per field. Base_Fields per Variant and per family. `required` = fields present in every non-flagged object (including `null` values). Each `required` entry records the Baseline_Rule marker `baseline:required` with the Laplace Confidence, with Support counted over the Variant's own non-flagged objects
    - _Requirements: 6.1, 6.5, 9.1, 9.2, 9.7, 26.13_
  - [x] 14.2 Implement co-occurrence mining (`deps/cooccurrence.py`)
    - Candidates from antecedent ⊆ consequent bitset tests. Discard and report candidates whose support is below `cooccur.min_support`
    - Select by MDL, or by `cooccur.min_confidence` when MDL is off. Canonical transitive reduction with SCCs collapsed to code-point cycles
    - Emit `dependentRequired`; fallback no rules
    - Each candidate is its own `cooccurrence_rule` Decision_Point, compared against the Variant subschema without it. Score candidates by exchanging the Record's presence terms (`MDLScorer.presence_bits`) instead of rescoring the subschema
    - A candidate's Confidence is `(count(a) + 1) / (count(a) + 2)`. Both `cooccur.min_support` and `cooccur.min_confidence` must be in the Model_Bundle for the Technique to be in effect, even under MDL. Set `rules.cooccurrence` to `implemented=True`
    - _Requirements: 9.3, 9.4, 9.5, 9.6, 9.8, 9.9_
  - [x] 14.3 Write property test for mining primitives
    - **Property 17: Mining primitives equal brute-force references** (LSH groups and Co_Occurrence_Rule candidates now; extended in 33.3, 35.3, 44.2)
    - **Validates: Requirements 5.7, 9.3**

- [x] 15. Implement the soundness check and relaxation
  - [x] 15.1 Implement the relaxation table (`decide/relax.py`)
    - Widening candidates per failing keyword, following the design table; Naive_Merge subschemas for new properties and extra elements
    - `not` contents are never edited (only the whole `not` can be removed). Any edit inside a `oneOf` branch converts that `oneOf` to `anyOf`. `$defs` edits apply at every referencing site
    - `if` contents are treated like `not` contents (the `if` is removed with its `then` and `else`). An edit that would reach inside a `not` or an `if` removes that keyword as part of the candidate. Removal is offered last for every failing keyword, whole `anyOf`/`oneOf` included; `properties`, `patternProperties`, and `prefixItems` are never removed
    - A `oneOf` converted beside an existing `anyOf` goes into a new `allOf` entry, and its Change lists the owner's `allOf` Constraint in `after_ids` (and in `before_ids` when the `allOf` already existed)
    - _Requirements: 3.3, 3.5_
  - [x] 15.2 Write property test for relaxation monotonicity
    - **Property 8: Relaxation monotonicity**
    - **Validates: Requirements 3.5**
  - [x] 15.3 Implement the soundness loop (`pipeline/soundness.py`)
    - Compile the covering schema; validate every non-flagged document in parallel chunks, including documents left out of samples; map errors to ConstraintIds
    - Process failures in document-index and ConstraintId order. Resolve through the `relaxation` Decision_Point (MDL over the Path's Observations plus the failing values, otherwise candidate order). Record RelaxationRecords and recompute Support
    - Stop at `soundness.max_passes`, then run the removal loop. Write the report counts
    - Passes 1 to `max_passes − 1` are followed by relaxation; if pass `max_passes` still fails, run the removal loop, so `max_passes = 1` only removes. `passes` counts every validation, the removal loop's included
    - Count per change record: `relaxations` includes the removal loop's `oneOf`-conversion guard records, so change records number `relaxations + removals`. Removing a whole `anyOf`/`oneOf` is one removal, and replacing a `false` subschema with `true` is a removal
    - Score candidates on the instances the failing node is evaluated on (a walk of the validated documents along the schema, plus the failing values). Recompute the Support of changed Constraints once, at the end of the stage, and leave Confidence to 17.1
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.6, 3.7_

- [x] 16. Wire the Structure_Analyzer walk and the inference pipeline
  - [x] 16.1 Implement the analyzer walk (`structure/analyzer.py`)
    - `StructureAnalyzer.analyze(view)` dispatches scalars, objects (record_map → closed/open → variants → merge/split → per-branch recursion → presence and co-occurrence), and arrays
    - Mixed types: one object branch coexists in a single schema object; two or more object branches become an `anyOf` plus a branch for the remaining types
    - `type` on object, array, and mixed nodes records the Baseline_Rule marker `baseline:type`
    - A closed Record with two or more Variant branches puts `additionalProperties: false` inside each branch, never beside the `anyOf`/`oneOf`
    - A Closed_Enum beside arrays or objects is its own `anyOf` branch; mixed nodes are always `anyOf`. Added branches take the Merge-or-Split or Closed-or-Open-Enum decision's record (design, Open Decisions). The key Format inside `propertyNames` shares the `propertyNames` record
    - _Requirements: 4.2, 4.3, 5.4, 7.2, 7.3, 9.7, 26.13_
  - [x] 16.2 Implement pipeline orchestration (`pipeline/infer.py`)
    - `InferenceContext`; stages config → bundle → ingest → flatten → analyze → soundness → emit layout, each timed under its component name
    - With the outlier and change-point Techniques unimplemented or disabled, record zero Flagged_Items and "change-point detection not applied"
    - Errors become `InferenceError(stage, partial_report)`. `NoDocumentsError` for empty input
    - In-memory documents are ingested as the lines of a virtual `.jsonl` file (design, Open Decisions); `infer_documents(documents, config)` returns a `PipelineResult` for full-run tests
    - _Requirements: 1.8, 3.2, 17.6, 18.10, 27.7, 27.8, 27.10, 27.13_
  - [x] 16.3 Write property test for object nodes
    - **Property 10: Object nodes match their classification** (hybrid clause gated on `structure.hybrid_record_map`, active after 37.1)
    - **Validates: Requirements 4.2, 4.3, 4.4, 4.5, 4.6, 4.8, 4.9**
  - [x] 16.4 Write property test for Variant branches
    - **Property 12: Variant branch emission**
    - Judge disjointness by the 5.5 reading in the design's Open Decisions (`pairwise_disjoint`), so the `oneOf` "iff" matches the emitter
    - **Validates: Requirements 5.4, 5.5, 5.6**
  - [x] 16.5 Write property test for array nodes
    - **Property 14: Array nodes match their classification**
    - **Validates: Requirements 7.2, 7.3, 7.4, 7.9**
  - [x] 16.6 Write property test for required fields
    - **Property 18: Required means always present**
    - **Validates: Requirements 6.1, 9.1, 9.2, 9.7**
  - [x] 16.7 Write property test for emitted rules
    - **Property 19: Emitted rules hold and are irredundant** (checks are generic over emitted rule keywords and annotations, so later Techniques are covered automatically)
    - **Validates: Requirements 9.5, 9.6, 11.5, 11.6, 12.6, 12.7, 12.8**
  - [x] 16.8 Write property test for soundness
    - **Property 7: Soundness on non-flagged documents**
    - **Validates: Requirements 3.1, 3.2, 3.3, 3.7, 8.3**
  - [x] 16.9 Write property test for soundness bookkeeping
    - **Property 9: Soundness bookkeeping** (with injected over-tight Constraints)
    - **Validates: Requirements 3.4, 3.6**
  - [x] 16.10 Extend the round-trip property test to Inferrer outputs
    - **Property 3: Schema emit/load round trip** (Schema_Models produced by `pipeline.infer`)
    - **Validates: Requirements 2.7**

- [x] 17. Implement provenance finalization and the explanation file
  - [x] 17.1 Implement provenance finalization (`core/provenance.py`, `pipeline/infer.py`)
    - After soundness, compute Support for every Constraint over non-flagged Observations. Totals for Constraints inside `$defs` cover every referencing Path
    - Apply the default Confidence where no alternative exists. Ensure exactly one record per Constraint, including fallback- and Baseline_Rule-produced ones
    - Refresh Laplace default Confidences from the recomputed Support, since the soundness loop (15.3) updates Support but not Confidence
    - Reuse the soundness check's walk (`pipeline.soundness.count_instances`) for Support, run finalization inside the `Soundness` stage, and treat a missing or orphaned record as an internal error (`ProvenanceError`). Give Constraints inside subschemas a 3.3 relaxation creates the relaxation decision's record. Default and kept Confidences per the design's Open Decisions (`KIND_SPECIFIC_CONFIDENCE`)
    - _Requirements: 26.1, 26.2, 26.4, 26.10, 26.13_
  - [x] 17.2 Implement the explanation file and inline provenance (`io/emit.py`, `io/load.py`)
    - `explanation.json` = `{"format": 1, "records": {pointer: record}}`, sorted and written with the canonical encoder; `load_explanation` reads it back
    - Optional `x-provenance` per schema object, matching the explanation entries
    - `x-provenance` maps pointers relative to its schema object to records; the setting is `output.inline_provenance`. Records and emitted Constraints must correspond one to one, or `EmitError`
    - _Requirements: 26.5, 26.6, 26.7, 26.8_
  - [x] 17.3 Implement explain rendering (`core/provenance.py`)
    - `explain(schema, explanation, pointer)` resolves the pointer and prints every record field as plain text. Typed errors cover unreadable or unparseable inputs, invalid pointers, unresolved pointers, and missing entries
    - `explain` lives in `io/explain.py` and is re-exported from `core.provenance`; it returns the text. Paths are files, `bytes`/`str` are content. A pointer to a schema object is an error that lists its Constraints
    - _Requirements: 26.9, 26.12_
  - [x] 17.4 Write property test for provenance completeness
    - **Property 41: Provenance completeness**
    - **Validates: Requirements 4.1, 4.7, 7.1, 15.8, 26.1, 26.2, 26.13**
  - [x] 17.5 Write property test for explanation file integrity
    - **Property 42: Explanation file integrity**
    - **Validates: Requirements 26.5, 26.7, 26.8**
  - [x] 17.6 Write property test for annotation and report neutrality
    - **Property 43: Annotation and report neutrality** (nearest-schema clause gated on `corpus.nearest_schema`, active after 40.4)
    - **Validates: Requirements 25.5, 26.6, 26.11**

- [x] 18. Implement the public API and the CLI
  - [x] 18.1 Implement the public API (`src/jsinfer/__init__.py`)
    - `infer`, `infer_files`, `load_schema`, `emit_schema`, `InferenceResult`, and the exception types. `infer()` canonically serializes documents into the DocumentStore, so later stages are shared with the CLI
    - `InferenceResult` carries the explanation file (`SchemaEmitter().explanation(model, pointers)`) and the schema bytes honoring `output.inline_provenance`
    - Decided: non-JSON Python values raise `TypeError` (`ValueError` for a container that contains itself) and are never converted; NaN, infinities, and unpaired surrogates are skipped and recorded as in a `.jsonl` file (design Open Decisions, In-memory documents)
    - Also exports `explain`, `load_explanation`, and `RunConfiguration`, lazily (PEP 562). The schema file is `schema.json`; `versions`, `combined`, and `diffs` stay empty until 48.2 (design Open Decisions, Public API surface)
    - _Requirements: 27.9, 27.13_
  - [x] 18.2 Implement the CLI (`cli.py`)
    - `infer INPUT... [-c] [--set] [--technique] [--out]`: writes the schema, `explanation.json`, `report.json`, and `effective-config.toml` atomically to a temp directory under `--out`, overwriting without prompts and deleting the temp directory on failure. Exit codes 0, 1, 2, and 3
    - `explain`, `list-techniques` (reads no data), and `list-thresholds [--bundle]`
    - `infer` runs `jsinfer.infer_files` and writes `InferenceResult.files()` (`schema.json` with `x-provenance` when `output.inline_provenance` is on, `explanation.json`, `report.json`, `effective-config.toml`; the pipeline's `Schema_Emitter` stage produces them since 18.1) inside `core.determinism.atomic_outputs`
    - `explain` wraps its file arguments in `pathlib.Path` (a `str` is content to `io.explain.explain`), prints the returned text to stdout, and on `ExplainError` prints its message to stderr and exits 2
    - Exits 1 and 3 replace only `report.json` (the partial report) and keep an earlier run's other files; exit 2 writes nothing. `--set` values are TOML except the string settings, `--technique` is its shorthand, and the last value wins (design Open Decisions, CLI failed runs and CLI settings flags)
    - _Requirements: 1.8, 1.10, 20.2, 26.9, 26.12, 27.1, 27.3, 27.7, 27.10, 27.11, 28.10_
  - [x] 18.3 Write property test for invalid configurations
    - **Property 45: Invalid configurations stop before ingestion**
    - For the API, "exits non-zero" is `RunConfiguration.resolve` raising `ConfigError` before `infer`/`infer_files` reads a document. Found and fixed: `--technique` with an unregistered id and a switch other than on/off reported only the switch (design Open Decisions, Invalid configurations)
    - **Validates: Requirements 27.11, 28.9, 29.11**
  - [x] 18.4 Write CLI integration tests
    - End-to-end run, API/CLI equality, closed stdin, explain error paths, `list-techniques` and `list-thresholds` output
    - Closed stdin means no descriptor 0 at all, besides a terminal, `/dev/null`, and an open pipe. Found and fixed: loky workers died with descriptor 0 closed, so `cli.main` now reserves it (design Open Decisions, Standard input)
    - _Requirements: 26.9, 26.12, 27.1, 27.9, 28.10_

- [x] 19. Implement reproducibility plumbing and the Subschema_Checker
  - [x] 19.1 Implement run metadata and worker determinism (`core/report.py`, `pipeline/infer.py`)
    - Report `run` section: effective config, seed, bundle id, jsinfer and CPython versions, and locked package versions via `importlib.metadata`
    - Fill what 16.2 left out: `RunInfo.packages` (empty now) and the run timestamps. Record in the report every Resource_Parameter the components read, with its origin (20.5): `minhash.perm`, `lsh.bands`, `sample.*`, and `variants.*` in the VariantAnalyzer, among others; 16.2 records only the pipeline's own reads (`workers`, `ingest.chunk_size`, `soundness.max_passes`)
    - Work units defined by data, not worker count; `threadpoolctl` limits workers to one OpenMP thread; timings and timestamps only in exempt sections
    - Decided: the API copes with descriptor 0 closed too, through a reservation scoped to multi-worker runs (`core.workers.reserved_stdin`; design Open Decisions, Standard input)
    - Packages are the runtime closure of `uv.lock`, a static tuple checked against the lock file by a test; tasks run with OpenMP and BLAS limited to one thread; a component lists a Resource_Parameter only when its Technique runs (design Open Decisions, Run metadata and Resource_Parameter and Learned_Threshold reads; Determinism and reproducibility)
    - _Requirements: 27.5, 27.8, 27.12, 29.4_
  - [x] 19.2 Implement the Subschema_Checker (`io/subschema.py`)
    - Strip annotations and `x-` keywords; 2020-12→draft-04 shim; initial unsupported list from Open Decisions (`format`, `contains`, `propertyNames`, `if`/`then`/`else`, non-scalar `uniqueItems`, recursive `$ref`)
    - Run `isSubschema` in a child process with `subschema.timeout_s`, inside an isolated uv tool environment if pins conflict. Map outcomes to equivalent, subset, superset, incomparable, or `Unsupported` with its cause
    - Decided: jsonsubschema 0.0.8 is pinned in the project environment (no isolated environment). `dependentRequired`/`dependentSchemas` are unsupported rather than shimmed to `dependencies`, which jsonsubschema ignores; unknown and earlier-draft keywords are unsupported; `unavailable` is a fourth cause (design Open Decisions, jsonsubschema keyword coverage)
    - _Requirements: 28.5, 31.7, 31.8_
  - [x] 19.3 Write property test for inclusion relation mapping
    - **Property 54: Inclusion relation mapping** (stubbed checker)
    - **Validates: Requirements 31.7, 31.8**
  - [x] 19.4 Write property test for byte-identical reproducibility
    - **Property 46: Byte-identical reproducibility** (subprocesses with workers ∈ {1, 2, 4} and varied `PYTHONHASHSEED`)
    - Each example runs in three long-lived driver subprocesses (hash seed 0, 1, and `random` with 1, 2, and 4 workers and different output directories), not a 3×3 grid (design, Determinism and reproducibility). Added `src/jsinfer/__main__.py`, so `python -m jsinfer` runs the CLI
    - **Validates: Requirements 27.5, 27.12**
  - [x] 19.5 Write property test for order-insensitivity
    - **Property 47: Order-insensitivity**
    - The explanation files and reports are compared after mapping the document indices of relaxation and removal records (3.6), which name documents by position, to one order; everything else must be equal as it is
    - **Validates: Requirements 27.6**
  - [x] 19.6 Write property test for Technique isolation
    - **Property 48: Technique isolation**
    - Enabled Techniques that are not in effect get the same checks, with reason `missing-threshold` or `missing-model`; fallback indicators may name disabled Techniques; Decision_Points are observed through the report's decision entries (design Open Decisions, Techniques not in effect and fallback indicators)
    - **Validates: Requirements 26.10, 28.2, 28.4, 28.11, 28.12**
  - [x] 19.7 Write property test for all-off equals Naive_Merge
    - **Property 49: All Techniques off equals Naive_Merge**
    - Uses the real jsonsubschema; an `Unsupported` result fails the example whatever its cause
    - **Validates: Requirements 28.5, 34.4**
  - [x] 19.8 Write property test for cut-off provenance
    - **Property 31: Cut-off provenance**
    - Check that the report lists every Resource_Parameter a run read, components' reads included (20.5)
    - Reads are observed independently of the report, through tagged values from every value source. Checking that a Learned_Threshold is present is not a read; BIC is the only model-selection criterion; the Decision_Model cut-off checks apply once 22.4 makes them reachable (design Open Decisions, Resource_Parameter and Learned_Threshold reads)
    - **Validates: Requirements 20.1, 20.5, 20.7**
  - [x] 19.9 Write unit tests for configuration-specific fallbacks
    - 4.10, 5.10, 5.11, 7.10, 9.9, 15.7, 16.8, plus missing-threshold substitution (20.6)
    - The Decision_Model branches of 4.10, 5.11, 7.10, and 15.7 are covered at the engine and component level only, until 22.4; the 9.9 test asserts no `dependentRequired` at all, until 33.1
    - _Requirements: 4.10, 5.10, 5.11, 7.10, 9.9, 15.7, 16.8, 20.6_
  - [x] 19.10 Make the soundness processing order independent of document order (`pipeline/soundness.py`, `decide/relax.py`)
    - Order each pass's failure groups, and the removal loop's, by target only (node id, keyword, item in code-point order) instead of first document index then target. Keep scoring each relaxation against the model as relaxed so far (design Open Decisions, Soundness processing order)
    - Confirm that node ids depend only on the model, not on document order; otherwise order by the target's emitted JSON Pointer, then keyword and item
    - Update the `pipeline/soundness.py` and `decide/relax.py` docstrings and the order assertion in `tests/unit/test_soundness.py`. Add a regression test with the 19.5 counterexample (23 lists, `maxItems: 0` and `minItems: 2` injected at the root) that gives the same model in both document orders
    - Lift Property 47's one-Constraint limit: inject several over-tight Constraints, and update its docstring
    - _Requirements: 3.3, 3.7, 27.6_

- [x] 20. Checkpoint (M2): ensure all tests pass, ask the user if questions arise.

### M3: Learning

- [ ] 21. Implement normalization, the Synthetic_Generator, and labels
  - [ ] 21.1 Implement field-name normalization and Path sets (`corpus/normalize.py`)
    - `normalize_name` splits on `_`, `-`, and lower/digit→upper transitions, then casefolds
    - Normalized Path sets across branches, with `$ref` expanded to depth `nearest.ref_depth`
    - _Requirements: 24.2, 25.1_
  - [ ] 21.2 Write property test for field-name normalization
    - **Property 38: Field-name normalization**
    - **Validates: Requirements 24.2**
  - [ ] 21.3 Implement the Synthetic_Generator (`learn/synth.py`)
    - Per-document streams from `SeedSequence([seed, blake2b(schema_id), k, doc_index])`; sampled collection parameters recorded per collection
    - Value sources in 22.3 order: generators for the ten Formats; `examples`/`default`/`enum` from the field or from same-name corpus fields; then type-based values. Pattern sampler over `re._parser` after ECMA translation, with rejection fallback
    - `$ref` depth cap `synth.max_depth`; rejection sampling for `allOf`, `oneOf`, `not`, and `if`; every document validated; attempt budget `synth.doc_attempts`; uncompilable schemas excluded with a reason
    - For source (b) of 22.3, index `examples`, `default`, and `enum` values by normalized field name across the training-split and validation-split schemas the generator is given, never held-out ones; the train-only Corpus_Index cannot serve this (design, Generator)
    - _Requirements: 22.1, 22.2, 22.3, 22.7, 22.8, 22.9_
  - [ ] 21.4 Write property test for the generator
    - **Property 33: Generator correctness and determinism**
    - Also add the planted-structure `collections()` strategy (Maps, Variants, Tagged_Unions, Field_Groups, Tuples, recursion, rare outliers) to `tests/strategies.py`
    - **Validates: Requirements 22.7, 22.9**
  - [ ] 21.5 Implement label derivation and Path alignment (`learn/labels.py`)
    - `derive_labels` on the `$ref`-expanded model with Key, MapVal, and Elem stepping per 22.4. Missing labels and conflicting labels across branches become unknown
    - `align(schema, path)` maps inferred Paths to schema Paths
    - _Requirements: 22.4, 22.5, 31.9, 31.10_
  - [ ] 21.6 Write property test for label derivation
    - **Property 34: Label derivation is consistent**
    - **Validates: Requirements 22.4, 22.5, 31.9**

- [ ] 22. Implement featurization, Decision_Models, and threshold fitting
  - [ ] 22.1 Implement the featurizers and wire them in (`learn/features.py`, `structure/record_map.py`, `structure/arrays.py`, `structure/variants.py`, `values/enums.py`)
    - Per-kind feature tables from the design, the normalized MDL difference, hashed name and parent tokens, and a versioned feature spec
    - Pass a `features` callable to the DecisionEngine at every Judgment_Call site
    - Make the Substitution for a missing grouping threshold name what actually resolves `variant_grouping`: the grouping Techniques still in effect, or the one-Variant Fallback_Behavior when none is (design Open Decisions, Techniques not in effect and fallback indicators)
    - _Requirements: 15.2, 22.10_
  - [ ] 22.2 Implement oracle-resolver featurization (`learn/trainer.py`)
    - Run the real pipeline with an oracle resolver: upstream Judgment_Calls take the ground-truth label when known and MDL otherwise. Record features and labels per Judgment_Call, aligned with `align()`
    - Exclude unknown-label and no-Observation examples and count them per kind and per cause
    - _Requirements: 22.6, 22.10_
  - [ ] 22.3 Write property test for feature parity
    - **Property 35: Feature parity**
    - **Validates: Requirements 22.10**
  - [ ] 22.4 Implement Decision_Models (`learn/decision_models.py`)
    - `HistGradientBoostingClassifier(random_state=…, early_stopping=False)`, calibrated with `CalibratedClassifierCV(FrozenEstimator(m), method="sigmoid")` on validation examples
    - Confirm the closed Closed-or-Open-Record clauses of Properties 10 and 41 run once a Decision_Model can select a closed Record; MDL does not close Records on their inputs (even at 1,000 documents)
    - Report whether the normalized MDL-difference feature carries signal for Closed-or-Open-Record (its spread and importance on validation examples). Revisit the Good–Turing unseen-key charge only if it does not and the Decision_Model cannot close Records either (design Open Decisions, Closed-or-Open-Record Judgment_Call)
    - Omit a kind with only one class and record the per-class counts. Validation metrics: n, accuracy, macro-F1, and ECE before and after calibration with `ece.bins`
    - When the `decide.dm_*` Techniques become `implemented=True`, add pipeline-level tests of the Decision_Model branches of 4.10, 5.11, 7.10, and 15.7 (an enabled model, a missing model, `decide.dm_closed_enum` on with `values.enum_estimation` off) to `tests/unit/test_config_fallbacks.py`, and check that Properties 31 and 48 reach Decision_Model cut-offs and fallbacks
    - _Requirements: 21.3, 21.4, 21.5, 22.11_
  - [ ] 22.5 Implement Learned_Threshold fitting (`learn/trainer.py`, `config/thresholds.py`)
    - Deterministic grid search on validation examples, ties to the smallest value. Store the fitted value, objective value, example count, and Training_Data id. Omit thresholds with no validation examples and record why
    - Objectives for the M2 thresholds: ARI for `lsh.keyset_jaccard`, `dbscan.eps`, `dbscan.min_samples`, and `hdbscan.min_cluster_size`; `dependentRequired` F1 for `cooccur.min_support` and `cooccur.min_confidence`. Fit `dbscan.min_samples` in objects and `hdbscan.min_cluster_size` in distinct signatures, the units task 13.2 uses. Fit `cooccur.min_confidence` on the Confidence `(count(a) + 1) / (count(a) + 2)` of task 14.2, which makes it a minimum Support count
    - _Requirements: 20.3, 20.4, 20.8_

- [ ] 23. Implement the Model_Bundle writer and the Trainer
  - [ ] 23.1 Implement the bundle writer (`learn/bundle.py`)
    - Bundle layout, manifest fields, `thresholds.json`, `feature_spec.json`, and models, written atomically (temp directory, then rename)
    - The reader treats a feature-spec mismatch as a missing model
    - Record `artifacts {name: {path, sha256}}` in the manifest. The reader verifies the digest before unpickling or reading an artifact and treats a missing file or a mismatch as a missing artifact (substitution cause `checksum`). Document that only trusted bundles should be loaded, since the digest does not make an untrusted pickle safe
    - _Requirements: 21.1, 21.2, 21.10, 22.10_
  - [ ] 23.2 Implement `Trainer.run` and `jsinfer train` (`learn/trainer.py`, `cli.py`, `bench/splits.py`)
    - Define the `SchemaSplit` file model (load and save) and a minimal corpus manifest file model (schema id, source, schema file path relative to the manifest) in `bench/splits.py`; 25.1 later writes the same model. Read train and validation schemas by id from the corpus manifest named in the training config (design Open Decisions, Corpus manifest and training fixtures)
    - Register the Resource_Parameter `train.max_schemas` (default 2,000, provisional until 29.9): above it, sample each split's sorted schema ids in proportion to its size with `rng("train.subset", split)`, and list the schemas used per split in the manifest
    - Run the design's training order. Reject held-out or unknown schemas with a non-zero exit and no bundle
    - Omit artifacts of disabled or unimplemented Techniques (label model, Semantic_Type_Classifier, Corpus_Index) and record the reasons. Stage errors remove the temp directory
    - _Requirements: 21.1, 21.2, 21.3, 21.7, 21.8, 21.9, 22.1, 22.6_
  - [ ] 23.3 Write property test for missing artifacts
    - **Property 32: Missing artifacts degrade to fallbacks**
    - Include artifacts whose SHA-256 does not match the manifest
    - **Validates: Requirements 20.6, 21.10, 22.14, 24.9**
  - [ ] 23.4 Hand-write the fixture schema corpus (`tests/fixtures/corpus/`)
    - 30 draft 2020-12 schemas written for the tests, with no SchemaStore or JSONSchemaBench content (design Open Decisions, Corpus manifest and training fixtures)
    - Cover every Judgment_Call label of 22.4 in both classes (Record and Map, closed and open Record, Tuple and List, Closed_Enum and open enum, Split and Merge) in both the training and the validation split, so `tiny_bundle` trains a Decision_Model per kind. Also cover the ten Formats, `examples`/`default`/`enum` values, normalized field names shared across schemas (22.3(b)), `$ref` with recursion, `allOf`/`oneOf`/`not`/`if`, and Patterns
    - Add a corpus manifest in the 23.2 file model and a fixture Schema_Split with training, validation, and held-out schemas, so 23.5 can test held-out rejection (21.8)
    - A unit test checks that every schema passes the meta-schema check and compiles with the Validator, that the manifest and split agree, and that `derive_labels` (21.5) finds both classes of every kind in the training and validation splits
    - _Requirements: 21.8, 21.9, 22.3, 22.4_
  - [ ] 23.5 Write Trainer integration tests and fixtures
    - The fixture corpus and split of 23.4; session-scoped `tiny_bundle` cached by content hash; `empty_bundle`; two-run reproducibility; held-out schema rejection
    - _Requirements: 21.7, 21.8, 21.9_

- [ ] 24. Checkpoint (M3): ensure all tests pass, ask the user if questions arise.

### M4: Evaluation Harness

- [ ] 25. Implement corpus loading, splits, and leakage checks
  - [ ] 25.1 Implement the corpus loader (`bench/corpus_loader.py`)
    - Read a SchemaStore checkout at a pinned commit (schemas, positive and negative tests, catalog versions) and a JSONSchemaBench checkout at a pinned revision from configured local paths
    - Assign unique ids with source and snapshot, and link test files. Omit unreadable or invalid files with reasons. Write the corpus manifest the Trainer reads, in the file model of 23.2, extended with test-file paths and catalog versions
    - _Requirements: 30.1, 30.9_
  - [ ] 25.2 Implement the split builder and `jsinfer split` (`bench/splits.py`, `cli.py`)
    - MinHash/LSH over normalized Path sets with exact Jaccard on candidate pairs; cut-off from a 1–3 component GMM on logit similarities selected by BIC; union-find over catalog versions and pairs above the cut-off
    - Assign forced held-out groups first, then the seeded deficit assignment. Persist everything 30.4 lists, including deviation flags
    - Exclude schemas from synthetic generation per 30.5. Reject invalid proportions without writing a split
    - _Requirements: 30.2, 30.3, 30.4, 30.5, 30.10_
  - [ ] 25.3 Write property test for split integrity
    - **Property 51: Split integrity**
    - **Validates: Requirements 30.2, 30.3, 30.4, 30.10**
  - [ ] 25.4 Implement leakage checks (`bench/leakage.py`)
    - Before any run, compare the training manifest with held-out groups, Real_Dataset collections, and loaded ids. Abort on overlap, a missing manifest, or unknown ids, listing every offender
    - _Requirements: 30.7, 30.8, 30.11_
  - [ ] 25.5 Write unit tests for leakage aborts
    - _Requirements: 30.7, 30.8, 30.11_
  - [ ] 25.6 Probe jsonsubschema keyword coverage (open decision, M4)
    - Integration test (`tests/integration/test_subschema_probe.py`) with known inclusion pairs per keyword
    - Update the supported-keyword list in `io/subschema.py` from the results, and record them in the test module docstring
    - Map jsonsubschema's own unsupported cases (an `enum` of objects, a negated object or array schema), which now surface as `error`, to `keywords` where the probe identifies them. Recheck `dependencies` (ignored by 0.0.8, so `dependentRequired` is unsupported), `multipleOf`, number negation, and integer/number `enum` handling
    - _Requirements: 31.7, 31.8_
  - [ ] 25.7 Add corpus-parametrized cases to the round-trip and translation tests
    - **Property 3: Schema emit/load round trip** and **Property 5: Draft translation preserves verdicts** over fully supported corpus schemas and SchemaStore tests (fixed subset in CI, full corpus nightly)
    - **Validates: Requirements 2.8, 2.9, 34.5**

- [ ] 26. Implement Mutation_Operators
  - [ ] 26.1 Implement mutations and negatives (`learn/mutations.py`)
    - All 14 operators with site enumeration and the design's violation forms; one mutated schema per site, or a seeded subset at the per-operator limit
    - Generate candidates from the mutated schema and keep distinct ones the ground truth rejects (with `format` asserted), up to the target or the attempt budget
    - Label each negative with operator, schema id, and site pointer. Record skips and unproductive operators
    - _Requirements: 32.1, 32.2, 32.3, 32.4, 32.5, 32.6, 32.7, 32.9, 32.10_
  - [ ] 26.2 Write property test for mutations
    - **Property 55: Negative correctness and mutation discipline**
    - **Validates: Requirements 32.2, 32.3, 32.4, 32.5, 32.9, 32.10**

- [ ] 27. Implement metrics and bootstrap statistics
  - [ ] 27.1 Implement metrics (`bench/metrics.py`)
    - Recall, negative rejection rate, and harmonic mean (0 when both are 0); undefined propagation; aggregates that skip undefined values and report their count; per-operator rejection rates (not applicable at zero)
    - Judgment accuracy via `derive_labels`; Constraint precision and recall via `align()` and the 31.10 matching rules; reliability bins reusing the ECE function
    - _Requirements: 31.3, 31.4, 31.5, 31.9, 31.10, 31.14, 32.8, 33.7, 33.9, 35.6_
  - [ ] 27.2 Write property test for metric definitions
    - **Property 53: Metric definitions**
    - **Validates: Requirements 31.3, 31.4, 31.5, 31.10, 31.14, 32.8, 33.7, 33.9**
  - [ ] 27.3 Implement the bootstrap (`bench/bootstrap.py`)
    - 95% percentile CI with `bootstrap.resamples` seeded resamples; paired test with p = min(1, 2·min(P*(Δ ≤ 0), P*(Δ ≥ 0))); not computable below two values or pairs
    - _Requirements: 35.1, 35.2_
  - [ ] 27.4 Write property test for statistical summaries
    - **Property 56: Statistical summaries**
    - **Validates: Requirements 21.5, 35.1, 35.2, 35.6**

- [ ] 28. Implement the run isolation and Baseline_Adapters
  - [ ] 28.1 Implement the adapter protocol and runner (`bench/baselines/base.py`, `bench/runner.py`)
    - Child processes with `start_new_session=True`, `os.killpg` on timeout (`bench.run_timeout_s`), a psutil process-tree RSS sampler, wall-clock timing, sequential runs, and the same JSONL input for every system. The Inferrer runs as `jsinfer infer`
    - Availability checks; failures recorded with exit status and stderr; outputs meta-validated; older-draft outputs translated with the Schema_Loader, recording translations and unsupported constructs; `BaselineInfo` recorded
    - _Requirements: 34.1, 34.2, 34.3, 34.5, 34.6, 34.7, 34.8, 34.11, 35.7_
  - [ ] 28.2 Implement the Naive_Merge and JSONoid adapters (`bench/baselines/naive.py`, `bench/baselines/jsonoid.py`)
    - Naive adapter calls `pipeline/naive_merge.py`. JSONoid runs `java -jar` on a jar pinned by release and SHA-256, with `--random-seed`, and records options, version, and Java version
    - _Requirements: 34.1, 34.4, 34.11_
  - [ ] 28.3 Write adapter tests
    - Stub baselines for the failure, timeout, unavailable, and invalid-output paths; JSONoid test skipped when Java is absent
    - _Requirements: 34.6, 34.7, 34.8_

- [ ] 29. Implement the synthetic and real-data benchmarks
  - [ ] 29.1 Implement the synthetic benchmark (`bench/synthetic.py`)
    - For each eligible Held_Out_Schema: training collections at each configured size; held-out positives distinct from training documents; mutation negatives; `corpus.exclude_ids` set to the held-out group
    - Run every available system, then compute recall, negative rejection, harmonic mean, inclusion relation, judgment accuracy, and Constraint precision and recall with `format` asserted. Record exclusions, failures, and timeouts as missing
    - Load and re-emit each truth schema as draft 2020-12 before the Subschema_Checker, which rejects earlier-draft and unknown keywords. Inferred schemas with `dependentRequired` get `Unsupported` until 25.6 (design Open Decisions, jsonsubschema keyword coverage)
    - _Requirements: 30.6, 31.1, 31.2, 31.6, 31.7, 31.8, 31.11, 31.12, 31.13_
  - [ ] 29.2 Implement the real-data benchmark (`bench/real.py`)
    - Manifest parsing with exclusions; SchemaStore schemas with positive tests as Real_Datasets; cleaning per 33.5
    - Seeded min(k, n) folds shared by all systems; generated negatives when none remain; per-fold metrics; not-available handling
    - _Requirements: 33.1, 33.2, 33.3, 33.4, 33.5, 33.7, 33.8, 33.9_
  - [ ] 29.3 Write property test for evaluation data preparation
    - **Property 52: Evaluation data preparation**
    - **Validates: Requirements 31.1, 33.3, 33.5**

- [ ] 30. Implement ablation, reporting, and the bench CLI
  - [ ] 30.1 Implement the ablation study (`bench/studies.py`)
    - One configuration per default-enabled Technique, disabling dependents transitively and recording the disabled set. Same collections and seed. Differences with paired CIs and p-values
    - _Requirements: 35.3, 35.10_
  - [ ] 30.2 Write property test for study construction
    - **Property 57: Study construction** (ablation clause now; contamination clause added in 41.3)
    - **Validates: Requirements 35.3, 35.10**
  - [ ] 30.3 Implement reporting (`bench/reporting.py`)
    - CSV and JSON written from the same rows; Markdown and LaTeX summary tables; Real_Dataset tables first and labeled primary, with per-dataset n, folds, exclusions, negative source, and exclusion counts
    - Unavailable systems marked; ReCG reimplementation label; timeout, failure, and missing counts; runtime, RSS, schema size, and DL columns
    - _Requirements: 33.6, 34.6, 34.9, 34.10, 35.7, 35.8, 35.12_
  - [ ] 30.4 Write property test for results serialization
    - **Property 58: Results serialization and aggregation**
    - **Validates: Requirements 34.9, 35.8, 35.12**
  - [ ] 30.5 Implement the Harness facade and `jsinfer bench` (`bench/runner.py`, `cli.py`)
    - `Harness` with `build_split`, `check_leakage`, `synthetic`, `real`, and `ablation`. `jsinfer bench {synthetic,real,ablation} -c bench.toml` runs the leakage check before any run
    - _Requirements: 30.7, 30.8, 30.11, 34.3_
  - [ ] 30.6 Write benchmark smoke tests (`tests/bench_smoke/`)
    - Five-schema fixture corpus with stub baselines: wiring, failure, timeout, and unavailable paths, plus every output format
    - _Requirements: 34.6, 34.7, 34.8, 35.8_

- [ ] 31. Checkpoint (M4): ensure all tests pass, ask the user if questions arise.

### M5: Should tier

- [ ] 32. Implement FlashProfile-style pattern induction
  - [ ] 32.1 Implement induction (`values/patterns.py`, `values/inferrer.py`, `config/registry.py`)
    - Atom tokenization; profiles by atom-class sequence; generality ladder (exact lengths, ranges, class widening, `+`); greedy merging down to `patterns.max_alternatives`
    - Validator check of every candidate against all distinct values (induction may use a sample); `patterns.attempt_budget`; MDL selection with ties to no Pattern, then fewer sub-patterns; discarded candidates and budget exhaustion reported
    - Applies to Paths with no selected Format that are not Closed_Enums, and to Map keys through `key_constraint` → `propertyNames`
    - _Requirements: 4.4, 14.1, 14.2, 14.3, 14.6, 14.7, 14.8_
  - [ ] 32.2 Write unit tests for induction limits
    - Attempt budget, the alternatives cap, and reporting
    - _Requirements: 14.7, 14.8_

- [ ] 33. Implement Field_Groups with FP-growth and Krimp-style selection
  - [ ] 33.1 Implement field groups (`structure/field_groups.py`, `structure/analyzer.py`, `config/registry.py`)
    - In-house FP-growth over distinct non-base key sets with counts at `fpgrowth.min_support`, sampled at `sample.fpgrowth`; Krimp-style code-table acceptance when total DL drops
    - All-or-none filter (violations reported) and greedy disjointness filter by DL gain; MDL selection among ∅ and the FP set
    - Loosen `test_disabled_cooccurrence_mining_emits_no_rules_and_keeps_required` in `tests/unit/test_config_fallbacks.py`, which asserts no `dependentRequired` at all: with co-occurrence mining off, Field_Groups may still emit it (9.9)
    - Emit `dependentRequired` cliques, with presence coded once in the MDL_Scorer; fallback none
    - Pass the selected groups to `CooccurrenceMiner.rules(groups=...)` so their cliques are given rules. When an antecedent has both a clique entry and a Co_Occurrence_Rule entry, merge them into one `dependentRequired` entry (one Constraint) and combine the two Provenance_Records
    - Property 19 then treats Field_Group pairs as given rules, and its check that a rule decision without a record is derivable in some Variant at its Path becomes live; confirm it passes
    - _Requirements: 6.2, 6.3, 6.6, 6.7, 6.8, 6.9_
  - [ ] 33.2 Write property test for Field_Groups
    - **Property 13: Field_Groups are well-formed and hold**
    - **Validates: Requirements 6.5, 6.6, 6.7, 6.8**
  - [ ] 33.3 Extend the mining-primitives test with FP-growth
    - **Property 17: Mining primitives equal brute-force references** (FP-growth frequent itemsets)
    - **Validates: Requirements 6.3**

- [ ] 34. Implement Tagged_Unions with per-field decision trees
  - [ ] 34.1 Implement trees and Discriminators (`deps/tagged_unions.py`, `structure/analyzer.py`, `config/registry.py`, `learn/trainer.py`)
    - In-house categorical tree with splits `P == v`: grown while DL decreases, post-pruned, limited by `trees.max_depth` and `trees.max_values`, sampled at `sample.trees`. Trees per field-presence target and per Variant target
    - Discriminator candidates are present everywhere and are strings, integers, or booleans. Select by DL with `tagged.branch_support` eligibility and ties by name; report non-selected candidates
    - Branches: dedicated `const`/`enum` branches with `required`, and a fallback branch per 10.5 and 10.8; exactly-one validation; value groups replace Variant branches
    - Extend the branch routing of Properties 12, 18, and 19 (which follow the Variant branches) to Discriminator value groups
    - Register the `tagged.branch_support` objective (branch F1)
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.8_
  - [ ] 34.2 Write property test for Tagged_Unions
    - **Property 20: Tagged unions are exact**
    - **Validates: Requirements 10.1, 10.2, 10.4, 10.5, 10.7, 10.8**

- [ ] 35. Implement exclusion mining
  - [ ] 35.1 Implement exclusions (`deps/exclusions.py`, `structure/analyzer.py`, `config/registry.py`, `learn/trainer.py`)
    - Members are fields or Field_Groups. Maximal cliques of the never-co-occur graph via Bron–Kerbosch with pivoting in canonical order; Confidence `1 − exp(−E)` with E reported
    - "Already enforced" check with Validator witnesses (kept when no witness exists); MDL or `exclusion.min_confidence`; `oneOf` vs `not` emission; rejections reported; fallback none
    - Register the `exclusion.min_confidence` objective
    - Settle the Support of Constraints inside `not` (now (0, 0), as `not` contents are not walked) and inside `if` (now always consistent); keep `rules.exclusion` in `KIND_SPECIFIC_CONFIDENCE` so its `1 − exp(−E)` Confidence survives finalization
    - Property 19's Exclusion clauses activate automatically. If an Exclusion `oneOf` sits on a node without `type` (beside Variant branches), extend that test's routing walk
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7_
  - [ ] 35.2 Write property test for Confidence monotonicity
    - **Property 21: Confidence functions are monotone** (exclusion clause now; key, reference, and FD clause added in 44.3)
    - **Validates: Requirements 11.2**
  - [ ] 35.3 Extend the mining-primitives test with exclusions
    - **Property 17: Mining primitives equal brute-force references** (maximal Exclusion_Rule candidates)
    - **Validates: Requirements 11.1**

- [ ] 36. Implement Shared_Definitions and recursion
  - [ ] 36.1 Implement the shared-definition pass (`structure/shared_defs.py`, `structure/analyzer.py`, `pipeline/infer.py`, `config/registry.py`)
    - Canonical subschema hash that ignores provenance; inline vs extracted model-bit comparison with rejections reported
    - Cross-Path Merge-or-Split over union views for object subschemas sharing a property, using the same Decision_Model kind
    - Recursion: prefix-related pairs become a self-referencing definition inferred from the union of levels, and the analyzer emits `$ref` at the recursive step
    - Naming per 8.5 with collision suffixes and default name `def`; Support across referencing Paths; `structure.shared_defs` and `structure.recursion` toggles per 8.7 and 8.8
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8, 26.2_
  - [ ] 36.2 Write property test for Shared_Definitions
    - **Property 16: Shared definitions are faithful**
    - **Validates: Requirements 8.2, 8.4, 8.5**

- [ ] 37. Implement hybrid Record/Map partitioning
  - [ ] 37.1 Implement the hybrid partition (`structure/record_map.py`, `config/registry.py`)
    - Top-j partitions by key frequency, plus the induced-Pattern partition; replaces the outcome only if strictly below both pure encodings
    - `patternProperties` when the Pattern matches every Map key and no Record field, otherwise `additionalProperties`; Closed-or-Open applies to the partitioned Record
    - Property 10's hybrid clause reads the partition DL from the report's `record_map_partition` entry at the Path; adjust that lookup if the entry differs
    - _Requirements: 4.5, 4.6, 4.7_

- [ ] 38. Implement outlier detection and MDL exception encoding
  - [ ] 38.1 Implement IF and LOF document flagging (`robust/outliers.py`, `pipeline/infer.py`, `config/registry.py`, `learn/trainer.py`)
    - Document features: BLAKE2b-hashed presence and type bits, size, depth, key and array counts, type rarity, and robust z-scores; rows sorted canonically before fitting
    - `IsolationForest` with a seeded `random_state`; LOF with `novelty=True`, sampled at `sample.lof`. Flag on strict threshold exceedance; abandon flags when every document would be flagged
    - Report entries per Flagged_Item; stage wired before analysis. Register the `iforest.score`, `lof.score`, and `lof.n_neighbors` objectives (F1 on mutation-corrupted validation documents)
    - _Requirements: 17.1, 17.4, 17.5, 17.6, 17.7, 17.9_
  - [ ] 38.2 Implement MDL exception encoding (`robust/outliers.py`, `structure/analyzer.py`, `values/inferrer.py`, `config/registry.py`)
    - Exception-bearing candidates at value and structural-form Decision_Points, with ties to widening. Winning rejections become Flagged_Items that affect only their subtree
    - Report the Constraints or Variant each flag would have added; fallback widen
    - _Requirements: 17.2, 17.3, 17.4, 17.5, 17.8_
  - [ ] 38.3 Write property test for outlier bookkeeping
    - **Property 27: Outlier bookkeeping**
    - **Validates: Requirements 17.1, 17.4, 17.5, 17.9**

- [ ] 39. Implement weak supervision
  - [ ] 39.1 Implement the Labeling_Functions (`learn/labeling_functions.py`)
    - At least three LFs per Judgment_Call kind, from the design table. Every cut-off is an `lf.<id>.*` Learned_Threshold or an MDL comparison
    - _Requirements: 23.1_
  - [ ] 39.2 Implement Dawid–Skene (`learn/label_model.py`)
    - EM over confusion matrices on non-abstain votes, Laplace smoothing, majority-vote initialization, `ds.max_iter` and `ds.tol`; NaN rows when every LF abstains
    - _Requirements: 23.3_
  - [ ] 39.3 Write property test for the label model and LF statistics
    - **Property 36: Label model and LF statistics are well-formed**
    - **Validates: Requirements 23.3, 23.5**
  - [ ] 39.4 Integrate weak supervision into the Trainer (`learn/trainer.py`, `learn/bundle.py`, `config/registry.py`)
    - Read manifest collections with the Ingestor; exclude bad entries with reasons; filter Real_Dataset ids and equal documents; fit LF thresholds on validation data
    - Label model per kind; second Decision_Model per kind using weighted weak examples, excluding tied labels and never adding them to validation; LF statistics and weak example counts in the manifest
    - _Requirements: 21.3, 23.2, 23.4, 23.5, 23.6, 23.7_
  - [ ] 39.5 Write property test for leakage guards
    - **Property 37: Leakage guards** (weak-supervision filtering and the harness abort guards of 25.4)
    - **Validates: Requirements 23.6, 30.7, 30.8, 30.11**

- [ ] 40. Implement corpus suggestions and the nearest known schema
  - [ ] 40.1 Build the Corpus_Index in the Trainer (`corpus/index.py`, `learn/trainer.py`, `learn/bundle.py`)
    - SQLite counts per name, (parent, name), and (name, constraint) from training-split schemas only, plus a table of normalized Path sets
    - MinHash signatures computed at load time and seeded from the run seed, with an optional seed-0 cache; `exclude_ids` filtering
    - _Requirements: 24.1, 25.1, 30.6_
  - [ ] 40.2 Implement name-based suggestions (`corpus/advisor.py`, `values/inferrer.py`, `config/registry.py`, `learn/trainer.py`)
    - Conditional frequencies, using the parent context when it matches, ranked deterministically
    - Acceptance checks: `corpus.suggest_min_confidence`, non-flagged values present, keyword applicability, and Validator acceptance. Failures are reported with counts. A shared keyword becomes one Decision_Point
    - Fallback and missing-index substitution; register the suggestion-acceptance objective
    - Keep `corpus.name_suggest` in `KIND_SPECIFIC_CONFIDENCE` so suggestion scores survive finalization
    - _Requirements: 24.3, 24.5, 24.6, 24.7, 24.8, 24.9_
  - [ ] 40.3 Write property test for suggestions
    - **Property 39: Suggestions are ranked and accounted for** (name-based clauses now; CF clauses added in 46.2)
    - **Validates: Requirements 24.3, 24.6, 30.6**
  - [ ] 40.4 Implement nearest-known-schema lookup (`corpus/advisor.py`, `pipeline/infer.py`, `config/registry.py`)
    - LSH candidates ranked by exact Jaccard with ties by source id; up to `nearest.next` follow the top one; shared and only-in lists; per Schema_Version
    - Report statuses for not found, disabled, and empty index. Schema and explanation bytes do not change
    - _Requirements: 25.1, 25.2, 25.3, 25.4, 25.5, 25.6, 25.7_
  - [ ] 40.5 Write property test for the nearest known schema
    - **Property 40: Nearest known schema**
    - **Validates: Requirements 25.1, 25.2, 25.3, 25.7, 30.6**

- [ ] 41. Implement the Should-tier harness extensions
  - [ ] 41.1 Implement the ReCG adapter (`bench/baselines/recg.py`, open decision)
    - Run the official command when one is configured. Otherwise run the in-house cluster-and-generalize reimplementation, labeled "ReCG (reimplementation)" in every output
    - _Requirements: 34.1, 34.10, 34.11_
  - [ ] 41.2 Implement the robustness study (`bench/studies.py`)
    - Seeded contamination at each configured rate, using mutation negatives disjoint from measurement negatives (documented rounding); changes in recall and rejection; loosened-Constraint counts per 35.4
    - _Requirements: 35.4_
  - [ ] 41.3 Extend the study-construction test with contamination
    - **Property 57: Study construction** (contamination clause)
    - **Validates: Requirements 35.4**
  - [ ] 41.4 Implement the calibration study (`bench/studies.py`)
    - Reliability-curve data and ECE over the Inferrer's Constraints on Held_Out_Schemas, using `ece.bins`
    - _Requirements: 35.6_
  - [ ] 41.5 Implement benchmark resume (`bench/resume.py`, `bench/runner.py`)
    - `results/<hash>.json` written by temp file and rename, with completion markers; restarts skip complete combinations and recompute absent or partial ones
    - _Requirements: 35.9_
  - [ ] 41.6 Write property test for benchmark resume
    - **Property 59: Benchmark resume**
    - **Validates: Requirements 35.9**
  - [ ] 41.7 Add the `robustness` and `calibration` bench subcommands (`cli.py`)
    - _Requirements: 35.4, 35.6_

- [ ] 42. Checkpoint (M5): ensure all tests pass, ask the user if questions arise.

### M6: Could tier

- [ ] 43. Implement array grammar inference
  - [ ] 43.1 Implement the array grammar (`structure/arrays.py`, `config/registry.py`)
    - Element kinds are JSON types, with objects split by Variant. Candidate grammars: `K*`, a prefix plus `K*`, `(K1|K2)*`, repeated groups found by a period check, and the exact tuple. Pick the lowest DL that matches every array
    - Expressible patterns produce a `prefixItems`/`items`/`contains` candidate compared with the 7.2–7.4 schema (ties keep the latter, rejected candidate recorded). Others go in `x-array-pattern`
    - _Requirements: 7.5, 7.6, 7.7, 7.8_
  - [ ] 43.2 Write property test for Array_Patterns
    - **Property 15: Array_Pattern soundness and precision**
    - **Validates: Requirements 7.5, 7.6**

- [ ] 44. Implement keys, references, and functional dependencies
  - [ ] 44.1 Implement the global pass (`deps/keys.py`, `pipeline/infer.py`, `config/registry.py`, `learn/trainer.py`)
    - Keys with collection and array scope, inclusion into selected keys, and FDs within one object scope (sampled at `sample.fd`). Confidences `n!/nⁿ`, `2^−(n−d)`, and `2^−d_A`
    - Discard and report below-threshold candidates. `uniqueItems` for scalar Lists; `x-key`, `x-references`, and `x-functional-dependency`; fallbacks per 12.9 and 12.10
    - Keep `rules.keys` and `rules.fd_ind` in `KIND_SPECIFIC_CONFIDENCE` so their Confidences survive finalization
    - Emit `x-key`, `x-references`, and `x-functional-dependency` in the payload shapes Property 19 reads (design, Open Decisions), or update its readers (`key_scope`, `referenced_path`, `dependency_paths`)
    - Register the half-split objectives for `keys.min_confidence`, `ind.min_confidence`, and `fd.min_confidence`
    - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 12.6, 12.7, 12.8, 12.9, 12.10_
  - [ ] 44.2 Extend the mining-primitives test with keys, references, and FDs
    - **Property 17: Mining primitives equal brute-force references** (Key_Constraint, Reference_Constraint, and Functional_Dependency candidates)
    - **Validates: Requirements 12.1, 12.2, 12.3, 12.4**
  - [ ] 44.3 Extend the Confidence monotonicity test with keys, references, and FDs
    - **Property 21: Confidence functions are monotone**
    - **Validates: Requirements 12.5**

- [ ] 45. Implement the Semantic_Type_Classifier
  - [ ] 45.1 Implement the classifier (`values/semantic.py`, `values/inferrer.py`, `learn/trainer.py`, `learn/bundle.py`, `config/registry.py`)
    - Sherlock-style aggregate features of a seeded value sample, plus hashed name and parent tokens; multiclass HGB with the documented classes and semantic-type → Format or Pattern mapping
    - Trained per 21.6 and calibrated on validation data, with manifest metrics. Inference candidate gated by `semantic.min_probability`; predictions with no mapping are reported. Register the objective
    - Keep `values.semantic_type` in `KIND_SPECIFIC_CONFIDENCE` so the calibrated probability survives finalization
    - _Requirements: 13.5, 13.7, 21.4, 21.5, 21.6_
  - [ ] 45.2 Write unit tests for the classifier
    - Threshold gating, unmapped types, and missing-model fallback
    - _Requirements: 13.7, 21.10_

- [ ] 46. Implement collaborative-filtering suggestions
  - [ ] 46.1 Implement CF suggestions (`corpus/advisor.py`, `corpus/index.py`, `config/registry.py`)
    - Noisy-OR over `P(constraint | sibling name)` from training-split counts, giving scores in [0, 1]; a duplicate of a name-based suggestion keeps the higher Confidence
    - Keep `corpus.cf_suggest` in `KIND_SPECIFIC_CONFIDENCE` so CF scores survive finalization
    - _Requirements: 24.4, 24.8_
  - [ ] 46.2 Extend the suggestions test with CF clauses
    - **Property 39: Suggestions are ranked and accounted for** (CF scores and duplicate handling)
    - **Validates: Requirements 24.4**

- [ ] 47. Implement Boolean matrix factorization field groups
  - [ ] 47.1 Implement BMF (`structure/field_groups.py`, `config/registry.py`)
    - Greedy MDL tiling from distinct transactions with k chosen by DL, sampled at `sample.bmf`. Output goes through the same filters, and selection compares ∅, FP, and BMF
    - _Requirements: 6.2, 6.4_
  - [ ] 47.2 Write unit tests for BMF
    - Factor selection and filter interaction
    - _Requirements: 6.4_

- [ ] 48. Implement change points and Schema_Versions
  - [ ] 48.1 Implement the Version_Detector (`robust/versions.py`, `config/registry.py`, `learn/trainer.py`)
    - Ordering per 18.2–18.4 with UTC normalization and the 18.11 fallback
    - PELT with a custom KT Bernoulli `BaseCost`, `changepoint.penalty`, and `changepoint.min_segment`; seeded sampling at `sample.changepoint` with boundary refinement; criterion values reported
    - Register the boundary-F1 objectives
    - _Requirements: 18.1, 18.2, 18.3, 18.4, 18.5, 18.11_
  - [ ] 48.2 Implement version outputs and wiring (`pipeline/versions_out.py`, `pipeline/infer.py`, `pipeline/soundness.py`)
    - Per-segment inference and soundness; segment labels; combined `anyOf` with hoisted `v<i>-` `$defs` and rewritten refs; diffs over (Path, keyword) → value maps
    - Explanation keys follow the combined schema. With no change point, emit a single schema
    - Keep `PipelineResult.schema` the combined schema and expose the per-Version Schema_Models on the result, so the gated per-Version checks of Properties 3 and 7 run
    - Fill `InferenceResult.versions`, `combined`, and `diffs`, which 18.1 leaves empty
    - _Requirements: 3.1, 18.6, 18.7, 18.8, 18.9, 18.10, 26.5_
  - [ ] 48.3 Write property test for ordering and segmentation
    - **Property 28: Ordering and segmentation**
    - **Validates: Requirements 18.2, 18.3, 18.4, 18.5, 18.6, 18.7, 18.8, 18.11**
  - [ ] 48.4 Implement the versioning study and `bench versioning` (`bench/studies.py`, `cli.py`)
    - Concatenate a Held_Out_Schema collection with a mutated successor; report change-point count, location error, per-segment harmonic means, and the missed-run fraction
    - _Requirements: 35.5, 35.11_

- [ ] 49. Add the nightly performance test
  - [ ] 49.1 Write the performance test (`tests/perf/`)
    - Generate the 100k-document reference collection; check the median of 3 runs against the time target and the summed peak RSS from psutil
    - Include a Variant-heavy case. Noise assignment (5.9) costs one MDL scoring per noise signature and existing group: with thresholds that label nearly every signature as noise, 20k documents with 960 signatures took about 50 s in task 13.2, mostly in the MDL_Scorer's per-distinct-value loop. Profile it if the time target is at risk
    - Include a deeply nested case. `View.digest` (through `core.canonical_bytes`) does work proportional to the Path length for every view, so the walk is roughly cubic in depth: about 4.4 s at depth 100 and 13.5 s at depth 150 with every Technique off (task 16.1). Cache digests if the time target is at risk
    - _Requirements: 29.6, 29.7_

- [ ] 50. Final checkpoint: ensure all tests pass, ask the user if questions arise.
  - Raise property coverage that the event statistics show is thin: the removal loop in Property 7 (about 2% of `ci` examples, 7% under `nightly`) and a soundness removal of a `dependentRequired` entry in Property 19 (never observed), for example by weighting `soundness.max_passes = 1` and injecting rule-breaking values
  - Investigate why whether tagged Variants split depends on field names (in Property 42, a tag named `type` beside fields `~` and `/` stayed merged, while `kind`, `$ref`, or `""` tags split), likely in the MinHash/LSH grouping's DL merge
  - Watch Property 42 under `nightly`: one early randomized probe timed out at 300 s and could not be reproduced
  - Hypothesis favors the first `sampled_from` choice and small integers; Property 42 offsets its draws by the documents' size, while Property 3's sum-of-two-draws spread still leans toward index 0. Apply the same correction where coverage events show the skew

## Notes

- Tasks marked `*` are optional test tasks and can be skipped for a faster MVP. Tasks 5.2 and 25.6 resolve open decisions from the design, so they are not optional.
- Each property test lives in `tests/property/` and implements exactly one design property. "Extend" sub-tasks add cases to the existing test for that property when a new component lands.
- 29.9 (training within 6 hours) is checked manually on the Reference_Laptop, per the design's Testing Strategy.
- ReCG and benchmark resume sit in M5 and ablation sits in M4, following the design's milestone table and the delivery tiers.

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1"] },
    { "id": 1, "tasks": ["1.2", "1.3", "2.1"] },
    { "id": 2, "tasks": ["2.2", "2.4", "2.6", "3.1", "3.2"] },
    { "id": 3, "tasks": ["2.3", "2.5", "3.3", "3.4", "4.1", "5.1"] },
    { "id": 4, "tasks": ["2.7", "3.5", "3.6", "4.2", "6.1", "6.2", "7.1"] },
    { "id": 5, "tasks": ["5.2", "6.3", "7.2"] },
    { "id": 6, "tasks": ["4.3", "4.4", "6.4", "6.5", "6.6", "6.7", "6.8"] },
    { "id": 7, "tasks": ["9.1", "10.1"] },
    { "id": 8, "tasks": ["9.2", "10.2"] },
    { "id": 9, "tasks": ["10.3"] },
    { "id": 10, "tasks": ["10.4", "11.1"] },
    { "id": 11, "tasks": ["11.2", "12.1", "12.2", "12.3", "12.4", "13.2", "14.1", "15.1"] },
    { "id": 12, "tasks": ["11.3", "12.5", "12.6", "12.7", "12.8", "13.3", "13.4", "14.2", "15.2"] },
    { "id": 13, "tasks": ["12.9", "13.1", "13.5", "14.3", "15.3"] },
    { "id": 14, "tasks": ["16.1"] },
    { "id": 15, "tasks": ["16.2", "17.2", "19.2"] },
    { "id": 16, "tasks": ["16.3", "16.4", "16.5", "16.6", "16.7", "16.8", "16.9", "16.10", "17.1", "19.3"] },
    { "id": 17, "tasks": ["17.3", "18.1"] },
    { "id": 18, "tasks": ["17.4", "17.5", "17.6", "18.2"] },
    { "id": 19, "tasks": ["18.3", "18.4", "19.1"] },
    { "id": 20, "tasks": ["19.4", "19.5", "19.6", "19.7", "19.8", "19.9"] },
    { "id": 21, "tasks": ["19.10", "21.1", "21.5", "22.4"] },
    { "id": 22, "tasks": ["21.2", "21.3", "21.6", "22.1"] },
    { "id": 23, "tasks": ["21.4", "22.2"] },
    { "id": 24, "tasks": ["22.3", "22.5", "23.1"] },
    { "id": 25, "tasks": ["23.2"] },
    { "id": 26, "tasks": ["23.3", "23.4"] },
    { "id": 27, "tasks": ["23.5"] },
    { "id": 28, "tasks": ["25.1", "25.4", "25.6", "26.1", "27.1", "27.3", "28.1"] },
    { "id": 29, "tasks": ["25.2", "25.5", "26.2", "27.2", "27.4", "28.2"] },
    { "id": 30, "tasks": ["25.3", "25.7", "28.3", "29.1", "29.2"] },
    { "id": 31, "tasks": ["29.3", "30.1", "30.3"] },
    { "id": 32, "tasks": ["30.2", "30.4", "30.5"] },
    { "id": 33, "tasks": ["30.6"] },
    { "id": 34, "tasks": ["32.1", "39.1", "39.2", "40.1", "41.1", "41.2", "41.5"] },
    { "id": 35, "tasks": ["32.2", "33.1", "39.3", "41.3", "41.4", "41.6"] },
    { "id": 36, "tasks": ["33.2", "33.3", "34.1", "41.7"] },
    { "id": 37, "tasks": ["34.2", "35.1"] },
    { "id": 38, "tasks": ["35.2", "35.3", "36.1"] },
    { "id": 39, "tasks": ["36.2", "37.1"] },
    { "id": 40, "tasks": ["38.1"] },
    { "id": 41, "tasks": ["38.2"] },
    { "id": 42, "tasks": ["38.3", "39.4"] },
    { "id": 43, "tasks": ["39.5", "40.2"] },
    { "id": 44, "tasks": ["40.3", "40.4"] },
    { "id": 45, "tasks": ["40.5"] },
    { "id": 46, "tasks": ["43.1"] },
    { "id": 47, "tasks": ["43.2", "44.1"] },
    { "id": 48, "tasks": ["44.2", "44.3", "45.1"] },
    { "id": 49, "tasks": ["45.2", "46.1"] },
    { "id": 50, "tasks": ["46.2", "47.1"] },
    { "id": 51, "tasks": ["47.2", "48.1"] },
    { "id": 52, "tasks": ["48.2"] },
    { "id": 53, "tasks": ["48.3", "48.4", "49.1"] }
  ]
}
```
