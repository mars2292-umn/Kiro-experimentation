# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:e09621efc11b8da760a9d7517ffafccbf95f5718ed6775898e3c8e32ae7649bc (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.2 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77224 cycles (1.207 ms), filter mean 81937 cycles (1.280 ms): difference 4713 cycles (6.10 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 4713 cycles (0.074 ms) per log Job over its 49 Jobs, interrupted by 124 Kernel entries: 1862 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74886 max 82467, log min 80837 max 86139 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 146697 ticks (586788 cycles) against its largest execution 86139 cycles; control's largest response 41467 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 3903 / 61.0 | 3379 | 2^12:199 |
| 2 | 99 | 3495 / 54.6 | 3279 | 2^12:99 |
| 3 | 49 | 3420 / 53.4 | 3379 | 2^12:49 |
| 5 | 49 | 3420 / 53.4 | 3379 | 2^12:49 |
| 6 | 19 | 3440 / 53.8 | 3394 | 2^12:19 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.592 | 0 | 0 | 0 |
| 2 | 99 | 7.698 | 0 | 0 | 0 |
| 3 | 49 | 10.387 | 0 | 0 | 0 |
| 4 | 109 | 7.641 | 110 | 2184 | 1986 |
| 5 | 49 | 9.169 | 0 | 0 | 0 |
| 6 | 20 | 1.739 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82467 / 1.289 (min 74886, mean 77224) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 405029 / 6.329 (min 397932, mean 398750) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 79449 / 1.241 (min 75027, mean 75500) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 109 | 6780 / 0.106 (min 3059, mean 4461) | 0 above the Budget; 2^12:66 2^13:43 |
| Task 5 entry | 49 | 86139 / 1.346 (min 80837, mean 81937) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 10352 / 0.162 (min 6860, mean 9952) | 0 above the Budget; 2^13:2 2^14:18 |
| Task 2 on Resource 0 | 99 | 385263 / 6.020 | |
| Task 2 on Resource 1 | 198 | 264 / 0.004 | |
| Task 6 on Resource 2 | 20 | 273 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 526 | 2824 / 44.12 | 2701 |
| timer | 622 | 3288 / 51.38 | 2998 |
| intake | 110 | 2307 / 36.05 | 2110 |
| lock | 317 | 463 / 7.23 | 462 |
| unlock | 317 | 1814 / 28.34 | 1638 |
| job_end | 526 | 1943 / 30.36 | 1737 |
| now | 67924 | 242 / 3.78 | 240 |
| dma | 20 | 703 / 10.98 | 701 |
| epilogue | 526 | 1211 / 18.92 | 652 |
| view | 700 | 330 / 5.16 | 297 |

Entries that interrupted an executing Job: Task 0: 123, Task 2: 279, Task 3: 10, Task 4: 43, Task 5: 124, Task 6: 18

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3372 cycles
- Task 5 at 1924.000 ms, 3372 cycles
- Task 2 at 1925.000 ms, 3423 cycles
- Task 0 at 1930.000 ms, 3783 cycles
- Task 0 at 1940.000 ms, 3116 cycles
- Task 2 at 1945.000 ms, 3116 cycles
- Task 0 at 1950.000 ms, 3447 cycles
- Task 0 at 1960.000 ms, 3116 cycles
- Task 2 at 1965.000 ms, 3040 cycles
- Task 0 at 1970.000 ms, 3227 cycles
- Task 0 at 1980.000 ms, 3040 cycles
- Task 0 at 1990.000 ms, 2968 cycles


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
     Finished in 7.34s
```
