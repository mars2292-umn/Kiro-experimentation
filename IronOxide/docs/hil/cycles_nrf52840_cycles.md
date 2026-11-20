# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:5791ba91b0c753f455c6107ec5390719eae75552a4c4c145d4d6f60b08a2fa76 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.9 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 78061 cycles (1.220 ms), log mean 84942 cycles (1.327 ms): difference 6882 cycles (8.82 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 6882 cycles (0.108 ms) per log Job over its 49 Jobs, interrupted by 163 Kernel entries: 2069 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75481 max 86631, log min 81809 max 96579 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 151195 ticks (604780 cycles) against its largest execution 96579 cycles; control's largest response 42424 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 7896 / 123.4 | 3651 | 2^12:174 2^13:25 |
| 2 | 99 | 6272 / 98.0 | 3483 | 2^12:98 2^13:1 |
| 3 | 49 | 3324 / 51.9 | 3265 | 2^12:49 |
| 5 | 79 | 4358 / 68.1 | 3716 | 2^12:66 2^13:13 |
| 6 | 19 | 3880 / 60.6 | 3819 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.651 | 0 | 0 | 0 |
| 2 | 99 | 7.772 | 0 | 0 | 0 |
| 3 | 49 | 9.450 | 0 | 0 | 0 |
| 4 | 132 | 7.855 | 133 | 2220 | 2076 |
| 5 | 80 | 8.142 | 0 | 0 | 0 |
| 6 | 20 | 1.966 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 86631 / 1.354 (min 75481, mean 78061) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 407537 / 6.368 (min 400055, mean 400867) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 96579 / 1.509 (min 81809, mean 84942) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 132 | 7606 / 0.119 (min 3568, mean 5239) | 0 above the Budget; 2^12:75 2^13:57 |
| Task 5 entry | 80 | 15497 / 0.242 (min 6800, mean 7983) | 0 above the Budget; 2^13:59 2^14:21 |
| Task 6 entry | 20 | 10517 / 0.164 (min 6853, mean 7288) | 0 above the Budget; 2^13:17 2^14:3 |
| Task 2 on Resource 0 | 99 | 392041 / 6.126 | |
| Task 2 on Resource 1 | 198 | 7079 / 0.111 | |
| Task 5 on Resource 2 | 80 | 4277 / 0.067 | |
| Task 6 on Resource 2 | 20 | 470 / 0.007 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 580 | 3270 / 51.09 | 3068 |
| timer | 694 | 3623 / 56.61 | 3069 |
| intake | 133 | 2331 / 36.42 | 2188 |
| lock | 397 | 450 / 7.03 | 448 |
| unlock | 397 | 1839 / 28.73 | 1663 |
| job_end | 580 | 2065 / 32.27 | 1772 |
| now | 49986 | 228 / 3.56 | 225 |
| dma | 1 | 736 / 11.50 | 736 |
| epilogue | 580 | 1253 / 19.58 | 706 |
| view | 780 | 329 / 5.14 | 301 |

Entries that interrupted an executing Job: Task 0: 130, Task 2: 290, Task 3: 163, Task 4: 57, Task 5: 23, Task 6: 2

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 8632 cycles; Task 2: 4 (dispatch, timer, intake), 11787 cycles; Task 3: 7 (dispatch, timer, intake), 21103 cycles; Task 4: 1 (timer), 3074 cycles; Task 5: 3 (dispatch, timer, intake), 8417 cycles; Task 6: 1 (timer), 2954 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3798 cycles
- Task 0 at 1930.000 ms, 4098 cycles
- Task 0 at 1940.000 ms, 3264 cycles
- Task 2 at 1945.000 ms, 3260 cycles
- Task 0 at 1950.000 ms, 4294 cycles
- Task 5 at 1950.000 ms, 4294 cycles
- Task 0 at 1960.000 ms, 3264 cycles
- Task 2 at 1965.000 ms, 3224 cycles
- Task 0 at 1970.000 ms, 3366 cycles
- Task 5 at 1975.000 ms, 3232 cycles
- Task 0 at 1980.000 ms, 3232 cycles
- Task 0 at 1990.000 ms, 3192 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=132
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
     Finished in 8.07s
```
