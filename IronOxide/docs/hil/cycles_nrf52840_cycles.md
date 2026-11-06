# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:4657c630218405b78b427afee69bdec5f8878d3eca525386bd3290e444091383 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 11.3 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 76902 cycles (1.202 ms), log mean 81442 cycles (1.273 ms): difference 4541 cycles (5.90 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 4541 cycles (0.071 ms) per log Job over its 49 Jobs, interrupted by 118 Kernel entries: 1885 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74921 max 78900, log min 80661 max 82938 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 145762 ticks (583048 cycles) against its largest execution 82938 cycles; control's largest response 40899 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4027 / 62.9 | 3465 | 2^12:199 |
| 2 | 99 | 3771 / 58.9 | 3359 | 2^12:99 |
| 3 | 49 | 3188 / 49.8 | 3188 | 2^12:49 |
| 5 | 79 | 3947 / 61.7 | 3576 | 2^12:79 |
| 6 | 19 | 3736 / 58.4 | 3736 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.556 | 0 | 0 | 0 |
| 2 | 99 | 7.663 | 0 | 0 | 0 |
| 3 | 49 | 9.110 | 0 | 0 | 0 |
| 5 | 80 | 7.821 | 0 | 0 | 0 |
| 6 | 20 | 1.882 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 78900 / 1.233 (min 74921, mean 76902) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 402269 / 6.285 (min 397894, mean 398393) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 82938 / 1.296 (min 80661, mean 81442) | 0 above the Budget; 2^15:49 |
| Task 5 entry | 80 | 9620 / 0.150 (min 5872, mean 6942) | 0 above the Budget; 2^13:59 2^14:21 |
| Task 6 entry | 20 | 6869 / 0.107 (min 6869, mean 6869) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 385316 / 6.021 | |
| Task 2 on Resource 1 | 198 | 268 / 0.004 | |
| Task 5 on Resource 2 | 80 | 296 / 0.005 | |
| Task 6 on Resource 2 | 20 | 277 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 448 | 2747 / 42.92 | 2620 |
| timer | 503 | 3592 / 56.12 | 3093 |
| lock | 397 | 454 / 7.09 | 453 |
| unlock | 397 | 1761 / 27.52 | 1623 |
| job_end | 448 | 1954 / 30.53 | 1748 |
| now | 68760 | 234 / 3.66 | 231 |
| dma | 20 | 697 / 10.89 | 697 |
| epilogue | 448 | 1221 / 19.08 | 696 |
| view | 616 | 327 / 5.11 | 290 |

Entries that interrupted an executing Job: Task 0: 101, Task 2: 204, Task 3: 118, Task 5: 21

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3683 cycles
- Task 0 at 1930.000 ms, 3755 cycles
- Task 0 at 1940.000 ms, 3196 cycles
- Task 2 at 1945.000 ms, 3196 cycles
- Task 0 at 1950.000 ms, 3759 cycles
- Task 5 at 1950.000 ms, 3759 cycles
- Task 0 at 1960.000 ms, 3196 cycles
- Task 2 at 1965.000 ms, 3156 cycles
- Task 0 at 1970.000 ms, 3367 cycles
- Task 5 at 1975.000 ms, 3152 cycles
- Task 0 at 1980.000 ms, 3160 cycles
- Task 0 at 1990.000 ms, 3124 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=0
echo=0
blocked_releases=0
filter=80
filter_state_bits=1120403155
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
     Finished in 6.72s
```
