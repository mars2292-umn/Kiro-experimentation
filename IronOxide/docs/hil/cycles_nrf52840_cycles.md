# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:579dc8be570e6e5d15458b4d90ed4d551d24161b5425d95db77b82ab3501f615 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.9 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77289 cycles (1.208 ms), log mean 82396 cycles (1.287 ms): difference 5107 cycles (6.61 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5107 cycles (0.080 ms) per log Job over its 49 Jobs, interrupted by 118 Kernel entries: 2121 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75276 max 79328, log min 81479 max 84087 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 147773 ticks (591092 cycles) against its largest execution 84087 cycles; control's largest response 41363 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4120 / 64.4 | 3549 | 2^12:189 2^13:10 |
| 2 | 99 | 3772 / 58.9 | 3372 | 2^12:99 |
| 3 | 49 | 3223 / 50.4 | 3223 | 2^12:49 |
| 5 | 79 | 4120 / 64.4 | 3651 | 2^12:69 2^13:10 |
| 6 | 19 | 3779 / 59.0 | 3779 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.585 | 0 | 0 | 0 |
| 2 | 99 | 7.703 | 0 | 0 | 0 |
| 3 | 49 | 9.236 | 0 | 0 | 0 |
| 5 | 80 | 7.933 | 0 | 0 | 0 |
| 6 | 20 | 1.937 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 79328 / 1.240 (min 75276, mean 77289) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 403939 / 6.312 (min 399999, mean 400372) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 84087 / 1.314 (min 81479, mean 82396) | 0 above the Budget; 2^15:49 |
| Task 5 entry | 80 | 10432 / 0.163 (min 6584, mean 7706) | 0 above the Budget; 2^13:58 2^14:22 |
| Task 6 entry | 20 | 7810 / 0.122 (min 7810, mean 7810) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 388958 / 6.077 | |
| Task 2 on Resource 1 | 198 | 474 / 0.007 | |
| Task 5 on Resource 2 | 80 | 501 / 0.008 | |
| Task 6 on Resource 2 | 20 | 483 / 0.008 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 448 | 3039 / 47.48 | 2896 |
| timer | 505 | 3540 / 55.31 | 3070 |
| lock | 397 | 457 / 7.14 | 456 |
| unlock | 397 | 1773 / 27.70 | 1619 |
| job_end | 448 | 1952 / 30.50 | 1735 |
| now | 50402 | 220 / 3.44 | 220 |
| dma | 20 | 711 / 11.11 | 711 |
| epilogue | 448 | 1246 / 19.47 | 727 |
| view | 616 | 329 / 5.14 | 293 |

Entries that interrupted an executing Job: Task 0: 101, Task 2: 204, Task 3: 118, Task 5: 22

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3696 cycles
- Task 0 at 1930.000 ms, 3616 cycles
- Task 0 at 1940.000 ms, 3215 cycles
- Task 2 at 1945.000 ms, 3219 cycles
- Task 0 at 1950.000 ms, 4120 cycles
- Task 5 at 1950.000 ms, 4120 cycles
- Task 0 at 1960.000 ms, 3215 cycles
- Task 2 at 1965.000 ms, 3187 cycles
- Task 0 at 1970.000 ms, 3720 cycles
- Task 5 at 1975.000 ms, 3183 cycles
- Task 0 at 1980.000 ms, 3183 cycles
- Task 0 at 1990.000 ms, 3143 cycles


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
     Finished in 7.94s
```
