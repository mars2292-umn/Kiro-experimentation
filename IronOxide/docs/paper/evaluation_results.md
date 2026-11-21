# IronOxide evaluation results

Status on 2026-11-12: items 1 and 2 complete; items 3 to 8 not started (waiting for the check-in after item 2). Every number below was obtained from a run in the session of 2026-11-12 on the machine named in item 2, and the raw logs are in `docs/paper/logs/`.

Added 2026-11-21: section 9, the Kernel masking time on `main`. It is a different revision and a different session from items 1 to 8; its sources are named there.

Revisions used throughout:

| Name | Commit | What it is |
|---|---|---|
| `ironoxide-eval-v1` (annotated tag) | `2bc1eb8673efecf049768341ce6f4550b99a7cef` (2026-11-07 15:05) | The evaluated Kernel |
| branch `paper-eval` | `95a3e921b8bca1659b257839500a7ab83f129410` | The tag plus evaluation tooling under `xtask/` only; `git diff ironoxide-eval-v1 -- IronOxide/crates` is empty |

## 1. Evaluated revision

Tag: `ironoxide-eval-v1` = `2bc1eb8`. Session-typed Endpoints: **out** (the stated default; to be confirmed at the check-in).

Why this commit. It is the last commit before task 21.2 (`06be388`, Kernel-owned Endpoint buffers). It contains the INVPC dispatch-window fix (plan 5.8a), L_kernel taken from the measured masked span, and the Budget `reported` fix. Its mutation register holds exactly 31 entries; the register grows to 33 at `4edb4ac` and to 58 at the head of `main`.

Commands:

```
git tag -a ironoxide-eval-v1 2bc1eb8
git worktree add --detach ../Kiro-experimentation-evaltag ironoxide-eval-v1      # pristine tag, mutation run
git worktree add -b paper-eval ../Kiro-experimentation-eval ironoxide-eval-v1    # tooling and later items
git diff ironoxide-eval-v1 --stat -- IronOxide/crates                            # empty on paper-eval
for c in 2bc1eb8 8f4641e 4edb4ac 423a366; do git show $c:IronOxide/docs/proof_mutations.toml | grep -c '^\[\[mutation\]\]'; done   # 31 31 33 58
```

What the choice leaves out. Between the tag and `423a366` the three Kernel crates change by 1,446 inserted and 26 deleted lines in 19 files (`git diff --stat 2bc1eb8..423a366 -- IronOxide/crates/rsk-{kernel,logic,entry}`), of which `arch/cm4/entry.rs` is 741. That is the Endpoint service path, the multiparty helpers and the re-initialization rules. Not yet checked: whether any of those 1,446 lines is a fix unrelated to Endpoints that the tag therefore lacks.

Not done in this session: `cargo xtask verify` at the tag. The commit message of `2bc1eb8` records a passing job; that is a claim of the history, not a result of this session.

## 2. Proof effort

Tag: `ironoxide-eval-v1`; tooling at `95a3e92` (`cargo xtask proof-effort`, new).

### 2.1 Line counts

Command: `cargo xtask proof-effort` (JSON: `--json`; one file line by line: `--lines <file>`). Raw output: `logs/proof-effort.txt`, `logs/proof-effort.json`.

| Crate | Executable | Specification | Proof | Harness: Kani | Harness: test | Harness: host double | Harness total | Feature-gated |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| rsk-entry | 102 | 0 | 0 | 0 | 0 | 1 | 1 | 5 |
| rsk-kernel | 2,040 | 246 | 84 | 401 | 103 | 41 | 545 | 1,422 |
| rsk-logic | 1,337 | 2,039 | 2,290 | 312 | 0 | 0 | 312 | 0 |
| **Total** | **3,479** | **2,285** | **2,374** | 713 | 103 | 42 | **858** | 1,427 |

Definitions. Executable is the existing PAR-01 counter (`xtask/src/checks/kernel_size.rs`): the Target's cfg with no Cargo feature, without comments, blank lines, attributes, layout-only lines, generated files and files under a cfg-false `mod`. The command's figures equal the R6.2 note of `cargo xtask check` (3,479; rsk-entry 102, rsk-kernel 2,040, rsk-logic 1,337). Specification is `spec fn` items, signature clauses and ghost items. Proof is `proof fn` bodies, proof blocks, assertions, ghost bindings and loop clauses. One class per line, executable over specification over proof. Harness is code the Target does not build. Feature-gated is code only a Cargo feature switches on (the HIL instrumentation, board variants); it is neither executable nor harness.

Caveats a reviewer needs:

