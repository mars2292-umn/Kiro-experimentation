# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:95476a3e75d1025f107b5332e8908b4263fb251c92f55318c40566c2d78f0b02 (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.9 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77291 cycles (1.208 ms), filter mean 81640 cycles (1.276 ms): difference 4349 cycles (5.63 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 4349 cycles (0.068 ms) per log Job over its 49 Jobs, interrupted by 98 Kernel entries: 2174 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75252 max 79324, log min 81637 max 81717 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 144130 ticks (576520 cycles) against its largest execution 81717 cycles; control's largest response 41235 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 3976 / 62.1 | 3561 | 2^12:199 |
| 2 | 99 | 3580 / 55.9 | 3360 | 2^12:99 |
| 3 | 49 | 3503 / 54.7 | 3503 | 2^12:49 |
| 5 | 49 | 3503 / 54.7 | 3503 | 2^12:49 |
| 6 | 19 | 3503 / 54.7 | 3495 | 2^12:19 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.577 | 0 | 0 | 0 |
| 2 | 99 | 7.693 | 0 | 0 | 0 |
| 3 | 49 | 10.236 | 0 | 0 | 0 |
| 5 | 49 | 9.008 | 0 | 0 | 0 |
| 6 | 20 | 1.771 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 79324 / 1.239 (min 75252, mean 77291) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 403959 / 6.312 (min 400219, mean 400454) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 75446 / 1.179 (min 75390, mean 75391) | 0 above the Budget; 2^15:49 |
| Task 5 entry | 49 | 81717 / 1.277 (min 81637, mean 81640) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 11382 / 0.178 (min 11378, mean 11378) | 0 above the Budget; 2^14:20 |
| Task 2 on Resource 0 | 99 | 385958 / 6.031 | |
| Task 2 on Resource 1 | 198 | 474 / 0.007 | |
| Task 6 on Resource 2 | 20 | 483 / 0.008 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 417 | 3039 / 47.48 | 2894 |
| timer | 483 | 3284 / 51.31 | 3072 |
| lock | 317 | 457 / 7.14 | 456 |
| unlock | 317 | 1767 / 27.61 | 1639 |
| job_end | 417 | 1890 / 29.53 | 1739 |
| now | 50422 | 220 / 3.44 | 220 |
| dma | 20 | 714 / 11.16 | 712 |
| epilogue | 417 | 1221 / 19.08 | 707 |
| view | 565 | 329 / 5.14 | 289 |

Entries that interrupted an executing Job: Task 0: 101, Task 2: 204, Task 5: 98, Task 6: 20

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3503 cycles
- Task 5 at 1924.000 ms, 3503 cycles
- Task 2 at 1925.000 ms, 3504 cycles
- Task 0 at 1930.000 ms, 3896 cycles
- Task 0 at 1940.000 ms, 3215 cycles
- Task 2 at 1945.000 ms, 3219 cycles
- Task 0 at 1950.000 ms, 3836 cycles
- Task 0 at 1960.000 ms, 3215 cycles
- Task 2 at 1965.000 ms, 3151 cycles
- Task 0 at 1970.000 ms, 3596 cycles
- Task 0 at 1980.000 ms, 3147 cycles
- Task 0 at 1990.000 ms, 3111 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles-fpu/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=0
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
     Finished in 8.02s
```
