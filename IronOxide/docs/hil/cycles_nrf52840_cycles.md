# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:27b745b93db8c469be59993563fb374b0e83fdee6cc9e5a7c224a6c4f557d056 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 11.8 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77437 cycles (1.210 ms), log mean 83005 cycles (1.297 ms): difference 5569 cycles (7.19 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5569 cycles (0.087 ms) per log Job over its 49 Jobs, interrupted by 157 Kernel entries: 1738 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74921 max 82580, log min 80641 max 87315 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 149220 ticks (596880 cycles) against its largest execution 87315 cycles; control's largest response 41697 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4299 / 67.2 | 3429 | 2^12:198 2^13:1 |
| 2 | 99 | 3723 / 58.2 | 3317 | 2^12:99 |
| 3 | 49 | 3188 / 49.8 | 3151 | 2^12:49 |
| 5 | 79 | 4299 / 67.2 | 3539 | 2^12:78 2^13:1 |
| 6 | 19 | 3732 / 58.3 | 3703 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.606 | 0 | 0 | 0 |
| 2 | 99 | 7.712 | 0 | 0 | 0 |
| 3 | 49 | 9.326 | 0 | 0 | 0 |
| 4 | 110 | 6.547 | 111 | 2019 | 1894 |
| 5 | 80 | 8.038 | 0 | 0 | 0 |
| 6 | 20 | 1.882 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82580 / 1.290 (min 74921, mean 77437) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 405501 / 6.336 (min 397770, mean 398436) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 87315 / 1.364 (min 80641, mean 83005) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 6851 / 0.107 (min 3040, mean 4187) | 0 above the Budget; 2^12:76 2^13:34 |
| Task 5 entry | 80 | 9715 / 0.152 (min 5848, mean 6823) | 0 above the Budget; 2^13:61 2^14:19 |
| Task 6 entry | 20 | 6869 / 0.107 (min 6841, mean 6849) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 392168 / 6.128 | |
| Task 2 on Resource 1 | 198 | 268 / 0.004 | |
| Task 5 on Resource 2 | 80 | 296 / 0.005 | |
| Task 6 on Resource 2 | 20 | 277 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 2795 / 43.67 | 2642 |
| timer | 652 | 3572 / 55.81 | 3011 |
| intake | 111 | 2089 / 32.64 | 1963 |
| lock | 397 | 454 / 7.09 | 453 |
| unlock | 397 | 1761 / 27.52 | 1581 |
| job_end | 558 | 1954 / 30.53 | 1714 |
| now | 68369 | 234 / 3.66 | 231 |
| dma | 20 | 697 / 10.89 | 697 |
| epilogue | 558 | 1221 / 19.08 | 678 |
| view | 754 | 327 / 5.11 | 297 |

Entries that interrupted an executing Job: Task 0: 137, Task 2: 279, Task 3: 157, Task 4: 34, Task 5: 19

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3639 cycles
- Task 0 at 1930.000 ms, 3647 cycles
- Task 0 at 1940.000 ms, 3144 cycles
- Task 2 at 1945.000 ms, 3144 cycles
- Task 0 at 1950.000 ms, 3643 cycles
- Task 5 at 1950.000 ms, 3643 cycles
- Task 0 at 1960.000 ms, 3144 cycles
- Task 2 at 1965.000 ms, 3108 cycles
- Task 0 at 1970.000 ms, 3251 cycles
- Task 5 at 1975.000 ms, 3104 cycles
- Task 0 at 1980.000 ms, 3112 cycles
- Task 0 at 1990.000 ms, 3124 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=110
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
     Finished in 7.00s
```