| Caveat | Size | Effect |
|---|---:|---|
| The PAR-01 counter cuts `Kernel::timer_event` (`kernel.rs:2271-2441`) at the brace group inside its `ensures`, and counts the rest of the header and the body as executable | 144 lines in the cut ranges; about 125 of them are not executable (measured on a scratch copy with the braces replaced: kernel.rs executable 543 to 418, proof 1,041 to 1,164, specification 924 to 930) | Executable is overstated: about 3,354 without the cut; proof about 2,497, specification about 2,291 |
| Executable lines with no code token of their own (comments and attributes inside multi-line items) | 235 of 3,479 | 3,244 executable lines carry a token |
| Ghost data types counted as executable (`Step`, `HwModel`) | 35 | Executable overstated |
| Specification and proof keep layout-only and comment lines, executable does not | spec 2,046 with code + 199 layout + 40 comment; proof 1,706 with code + 574 layout + 94 comment | For a like-for-like ratio use 2,046 and 1,706 |
| A crate's default features count as off | rsk-entry `app-p1-demo`, rsk-kernel `board-nrf52840` | Default-on code sits in the feature-gated column |

What the numbers show. With the counter as it stands the Kernel is 3,479 executable lines against 4,659 lines of specification and proof, a ratio of 1.34, almost all of it in rsk-logic; rsk-kernel's 2,040 executable lines (the architecture layer and the unsafe module) carry 330. On code-bearing lines only the ratio is 3,752 to 3,244, or 1.16, and the counter's mis-cut of one function moves about 125 lines from proof to executable.

The implementation was audited by three independent reviewers (a hand audit of the per-line classes in seven files, a numeric consistency check, and a code review of the timing mode); 16 findings were fixed, none changed a column of the table. The command has 29 unit tests; `cargo test -p xtask` passes (74 unit, 13 fixture) and `cargo xtask check` passes.

### 2.2 Wall-clock time of the proofs

Command: `cargo xtask proof-effort --time all` at `95a3e92`, 2026-11-12 23:33 to 23:37 UTC. Raw output: `logs/time-all.log`, `logs/timing-all.json`.

Machine: Apple M4 Pro, 14 cores (10 performance, 4 efficiency), 48 GiB, Darwin 27.0.0 arm64, on AC power. Load average at start 2.48 (an IDE was using about two cores; the machine was not otherwise idle).

| Step | Wall-clock | Result | Threads | Tool |
|---|---:|---|---|---|
| Verus, all three verified crates, cold | 58.6 s | 543 items verified, 0 errors | Verus default, 13 per `rust_verify` process (no `--num-threads` passed) | Verus 0.2026.10.04.426d8b0, Z3 4.16.0 |
| of which rsk-logic (includes vstd) | 34.6 s | 325 items; 1,876 dependency items verified in the same invocation | | |
| of which rsk-kernel | 7.7 s | 154 items | | |
| of which rsk-analyzer (not a Kernel crate) | 16.3 s | 64 items | | |
| Kani, both Kernel crates, cold | 175.2 s | 19 harnesses passed | Sequential, one harness at a time (no `--jobs` passed); CaDiCaL unless a harness names a solver | Kani 0.68.0, CBMC 6.11.0 |
| of which rsk-kernel | 143.7 s | 9 harnesses | | |
| of which rsk-logic | 31.4 s | 10 harnesses | | |

Cold means the step's target directory was deleted first, so the Verus time includes compiling and verifying the vendored `vstd` and building the Verus macros, and the Kani time includes the build. Each step ran once; no variance is reported.

What the numbers show. The full Kernel proof (479 items in rsk-logic and rsk-kernel) checks in 42 s from an empty directory on a laptop, and the bounded model checking of the unsafe module and the logic cross-checks takes just under three minutes. The whole proof step is therefore cheap enough to run on every change.

### 2.3 Proof mutations

Command, in the pristine checkout of the tag: `cargo xtask mutations`, 2026-11-12 23:01 to 23:09 UTC. Raw output: `logs/mutations.log`.

| Registered | Killed | Survived | Wall-clock |
|---:|---:|---:|---:|
| 31 | 31 | 0 | 513.4 s (user 649.9 s) |

Kill diagnostic, by kind:

| Verus diagnostic | Mutants |
|---|---:|
| postcondition not satisfied | 10 |
| assertion failed | 8 |
| precondition not satisfied | 6 |
| loop invariant not satisfied (end of body 3, before loop 1) | 4 |
| resource limit (rlimit) exceeded | 3 |

What the numbers show. Every registered mutant is rejected with its registered diagnostic text. Three of them (`mit-run-spacing-dropped`, `lock-by-other-than-top`, `dispatch-below-ceiling`, all in the history lemmas of `history.rs`) are rejected because the solver ran out of its resource limit in the named lemma, not because it reported a failed assertion; the register's expected text names the lemma and so accepts either. For those three the run shows that the proof no longer goes through, which is weaker than showing a refuted obligation.

## 3. RTIC baseline

Not started.

## 4. Uninstrumented IronOxide timing

Not started.

## 5. Unexplained observations

Not started.

## 6. Second application from a HAMR model

Not started.

## 7. Tock comparison

Not started.

## 8. Stub preemption windows

Not started.

## 9. Kernel masking time on `main` (added 2026-11-21)

