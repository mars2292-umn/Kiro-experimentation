#!/usr/bin/env python3
"""Task 8.4 of docs/pacemaker/tasks.md (PM21.2, PM21.5): rsk's build checks on
the pulse generator's Flight builds, one per Target variant.

Usage (from the workspace root): apps/pacemaker/build_checks.py <out-dir>

For each variant the binary is built without any rig feature, and then:

- the Link_Checker (`rsk-link`, with the Profile and the verified modules,
  as the verification job calls it): every function bounded, every stack
  region holding its worst case; its stack bounds are written;
- the Config_Checker (`rsk-confcheck`) on the Task_Model and the binary,
  with those stack bounds;
- the entry stubs (`cargo xtask stubs`) against the stub model's source.

The Profile_Lint is not per binary: `cargo xtask check` runs it over every
Flight crate of the workspace, the Application's four among them, and its
verdict line is recorded here as well. The reports go to <out-dir>; the
script exits 0 when every check passed.
"""
import hashlib
import json
import os
import re
import subprocess
import sys

out_dir = sys.argv[1]
os.makedirs(out_dir, exist_ok=True)
TARGET = "thumbv7em-none-eabihf"
VARIANTS = [
    ("qemu", [], "apps/pacemaker/gen"),
    ("nrf52840", ["--no-default-features", "--features", "nrf52840"], "apps/pacemaker/gen-nrf52840"),
]


def run(cmd):
    p = subprocess.run(cmd, capture_output=True, text=True)
    return p.returncode, p.stdout + p.stderr


def verified_modules():
    """As xtask's `verified_modules`: the modules of the Kernel crates that hold Verus code."""
    policy = open("Cargo.toml").read()
    out = []
    for name in re.findall(r"^([a-z0-9-]+) = \{[^}\n]*\bkernel = true", policy, re.M):
        src = os.path.join("crates", name, "src")
        for root, _, files in os.walk(src):
            for f in files:
                if f.endswith(".rs") and "verus! {" in open(os.path.join(root, f)).read():
                    rel = os.path.relpath(os.path.join(root, f), src)[:-3].split(os.sep)
                    out.append("::".join([name.replace("-", "_")] + [s for s in rel if s not in ("lib", "mod", "main")]))
    return sorted(out)


verified = verified_modules()
results = {}
ok_all = True
for name, features, gen in VARIANTS:
    target_dir = f"target/pacemaker-flight-{name}"
    code, text = run(["cargo", "build", "--quiet", "--release", "--target", TARGET, "-p", "pacemaker", "--target-dir", target_dir] + features)
    if code != 0:
        sys.exit(f"{name}: the build failed\n{text[-2000:]}")
    elf = f"{target_dir}/{TARGET}/release/pacemaker"
    model = f"{gen}/task_model.json"
    bounds = f"{out_dir}/stack_bounds_{name}.json"
    link = ["cargo", "run", "--quiet", "-p", "rsk-link", "--", elf, model, "--out", bounds, "--profile", "profile/profile.toml",
            "--core-subset", "profile/core_subset.toml", "--system-crate", "pacemaker-system"]
    for v in verified:
        link += ["--verified", v]
    link_code, link_text = run(link)
    conf_code, conf_text = run(["cargo", "run", "--quiet", "-p", "rsk-confcheck", "--", model, elf] + ([bounds] if os.path.isfile(bounds) else []))
    stubs_code, stubs_text = run(["cargo", "xtask", "stubs", elf])
    keep = lambda text: "\n".join(l for l in text.splitlines() if not l.startswith("warning") and "rsk-protocol" not in l and not re.match(r"^\s*(\||-->|=|\d+ \|)", l) and l.strip())
    open(f"{out_dir}/link_{name}.log", "w").write(keep(link_text) + "\n")
    open(f"{out_dir}/confcheck_{name}.log", "w").write(keep(conf_text) + "\n")
    open(f"{out_dir}/stubs_{name}.log", "w").write(keep(stubs_text) + "\n")
    last = lambda text, prefix: next((l for l in reversed(text.splitlines()) if l.startswith(prefix)), f"{prefix} no verdict line")
    results[name] = {
        "binary_sha256": hashlib.sha256(open(elf, "rb").read()).hexdigest(),
        "task_model": model,
        "link_checker": {"passed": link_code == 0, "verdict": last(link_text, "rsk-link:")},
        "config_checker": {"passed": conf_code == 0, "verdict": last(conf_text, "rsk-confcheck:")},
        "entry_stubs": {"passed": stubs_code == 0, "verdict": next((l.split(": ", 1)[1] for l in stubs_text.splitlines() if "the entry stubs are" in l), "no verdict line")},
    }
    ok_all = ok_all and link_code == 0 and conf_code == 0 and stubs_code == 0
    print(name, json.dumps(results[name], indent=1))

code, text = run(["cargo", "xtask", "check"])
lint = next((l for l in text.splitlines() if "Profile_Lint over" in l), "no Profile_Lint line")
results["profile_lint"] = {"passed": code == 0 and "no violation" in lint, "verdict": lint, "static_checks": text.strip().splitlines()[-1]}
ok_all = ok_all and results["profile_lint"]["passed"]
print(json.dumps(results["profile_lint"], indent=1))
results["verdict"] = "PASS" if ok_all else "FAIL"
json.dump(results, open(f"{out_dir}/build_checks.json", "w"), indent=2)
sys.exit(0 if ok_all else 1)
