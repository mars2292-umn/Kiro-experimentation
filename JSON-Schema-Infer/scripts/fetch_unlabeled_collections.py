"""Fetch the unlabeled Document_Collections of weak supervision (Requirement 23.2) and write their manifest.

Weak supervision trains on real JSON whose schema is unknown. Every collection the benchmark uses is a
Real_Dataset (SchemaStore test files, npm package documents, GitHub webhook payloads, country GeoJSON),
and 23.6 excludes those; documents from the same sources would also train on the benchmark's own
distribution. These collections come from public APIs in unrelated domains (OpenAlex was dropped: it
now needs an API key beyond the first page, and Crossref covers the same domain):

- ``crossref-works``: scholarly work records from the Crossref REST API (cursor paging).
- ``nobel-laureates``: laureate records from the Nobel Prize API v2.1.
- ``artic-artworks``: artwork records from the Art Institute of Chicago API (full records).
- ``openlibrary-search``: book search documents from Open Library, over a fixed list of subjects.

Requests carry a generic User-Agent and no personal details, and pause between pages. Each collection is
written as JSONL under ``datasets/unlabeled/<id>/documents.jsonl`` (``datasets/`` is ignored by git), with
``datasets/unlabeled/manifest.json`` listing ``{"collections": [{"id", "path", "source", "fetched"}]}``.
The Trainer removes documents equal to a Real_Dataset document and excludes collections whose id is a
Real_Dataset's (23.6), so the manifest is not filtered here.

Usage: ``python scripts/fetch_unlabeled_collections.py [--per-collection 1000]``.
"""

from __future__ import annotations

import argparse
import datetime
import json
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "datasets" / "unlabeled"
USER_AGENT = "jsinfer-research/0.1 (schema inference benchmark; weak supervision data)"
PAUSE_S = 1.0
SUBJECTS = ("history", "science", "poetry", "mathematics", "cooking", "travel", "music", "art", "philosophy", "medicine")


def get_json(url: str, retries: int = 4):
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "application/json"})
    for attempt in range(retries):
        try:
            with urllib.request.urlopen(request, timeout=90) as response:
                return json.load(response)
        except Exception as error:  # noqa: BLE001
            if attempt + 1 == retries:
                raise
            print(f"retrying {url}: {error}", file=sys.stderr)
            time.sleep(5 * (attempt + 1))
    raise AssertionError("unreachable")


def crossref(limit: int) -> list[dict]:
    docs: list[dict] = []
    cursor = "*"
    while len(docs) < limit:
        page = get_json(f"https://api.crossref.org/works?rows=200&sort=indexed&order=desc&cursor={urllib.parse.quote(cursor)}")
        items = page["message"]["items"]
        if not items:
            break
        docs.extend(items)
        cursor = page["message"]["next-cursor"]
        time.sleep(PAUSE_S)
    return docs[:limit]


def nobel(limit: int) -> list[dict]:
    docs: list[dict] = []
    offset = 0
    while len(docs) < limit:
        page = get_json(f"https://api.nobelprize.org/2.1/laureates?limit=100&offset={offset}")
        items = page.get("laureates", [])
        if not items:
            break
        docs.extend(items)
        offset += len(items)
        time.sleep(PAUSE_S)
    return docs[:limit]


def artic(limit: int) -> list[dict]:
    docs: list[dict] = []
    page_no = 1
    while len(docs) < limit:
        page = get_json(f"https://api.artic.edu/api/v1/artworks?limit=100&page={page_no}")
        items = page.get("data", [])
        if not items:
            break
        docs.extend(items)
        page_no += 1
        time.sleep(PAUSE_S)
    return docs[:limit]


def openlibrary(limit: int) -> list[dict]:
    docs: list[dict] = []
    per_subject = max(1, limit // len(SUBJECTS))
    for subject in SUBJECTS:
        page = get_json(f"https://openlibrary.org/search.json?q={urllib.parse.quote(subject)}&limit={per_subject}")
        docs.extend(page.get("docs", []))
        time.sleep(PAUSE_S)
    return docs[:limit]


SOURCES = {
    "crossref-works": ("https://api.crossref.org/works", crossref),
    "nobel-laureates": ("https://api.nobelprize.org/2.1/laureates", nobel),
    "artic-artworks": ("https://api.artic.edu/api/v1/artworks", artic),
    "openlibrary-search": ("https://openlibrary.org/search.json", openlibrary),
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--per-collection", type=int, default=1000)
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    entries = []
    for cid, (source, fetch) in SOURCES.items():
        target = OUT / cid / "documents.jsonl"
        if not target.exists():
            print(f"fetching {cid}", file=sys.stderr)
            try:
                docs = fetch(args.per_collection)
            except Exception as error:  # noqa: BLE001 - one unavailable source does not stop the others
                print(f"skipping {cid}: {error}", file=sys.stderr)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            with open(target, "w", encoding="utf-8") as handle:
                for doc in docs:
                    handle.write(json.dumps(doc, ensure_ascii=False) + "\n")
        count = sum(1 for _ in open(target, encoding="utf-8"))
        fetched = datetime.datetime.fromtimestamp(target.stat().st_mtime, datetime.timezone.utc).date().isoformat()
        entries.append({"id": cid, "path": str(target.relative_to(OUT)), "source": source, "fetched": fetched, "documents": count})
        print(f"{cid}: {count} documents", file=sys.stderr)
    (OUT / "manifest.json").write_text(json.dumps({"collections": entries}, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {OUT / 'manifest.json'}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
