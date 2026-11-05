# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:77469f734b502a042021c607f339404d1d5d07017c0009602893f4f5b7b30064 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 11.8 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77004 cycles (1.203 ms), log mean 82874 cycles (1.295 ms): difference 5870 cycles (7.62 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5870 cycles (0.092 ms) per log Job over its 49 Jobs, interrupted by 151 Kernel entries: 1905 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74877 max 81463, log min 80586 max 87848 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148175 ticks (592700 cycles) against its largest execution 87848 cycles; control's largest response 40732 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4212 / 65.8 | 3450 | 2^12:198 2^13:1 |
| 2 | 99 | 3728 / 58.2 | 3321 | 2^12:99 |
| 3 | 49 | 3183 / 49.7 | 3144 | 2^12:49 |
| 5 | 79 | 4212 / 65.8 | 3558 | 2^12:78 2^13:1 |
| 6 | 19 | 3735 / 58.4 | 3696 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.546 | 0 | 0 | 0 |
| 2 | 99 | 7.699 | 0 | 0 | 0 |
| 3 | 49 | 9.261 | 0 | 0 | 0 |
| 4 | 110 | 7.184 | 111 | 2011 | 1865 |
| 5 | 80 | 7.969 | 0 | 0 | 0 |
| 6 | 20 | 1.879 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 81463 / 1.273 (min 74877, mean 77004) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 405241 / 6.332 (min 397777, mean 398204) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 87848 / 1.373 (min 80586, mean 82874) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 6798 / 0.106 (min 3020, mean 4394) | 0 above the Budget; 2^12:68 2^13:42 |
| Task 5 entry | 80 | 9648 / 0.151 (min 5786, mean 7029) | 0 above the Budget; 2^13:55 2^14:25 |
| Task 6 entry | 20 | 10981 / 0.172 (min 6777, mean 7201) | 0 above the Budget; 2^13:18 2^14:2 |
| Task 2 on Resource 0 | 99 | 385326 / 6.021 | |
| Task 2 on Resource 1 | 198 | 3743 / 0.058 | |
| Task 5 on Resource 2 | 80 | 287 / 0.004 | |
| Task 6 on Resource 2 | 20 | 268 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 2753 / 43.02 | 2598 |
| timer | 648 | 3552 / 55.50 | 3010 |
| intake | 111 | 2083 / 32.55 | 1936 |
| lock | 397 | 447 / 6.98 | 445 |
| unlock | 397 | 1765 / 27.58 | 1589 |
| job_end | 558 | 1961 / 30.64 | 1719 |
| now | 69927 | 234 / 3.66 | 231 |
| dma | 20 | 705 / 11.02 | 705 |
| epilogue | 558 | 1211 / 18.92 | 672 |
| view | 750 | 327 / 5.11 | 297 |

Entries that interrupted an executing Job: Task 0: 112, Task 2: 271, Task 3: 151, Task 4: 42, Task 5: 25, Task 6: 4

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3644 cycles
- Task 0 at 1930.000 ms, 3660 cycles
- Task 0 at 1940.000 ms, 3143 cycles
- Task 2 at 1945.000 ms, 3143 cycles
- Task 0 at 1950.000 ms, 3808 cycles
- Task 5 at 1950.000 ms, 3808 cycles
- Task 0 at 1960.000 ms, 3143 cycles
- Task 2 at 1965.000 ms, 3155 cycles
- Task 0 at 1970.000 ms, 3272 cycles
- Task 5 at 1975.000 ms, 3099 cycles
- Task 0 at 1980.000 ms, 3107 cycles
- Task 0 at 1990.000 ms, 3039 cycles


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
     Finished in 6.99s
```
