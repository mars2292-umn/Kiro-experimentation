# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:558415dc1b2517cbfd783bd97a40f3208094cbd2b781c93715e84e9a4b90f160 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.5 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77244 cycles (1.207 ms), log mean 83391 cycles (1.303 ms): difference 6147 cycles (7.96 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 6147 cycles (0.096 ms) per log Job over its 49 Jobs, interrupted by 149 Kernel entries: 2021 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75058 max 79367, log min 81167 max 88401 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 149443 ticks (597772 cycles) against its largest execution 88401 cycles; control's largest response 41207 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4412 / 68.9 | 3500 | 2^12:187 2^13:12 |
| 2 | 99 | 3764 / 58.8 | 3393 | 2^12:99 |
| 3 | 49 | 3286 / 51.3 | 3234 | 2^12:49 |
| 5 | 79 | 4412 / 68.9 | 3631 | 2^12:72 2^13:7 |
| 6 | 19 | 3842 / 60.0 | 3785 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.575 | 0 | 0 | 0 |
| 2 | 99 | 7.690 | 0 | 0 | 0 |
| 3 | 49 | 9.340 | 0 | 0 | 0 |
| 4 | 110 | 7.779 | 110 | 2184 | 2015 |
| 5 | 80 | 8.029 | 0 | 0 | 0 |
| 6 | 20 | 1.927 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 79367 / 1.240 (min 75058, mean 77244) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 403403 / 6.303 (min 399358, mean 399839) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 88401 / 1.381 (min 81167, mean 83391) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 7092 / 0.111 (min 3151, mean 4640) | 0 above the Budget; 2^12:67 2^13:43 |
| Task 5 entry | 80 | 10247 / 0.160 (min 6299, mean 7350) | 0 above the Budget; 2^13:60 2^14:20 |
| Task 6 entry | 20 | 11054 / 0.173 (min 7454, mean 7821) | 0 above the Budget; 2^13:18 2^14:2 |
| Task 2 on Resource 0 | 99 | 388848 / 6.076 | |
| Task 2 on Resource 1 | 198 | 402 / 0.006 | |
| Task 5 on Resource 2 | 80 | 3921 / 0.061 | |
| Task 6 on Resource 2 | 20 | 411 / 0.006 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 2805 / 43.83 | 2668 |
| timer | 634 | 3653 / 57.08 | 3062 |
| intake | 110 | 2303 / 35.98 | 2133 |
| lock | 397 | 471 / 7.36 | 469 |
| unlock | 397 | 1847 / 28.86 | 1647 |
| job_end | 558 | 2027 / 31.67 | 1762 |
| now | 54154 | 237 / 3.70 | 235 |
| dma | 20 | 716 / 11.19 | 715 |
| epilogue | 558 | 1209 / 18.89 | 670 |
| view | 749 | 329 / 5.14 | 300 |

Entries that interrupted an executing Job: Task 0: 110, Task 2: 268, Task 3: 149, Task 4: 43, Task 5: 20, Task 6: 2

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3696 cycles
- Task 0 at 1930.000 ms, 3672 cycles
- Task 0 at 1940.000 ms, 3222 cycles
- Task 2 at 1945.000 ms, 3294 cycles
- Task 0 at 1950.000 ms, 3740 cycles
- Task 5 at 1950.000 ms, 3740 cycles
- Task 0 at 1960.000 ms, 3222 cycles
- Task 2 at 1965.000 ms, 3262 cycles
- Task 0 at 1970.000 ms, 3308 cycles
- Task 5 at 1975.000 ms, 3198 cycles
- Task 0 at 1980.000 ms, 3190 cycles
- Task 0 at 1990.000 ms, 3154 cycles


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
     Finished in 7.72s
```
