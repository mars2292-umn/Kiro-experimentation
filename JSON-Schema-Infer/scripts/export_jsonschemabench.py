"""Export the JSONSchemaBench parquet subsets (train/val/test per subset) to one JSONL file per subset."""
import json, sys
from pathlib import Path
import pyarrow.parquet as pq
from huggingface_hub import snapshot_download

out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
snap = Path(snapshot_download("epfl-dlab/JSONSchemaBench", repo_type="dataset", allow_patterns=["*/test-*.parquet", "*/train-*.parquet", "*/val-*.parquet"]))
sha = snap.name  # the snapshot directory is named by the commit sha
total = 0
for subset in sorted(p for p in snap.iterdir() if p.is_dir() and p.name != "data"):
    rows = 0
    with open(out / f"{subset.name}.jsonl", "w", encoding="utf-8") as handle:
        for file in sorted(subset.glob("*.parquet")):
            table = pq.read_table(file)
            cols = table.column_names
            for row in table.to_pylist():
                record = {"json_schema": row["json_schema"], "split": file.name.split("-")[0]}
                if "unique_id" in cols: record["unique_id"] = row["unique_id"]
                handle.write(json.dumps(record) + "\n"); rows += 1
    print(subset.name, rows, "rows; columns:", cols); total += rows
(out / "REVISION").write_text(sha + "\n")
print("total", total, "revision", sha)
