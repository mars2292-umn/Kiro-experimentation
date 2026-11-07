# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:2a55da0b01f7d0e0e816de60a91f6edbc1c5f5fea6e15b8b6d9c486b35d79027 (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.5 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77455 cycles (1.210 ms), filter mean 82564 cycles (1.290 ms): difference 5109 cycles (6.60 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5109 cycles (0.080 ms) per log Job over its 49 Jobs, interrupted by 126 Kernel entries: 1987 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75056 max 82844, log min 81268 max 86927 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 146817 ticks (587268 cycles) against its largest execution 86927 cycles; control's largest response 41171 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 3902 / 61.0 | 3323 | 2^12:199 |
| 2 | 99 | 3550 / 55.5 | 3290 | 2^12:99 |
| 3 | 49 | 3438 / 53.7 | 3392 | 2^12:49 |
| 5 | 49 | 3438 / 53.7 | 3392 | 2^12:49 |
| 6 | 19 | 3458 / 54.0 | 3411 | 2^12:19 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.573 | 0 | 0 | 0 |
| 2 | 99 | 7.688 | 0 | 0 | 0 |
| 3 | 49 | 10.400 | 0 | 0 | 0 |
| 4 | 110 | 7.723 | 111 | 2107 | 2002 |
| 5 | 49 | 9.176 | 0 | 0 | 0 |
| 6 | 20 | 1.759 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82844 / 1.294 (min 75056, mean 77455) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 402899 / 6.295 (min 399163, mean 399974) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 79926 / 1.249 (min 75167, mean 76033) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 7045 / 0.110 (min 3149, mean 4640) | 0 above the Budget; 2^12:67 2^13:43 |
| Task 5 entry | 49 | 86927 / 1.358 (min 81268, mean 82564) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 11054 / 0.173 (min 7454, mean 10637) | 0 above the Budget; 2^13:2 2^14:18 |
| Task 2 on Resource 0 | 99 | 385733 / 6.027 | |
| Task 2 on Resource 1 | 198 | 3174 / 0.050 | |
| Task 6 on Resource 2 | 20 | 401 / 0.006 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 527 | 2832 / 44.25 | 2700 |
| timer | 624 | 3252 / 50.81 | 2977 |
| intake | 111 | 2227 / 34.80 | 2120 |
| lock | 317 | 480 / 7.50 | 479 |
| unlock | 317 | 1813 / 28.33 | 1641 |
| job_end | 527 | 1974 / 30.84 | 1774 |
| now | 55102 | 236 / 3.69 | 233 |
| dma | 20 | 735 / 11.48 | 735 |
| epilogue | 527 | 1219 / 19.05 | 660 |
| view | 702 | 330 / 5.16 | 297 |

Entries that interrupted an executing Job: Task 0: 124, Task 2: 282, Task 3: 15, Task 4: 43, Task 5: 126, Task 6: 18

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3386 cycles
- Task 5 at 1924.000 ms, 3386 cycles
- Task 2 at 1925.000 ms, 3482 cycles
- Task 0 at 1930.000 ms, 3722 cycles
- Task 0 at 1940.000 ms, 3126 cycles
- Task 2 at 1945.000 ms, 3178 cycles
- Task 0 at 1950.000 ms, 3398 cycles
- Task 0 at 1960.000 ms, 3126 cycles
- Task 2 at 1965.000 ms, 3058 cycles
- Task 0 at 1970.000 ms, 3638 cycles
- Task 0 at 1980.000 ms, 3102 cycles
- Task 0 at 1990.000 ms, 3018 cycles


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
