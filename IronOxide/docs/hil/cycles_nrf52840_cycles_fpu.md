# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:9fd5452f7dca1c2f9df4f5282e4d0da5180056405814c1e17542a5e28baf07c6 (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 11.9 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77045 cycles (1.204 ms), filter mean 82372 cycles (1.287 ms): difference 5327 cycles (6.91 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5327 cycles (0.083 ms) per log Job over its 49 Jobs, interrupted by 137 Kernel entries: 1905 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74949 max 82459, log min 80775 max 85345 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 145825 ticks (583300 cycles) against its largest execution 85345 cycles; control's largest response 40753 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 3928 / 61.4 | 3327 | 2^12:199 |
| 2 | 99 | 3432 / 53.6 | 3212 | 2^12:99 |
| 3 | 49 | 3384 / 52.9 | 3332 | 2^12:49 |
| 5 | 49 | 3384 / 52.9 | 3332 | 2^12:49 |
| 6 | 19 | 3392 / 53.0 | 3327 | 2^12:19 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.547 | 0 | 0 | 0 |
| 2 | 99 | 7.699 | 0 | 0 | 0 |
| 3 | 49 | 10.333 | 0 | 0 | 0 |
| 4 | 110 | 6.156 | 111 | 2115 | 1975 |
| 5 | 49 | 9.114 | 0 | 0 | 0 |
| 6 | 20 | 1.740 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82459 / 1.288 (min 74949, mean 77045) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 405080 / 6.329 (min 398206, mean 398674) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 78425 / 1.225 (min 75061, mean 75430) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 6778 / 0.106 (min 3036, mean 4493) | 0 above the Budget; 2^12:66 2^13:44 |
| Task 5 entry | 49 | 85345 / 1.334 (min 80775, mean 82372) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 10426 / 0.163 (min 6982, mean 10004) | 0 above the Budget; 2^13:2 2^14:18 |
| Task 2 on Resource 0 | 99 | 385284 / 6.020 | |
| Task 2 on Resource 1 | 198 | 275 / 0.004 | |
| Task 6 on Resource 2 | 20 | 284 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 527 | 2848 / 44.50 | 2701 |
| timer | 630 | 3275 / 51.17 | 2924 |
| intake | 111 | 2244 / 35.06 | 2101 |
| lock | 317 | 466 / 7.28 | 464 |
| unlock | 317 | 1861 / 29.08 | 1667 |
| job_end | 527 | 1982 / 30.97 | 1779 |
| now | 66452 | 243 / 3.80 | 242 |
| dma | 20 | 710 / 11.09 | 710 |
| epilogue | 527 | 1204 / 18.81 | 663 |
| view | 703 | 327 / 5.11 | 293 |

Entries that interrupted an executing Job: Task 0: 112, Task 2: 277, Task 3: 5, Task 4: 44, Task 5: 137, Task 6: 18

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3380 cycles
- Task 5 at 1924.000 ms, 3380 cycles
- Task 2 at 1925.000 ms, 3356 cycles
- Task 0 at 1930.000 ms, 3612 cycles
- Task 0 at 1940.000 ms, 3116 cycles
- Task 2 at 1945.000 ms, 3056 cycles
- Task 0 at 1950.000 ms, 3348 cycles
- Task 0 at 1960.000 ms, 3116 cycles
- Task 2 at 1965.000 ms, 2980 cycles
- Task 0 at 1970.000 ms, 3088 cycles
- Task 0 at 1980.000 ms, 2980 cycles
- Task 0 at 1990.000 ms, 2940 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles-fpu/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=110
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
     Finished in 7.32s
```
