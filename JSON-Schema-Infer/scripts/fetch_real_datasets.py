"""Build the larger Real_Datasets (Requirement 33.1) under ``datasets/`` and write ``datasets/manifest.json``.

Three collections with independently authored schemas and hundreds of documents each:

- ``github-webhooks``: the payload examples of octokit/webhooks (a shallow checkout under
  ``checkouts/octokit-webhooks``) against the bundled schema of ``@octokit/webhooks-schemas``
  (downloaded from unpkg); one dataset over every event, plus one per event with at least
  ``--per-event-min`` payloads.
- ``npm-packages``: the ``latest`` version documents of popular packages from the npm registry
  against SchemaStore's ``package.json`` schema. That schema references other SchemaStore
  schemas by URL (``ava``, ``eslintConfig``, ``jscpd``, ``release``, ...), which the offline
  Validator cannot fetch, so those members are replaced by ``true`` in the written copy; the
  change is recorded in ``schema-changes.json`` next to it.
- ``geojson-countries``: the features of the ``datasets/geo-countries`` collection against
  ``https://geojson.org/schema/Feature.json``.

Usage: ``python scripts/fetch_real_datasets.py [--npm 300] [--per-event-min 8]``. Raw downloads go
to ``datasets/raw``; everything under ``datasets/`` is ignored by git.
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATASETS = ROOT / "datasets"
RAW = DATASETS / "raw"
WEBHOOKS = ROOT / "checkouts" / "octokit-webhooks" / "payload-examples" / "api.github.com"
SCHEMASTORE_PACKAGE = ROOT / "checkouts" / "schemastore" / "src" / "schemas" / "json" / "package.json"

URLS = {
    "webhooks-schema.json": "https://unpkg.com/@octokit/webhooks-schemas/schema.json",
    "geojson-feature.json": "https://geojson.org/schema/Feature.json",
    "countries.geojson": "https://raw.githubusercontent.com/datasets/geo-countries/main/data/countries.geojson",
}


def fetch(url: str, dest: Path) -> None:
    if dest.exists():
        return
    print(f"fetching {url}", file=sys.stderr)
    urllib.request.urlretrieve(url, dest)


def get_json(url: str, retries: int = 3):
    for attempt in range(retries):
        try:
            with urllib.request.urlopen(url, timeout=60) as response:
                return json.load(response)
        except Exception as error:  # noqa: BLE001
            if attempt + 1 == retries:
                raise
            print(f"retrying {url}: {error}", file=sys.stderr)
            time.sleep(2)


def write_jsonl(path: Path, documents) -> int:
    count = 0
    with open(path, "w", encoding="utf-8") as handle:
        for document in documents:
            handle.write(json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n")
            count += 1
    return count


def dataset_dir(name: str) -> Path:
    path = DATASETS / name
    path.mkdir(parents=True, exist_ok=True)
    return path


def webhooks(per_event_min: int) -> list[dict]:
    if not WEBHOOKS.is_dir():
        raise SystemExit(f"{WEBHOOKS} is missing: git clone --depth 1 https://github.com/octokit/webhooks checkouts/octokit-webhooks")
    schema = json.load(open(RAW / "webhooks-schema.json", encoding="utf-8"))
    entries = []
    everything = []
    for event in sorted(p for p in WEBHOOKS.iterdir() if p.is_dir()):
        payloads = [json.load(open(f, encoding="utf-8")) for f in sorted(event.glob("*.payload.json"))]
        everything.extend(payloads)
        if len(payloads) >= per_event_min:
            out = dataset_dir(f"github-webhooks-{event.name}")
            write_jsonl(out / "docs.jsonl", payloads)
            json.dump(schema, open(out / "schema.json", "w", encoding="utf-8"))
            entries.append({"id": f"github-webhooks-{event.name}", "documents": f"github-webhooks-{event.name}/docs.jsonl", "schema": f"github-webhooks-{event.name}/schema.json"})
    out = dataset_dir("github-webhooks")
    n = write_jsonl(out / "docs.jsonl", everything)
    json.dump(schema, open(out / "schema.json", "w", encoding="utf-8"))
    entries.insert(0, {"id": "github-webhooks", "documents": "github-webhooks/docs.jsonl", "schema": "github-webhooks/schema.json"})
    print(f"github-webhooks: {n} payloads, {len(entries) - 1} per-event datasets", file=sys.stderr)
    return entries


def offline_package_schema() -> tuple[dict, list[str]]:
    schema = json.load(open(SCHEMASTORE_PACKAGE, encoding="utf-8"))
    changed: list[str] = []

    def strip(node, pointer: str):
        if isinstance(node, dict):
            ref = node.get("$ref")
            if isinstance(ref, str) and not ref.startswith("#"):
                changed.append(pointer)
                return True
            return {k: strip(v, f"{pointer}/{k}") for k, v in node.items()}
        if isinstance(node, list):
            return [strip(v, f"{pointer}/{i}") for i, v in enumerate(node)]
        return node

    return strip(copy.deepcopy(schema), ""), changed


def npm(count: int) -> list[dict]:
    if not SCHEMASTORE_PACKAGE.is_file():
        raise SystemExit(f"{SCHEMASTORE_PACKAGE} is missing: the SchemaStore checkout is needed")
    names: list[str] = []
    start = 0
    while len(names) < count:
        page = get_json(f"https://registry.npmjs.org/-/v1/search?text=keywords:javascript&size=250&from={start}&popularity=1.0")
        found = [o["package"]["name"] for o in page["objects"]]
        if not found:
            break
        names.extend(n for n in found if n not in names)
        start += 250
    names = names[:count]
    documents = []
    for i, name in enumerate(names, 1):
        try:
            documents.append(get_json(f"https://registry.npmjs.org/{urllib.request.quote(name, safe='@')}/latest"))
        except Exception as error:  # noqa: BLE001
            print(f"skipping {name}: {error}", file=sys.stderr)
        if i % 50 == 0:
            print(f"npm: {i}/{len(names)}", file=sys.stderr)
    out = dataset_dir("npm-packages")
    n = write_jsonl(out / "docs.jsonl", documents)
    schema, changed = offline_package_schema()
    json.dump(schema, open(out / "schema.json", "w", encoding="utf-8"))
    json.dump({"source": str(SCHEMASTORE_PACKAGE.relative_to(ROOT)), "external_refs_replaced_by_true": changed}, open(out / "schema-changes.json", "w", encoding="utf-8"), indent=1)
    print(f"npm-packages: {n} documents; {len(changed)} external $ref members replaced", file=sys.stderr)
    return [{"id": "npm-packages", "documents": "npm-packages/docs.jsonl", "schema": "npm-packages/schema.json"}]


def geojson() -> list[dict]:
    collection = json.load(open(RAW / "countries.geojson", encoding="utf-8"))
    out = dataset_dir("geojson-countries")
    n = write_jsonl(out / "docs.jsonl", collection["features"])
    schema = json.load(open(RAW / "geojson-feature.json", encoding="utf-8"))
    json.dump(schema, open(out / "schema.json", "w", encoding="utf-8"))
    print(f"geojson-countries: {n} features", file=sys.stderr)
    return [{"id": "geojson-countries", "documents": "geojson-countries/docs.jsonl", "schema": "geojson-countries/schema.json"}]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--npm", type=int, default=300, help="how many npm packages to fetch (default 300)")
    parser.add_argument("--per-event-min", type=int, default=8, help="payloads an event needs for its own dataset (default 8)")
    args = parser.parse_args()
    RAW.mkdir(parents=True, exist_ok=True)
    for name, url in URLS.items():
        fetch(url, RAW / name)
    entries = [*webhooks(args.per_event_min), *npm(args.npm), *geojson()]
    manifest = {"datasets": entries}
    json.dump(manifest, open(DATASETS / "manifest.json", "w", encoding="utf-8"), indent=1)
    print(f"wrote {DATASETS / 'manifest.json'} with {len(entries)} datasets", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
