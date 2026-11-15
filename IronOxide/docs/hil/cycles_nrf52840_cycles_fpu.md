# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:fe6647a6a14b3c67a6ea0f51e9ce74d999ec52f54b746722b8eacdc4e6a5e098 (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.8 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 75866 cycles (1.185 ms), filter mean 82600 cycles (1.291 ms): difference 6734 cycles (8.88 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 6734 cycles (0.105 ms) per log Job over its 49 Jobs, interrupted by 119 Kernel entries: 2773 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75238 max 82534, log min 81578 max 87207 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 130113 ticks (520452 cycles) against its largest execution 87207 cycles; control's largest response 24713 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 6993 / 109.3 | 3408 | 2^12:198 2^13:1 |
| 2 | 99 | 5241 / 81.9 | 3459 | 2^12:89 2^13:10 |
| 3 | 49 | 3644 / 56.9 | 3435 | 2^12:49 |
| 5 | 49 | 3644 / 56.9 | 3435 | 2^12:49 |
| 6 | 19 | 6993 / 109.3 | 3647 | 2^12:18 2^13:1 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 1.545 | 0 | 0 | 0 |
| 2 | 99 | 6.642 | 0 | 0 | 0 |
| 3 | 49 | 9.361 | 0 | 0 | 0 |
| 4 | 109 | 5.845 | 110 | 2148 | 2023 |
| 5 | 49 | 8.132 | 0 | 0 | 0 |
| 6 | 20 | 1.774 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82534 / 1.290 (min 75238, mean 75866) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 340133 / 5.315 (min 335878, mean 336461) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 80245 / 1.254 (min 75380, mean 76233) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 109 | 13479 / 0.211 (min 3358, mean 4888) | 0 above the Budget; 2^12:66 2^13:42 2^14:1 |
| Task 5 entry | 49 | 87207 / 1.363 (min 81578, mean 82600) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 11426 / 0.179 (min 7826, mean 10640) | 0 above the Budget; 2^13:4 2^14:16 |
| Task 2 on Resource 0 | 99 | 325005 / 5.078 | |
| Task 2 on Resource 1 | 198 | 479 / 0.007 | |
| Task 6 on Resource 2 | 20 | 488 / 0.008 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 526 | 3060 / 47.81 | 2891 |
| timer | 533 | 3311 / 51.73 | 2952 |
| intake | 110 | 2260 / 35.31 | 2136 |
| lock | 317 | 453 / 7.08 | 451 |
| unlock | 317 | 1836 / 28.69 | 1647 |
| job_end | 526 | 1944 / 30.38 | 1749 |
| now | 40859 | 229 / 3.58 | 226 |
| dma | 20 | 709 / 11.08 | 707 |
| epilogue | 526 | 1248 / 19.50 | 683 |
| view | 695 | 329 / 5.14 | 296 |

Entries that interrupted an executing Job: Task 0: 25, Task 2: 256, Task 3: 14, Task 4: 45, Task 5: 119, Task 6: 16

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3412 cycles
- Task 5 at 1924.000 ms, 3412 cycles
- Task 2 at 1925.000 ms, 3425 cycles
- Task 0 at 1930.000 ms, 3497 cycles
- Task 0 at 1940.000 ms, 3152 cycles
- Task 2 at 1945.000 ms, 3156 cycles
- Task 0 at 1950.000 ms, 3437 cycles
- Task 0 at 1960.000 ms, 3152 cycles
- Task 2 at 1965.000 ms, 3080 cycles
- Task 0 at 1970.000 ms, 3585 cycles
- Task 0 at 1980.000 ms, 3076 cycles
- Task 0 at 1990.000 ms, 3004 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles-fpu/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=109
echo=0
blocked_releases=0
filter=49
filter_state_bits=1120384580
sampler=20
dma_ok=20
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=3
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=10
  task=255
  at_ms=2000
  detail=0
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=2000
  detail=0
  prs_lo=0
  prs_hi=0
RSK-OK
Firmware exited successfully
     Finished in 8.04s
```