This section is not about the evaluated tag. Revision: `main` at `44df784c` (2026-11-21), which differs from `ironoxide-eval-v1` in the Kernel crates. Every number is from a run of 2026-11-21 on the rig (nRF52840-DK, 64 MHz, 64 cycles = 1 µs); the reports are `docs/hil/stress_nrf52840_stress.md` and `docs/hil/release_timing_nrf52840_timing.md`, the exported parameters `docs/hil/kernel_timing_nrf52840_{stress,timing}.json`.

What is measured. The *hold* is the longest stretch in which a pending Kernel-level interrupt cannot be taken: one Kernel handler from the beginning of its Rust half to its end, with the unwind it runs before it returns. It replaces the *masked span* that item 1 names as the basis of L_kernel at the tag. The span is the longest run of Kernel entries back to back with no return to a Job; it is not a masked stretch, because the processor takes a pending Kernel-level interrupt at each exception return, and the Analyzer already charges each entry of such a run to the event that caused it.

Commands: `cargo xtask hil --stress` and `cargo xtask hil --release-timing` (3,660 s each); `cargo xtask hil --stress --short [--lean]` (180 s).

| Run | Build | Hold, worst case | As a rate (1 / hold) | Entries back to back (not masked) | Release lag, worst case |
|---|---|---:|---:|---:|---:|
| Stress, 3,660 s | instrumented | 7,044 cycles (110.1 µs) | 9.1 kHz | 26,010 cycles (406.4 µs) | 9,236 cycles (144.3 µs) |
| Release timing, 3,660 s | instrumented | 6,496 cycles (101.5 µs) | 9.9 kHz | 21,659 cycles (338.4 µs) | 8,573 cycles (134.0 µs) |
| Stress, 180 s | lean (no trace instrumentation) | 6,312 cycles (98.6 µs) | 10.1 kHz | 332.2 µs | not recorded here |

L_kernel as exported to the Analyzer (the hold with the 25 % margin of R34.6):

| Record | L_kernel | In time | As a rate |
|---|---:|---:|---:|
| Stress, 3,660 s | 8,805 cycles | 137.6 µs | 7.3 kHz |
| Release timing, 3,660 s | 8,120 cycles | 126.9 µs | 7.9 kHz |
| At the span basis, before the change (180 s stress run of 2026-11-21) | 28,700 cycles | 448.4 µs | 2.2 kHz |

The rate column is the reciprocal of the time: a Kernel-level interrupt can be delayed by up to one hold, so it bounds the event rate for which the Kernel can promise a response within one period of that rate. It is not a measured throughput.

What the numbers show. The worst hold is about 0.11 ms, 1.1 % of `control`'s 10 ms deadline. In the stress run it is a timer entry that enforces a Budget overrun and restarts the Partition, followed by the ended Job's unwind (2 entries); in the release-timing run it is one timer entry. The stated goal of 50 to 100 µs is met only at its upper edge and only in the lean build.

Caveats a reviewer needs:

| Caveat | Effect |
|---|---|
| The holds are measured maxima under the demo's stress and release-timing workloads, not a static bound | A longer hold on another workload or another interleaving is not excluded |
| The hour-long figures are from HIL builds with `trace-ram` and `trace-pins`; the instrumentation's cost is inside the hold | The lean figure (98.6 µs) is from a 180 s run only; no hour-long lean run exists |
| The change of basis made no Kernel path shorter | The drop from 28,700 to 8,805 cycles is a correction of what is counted, not a speed-up |
| No dedicated probe fires a Kernel-level interrupt into the middle of a run of entries | That a pending interrupt is taken between chained entries rests on the exception model, the handlers' code and the recorded composition of the longest runs |
| The stress run's Analyzer step rejects its inputs: `part2.filter`'s WCET with margin (51,239 cycles) exceeds its Budget (51,200), R28.4 | The observed response times of that run are not judged against measured WCRTs |
| `cargo xtask verify` was not rerun at `44df784c` | The job of the same day at `00d3c1fb` ended with 3 violations, none in the Kernel (two from a clean-checkout tooling defect since fixed, one from an expired UPPAAL licence lease since renewed) |

## Not completed, and why

- Items 3 to 8: not started; the instructions ask for a check-in after items 1 and 2.
- Item 1: the Endpoints decision is the default, not yet confirmed. The 1,446 Kernel lines added after the tag have not been reviewed for fixes unrelated to Endpoints.
- Item 1: no `cargo xtask verify` was run at the tag in this session.
- Item 2: the timings are single cold runs on a machine with an IDE active; no repeat runs.
- Item 2: three of the 31 mutation kills are solver resource-limit failures.
- Rig state on 2026-11-12, which bounds items 3, 4, 5, 7 and 8: `probe-rs list` shows board A (target, serial 001050217737) and board C (spare, 001050211852); board B (stimulus, 001050279833) is not attached. The fx2lafw analyzer is attached. Runs that need the stimulus board, which includes the release-timing campaigns of item 5, cannot run until B is attached or C is wired and assigned in its place.
