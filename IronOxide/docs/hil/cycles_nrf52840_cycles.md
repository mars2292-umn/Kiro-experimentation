# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:7c7a06ed2e038ac4e21e93fe47531f1b9462331a33116caa26e18eb9ade07728 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 11.8 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77428 cycles (1.210 ms), log mean 82408 cycles (1.288 ms): difference 4980 cycles (6.43 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 4980 cycles (0.078 ms) per log Job over its 49 Jobs, interrupted by 145 Kernel entries: 1683 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74829 max 82676, log min 80585 max 87843 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148382 ticks (593528 cycles) against its largest execution 87843 cycles; control's largest response 41601 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4327 / 67.6 | 3598 | 2^12:185 2^13:14 |
| 2 | 99 | 3827 / 59.8 | 3419 | 2^12:99 |
| 3 | 49 | 3288 / 51.4 | 3246 | 2^12:49 |
| 5 | 79 | 4327 / 67.6 | 3695 | 2^12:65 2^13:14 |
| 6 | 19 | 3848 / 60.1 | 3801 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.600 | 0 | 0 | 0 |
| 2 | 99 | 7.704 | 0 | 0 | 0 |
| 3 | 49 | 9.274 | 0 | 0 | 0 |
| 4 | 110 | 7.417 | 111 | 2056 | 1896 |
| 5 | 80 | 7.973 | 0 | 0 | 0 |
| 6 | 20 | 1.883 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82676 / 1.292 (min 74829, mean 77428) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 402045 / 6.282 (min 397686, mean 398140) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 87843 / 1.373 (min 80585, mean 82408) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 6871 / 0.107 (min 2976, mean 4391) | 0 above the Budget; 2^12:68 2^13:42 |
| Task 5 entry | 80 | 13805 / 0.216 (min 5776, mean 6979) | 0 above the Budget; 2^13:59 2^14:21 |
| Task 6 entry | 20 | 6789 / 0.106 (min 6765, mean 6769) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 385318 / 6.021 | |
| Task 2 on Resource 1 | 198 | 268 / 0.004 | |
| Task 5 on Resource 2 | 80 | 296 / 0.005 | |
| Task 6 on Resource 2 | 20 | 277 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 2715 / 42.42 | 2558 |
| timer | 655 | 3666 / 57.28 | 3119 |
| intake | 111 | 2124 / 33.19 | 1963 |
| lock | 397 | 452 / 7.06 | 450 |
| unlock | 397 | 1796 / 28.06 | 1615 |
| job_end | 558 | 1999 / 31.23 | 1749 |
| now | 69374 | 226 / 3.53 | 225 |
| dma | 20 | 693 / 10.83 | 693 |
| epilogue | 558 | 1162 / 18.16 | 656 |

Entries that interrupted an executing Job: Task 0: 134, Task 2: 270, Task 3: 145, Task 4: 42, Task 5: 25

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3739 cycles
- Task 0 at 1930.000 ms, 4091 cycles
- Task 0 at 1940.000 ms, 3240 cycles
- Task 2 at 1945.000 ms, 3240 cycles
- Task 0 at 1950.000 ms, 3683 cycles
- Task 5 at 1950.000 ms, 3683 cycles
- Task 0 at 1960.000 ms, 3240 cycles
- Task 2 at 1965.000 ms, 3252 cycles
- Task 0 at 1970.000 ms, 3603 cycles
- Task 5 at 1975.000 ms, 3200 cycles
- Task 0 at 1980.000 ms, 3204 cycles
- Task 0 at 1990.000 ms, 3132 cycles


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
     Finished in 7.01s
```
