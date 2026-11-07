# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:6f3cf5476d3c1122cd8a3e01300de3e10e68c4f2fdb6c615cdb6f4859541ec14 (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.6 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77294 cycles (1.208 ms), filter mean 82662 cycles (1.292 ms): difference 5368 cycles (6.95 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5368 cycles (0.084 ms) per log Job over its 49 Jobs, interrupted by 130 Kernel entries: 2023 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75090 max 83002, log min 81326 max 86204 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 145931 ticks (583724 cycles) against its largest execution 86204 cycles; control's largest response 41141 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4124 / 64.4 | 3467 | 2^12:198 2^13:1 |
| 2 | 99 | 3524 / 55.1 | 3305 | 2^12:99 |
| 3 | 49 | 3463 / 54.1 | 3420 | 2^12:49 |
| 5 | 49 | 3463 / 54.1 | 3420 | 2^12:49 |
| 6 | 19 | 3467 / 54.2 | 3420 | 2^12:19 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.571 | 0 | 0 | 0 |
| 2 | 99 | 7.712 | 0 | 0 | 0 |
| 3 | 49 | 10.345 | 0 | 0 | 0 |
| 4 | 110 | 6.226 | 110 | 2170 | 2037 |
| 5 | 49 | 9.121 | 0 | 0 | 0 |
| 6 | 20 | 1.764 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 83002 / 1.297 (min 75090, mean 77294) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 406142 / 6.346 (min 399318, mean 400230) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 78772 / 1.231 (min 75204, mean 75666) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 7076 / 0.111 (min 3180, mean 4664) | 0 above the Budget; 2^12:67 2^13:43 |
| Task 5 entry | 49 | 86204 / 1.347 (min 81326, mean 82662) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 11111 / 0.174 (min 7515, mean 10682) | 0 above the Budget; 2^13:2 2^14:18 |
| Task 2 on Resource 0 | 99 | 385738 / 6.027 | |
| Task 2 on Resource 1 | 198 | 402 / 0.006 | |
| Task 6 on Resource 2 | 20 | 411 / 0.006 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 527 | 2870 / 44.84 | 2738 |
| timer | 628 | 3279 / 51.23 | 2989 |
| intake | 110 | 2295 / 35.86 | 2163 |
| lock | 317 | 465 / 7.27 | 464 |
| unlock | 317 | 1845 / 28.83 | 1664 |
| job_end | 527 | 1982 / 30.97 | 1776 |
| now | 54151 | 240 / 3.75 | 240 |
| dma | 20 | 734 / 11.47 | 734 |
| epilogue | 527 | 1200 / 18.75 | 668 |
| view | 702 | 330 / 5.16 | 297 |

Entries that interrupted an executing Job: Task 0: 112, Task 2: 286, Task 3: 6, Task 4: 43, Task 5: 130, Task 6: 18

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3407 cycles
- Task 5 at 1924.000 ms, 3407 cycles
- Task 2 at 1925.000 ms, 3448 cycles
- Task 0 at 1930.000 ms, 3684 cycles
- Task 0 at 1940.000 ms, 3147 cycles
- Task 2 at 1945.000 ms, 3203 cycles
- Task 0 at 1950.000 ms, 3372 cycles
- Task 0 at 1960.000 ms, 3147 cycles
- Task 2 at 1965.000 ms, 3079 cycles
- Task 0 at 1970.000 ms, 3588 cycles
- Task 0 at 1980.000 ms, 3127 cycles
- Task 0 at 1990.000 ms, 3039 cycles


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
     Finished in 7.71s
```
