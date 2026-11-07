# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:7fc49c7c256605cb3904352f332eb434db8357ee0d216d576d2f243f1ae2ff28 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.5 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77391 cycles (1.209 ms), log mean 83372 cycles (1.303 ms): difference 5981 cycles (7.73 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5981 cycles (0.093 ms) per log Job over its 49 Jobs, interrupted by 151 Kernel entries: 1941 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75056 max 82040, log min 81116 max 88260 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 149409 ticks (597636 cycles) against its largest execution 88260 cycles; control's largest response 41269 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4346 / 67.9 | 3454 | 2^12:191 2^13:8 |
| 2 | 99 | 3678 / 57.5 | 3296 | 2^12:99 |
| 3 | 49 | 3174 / 49.6 | 3134 | 2^12:49 |
| 5 | 79 | 4346 / 67.9 | 3542 | 2^12:71 2^13:8 |
| 6 | 19 | 3722 / 58.2 | 3681 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.579 | 0 | 0 | 0 |
| 2 | 99 | 7.694 | 0 | 0 | 0 |
| 3 | 49 | 9.338 | 0 | 0 | 0 |
| 4 | 110 | 6.688 | 111 | 2124 | 2004 |
| 5 | 80 | 8.040 | 0 | 0 | 0 |
| 6 | 20 | 1.919 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82040 / 1.282 (min 75056, mean 77391) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 403767 / 6.309 (min 399199, mean 400088) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 88260 / 1.379 (min 81116, mean 83372) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 7045 / 0.110 (min 3201, mean 4986) | 0 above the Budget; 2^12:56 2^13:54 |
| Task 5 entry | 80 | 10211 / 0.160 (min 6287, mean 7497) | 0 above the Budget; 2^13:56 2^14:24 |
| Task 6 entry | 20 | 7486 / 0.117 (min 7454, mean 7462) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 388905 / 6.077 | |
| Task 2 on Resource 1 | 198 | 3966 / 0.062 | |
| Task 5 on Resource 2 | 80 | 3937 / 0.062 | |
| Task 6 on Resource 2 | 20 | 401 / 0.006 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 2832 / 44.25 | 2700 |
| timer | 664 | 3484 / 54.44 | 2965 |
| intake | 111 | 2243 / 35.05 | 2122 |
| lock | 397 | 480 / 7.50 | 479 |
| unlock | 397 | 1805 / 28.20 | 1617 |
| job_end | 558 | 2024 / 31.62 | 1767 |
| now | 55080 | 236 / 3.69 | 233 |
| dma | 20 | 732 / 11.44 | 732 |
| epilogue | 558 | 1220 / 19.06 | 678 |
| view | 753 | 330 / 5.16 | 301 |

Entries that interrupted an executing Job: Task 0: 123, Task 2: 286, Task 3: 151, Task 4: 54, Task 5: 24

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3610 cycles
- Task 0 at 1930.000 ms, 3526 cycles
- Task 0 at 1940.000 ms, 3126 cycles
- Task 2 at 1945.000 ms, 3130 cycles
- Task 0 at 1950.000 ms, 3638 cycles
- Task 5 at 1950.000 ms, 3638 cycles
- Task 0 at 1960.000 ms, 3126 cycles
- Task 2 at 1965.000 ms, 3094 cycles
- Task 0 at 1970.000 ms, 3270 cycles
- Task 5 at 1975.000 ms, 3078 cycles
- Task 0 at 1980.000 ms, 3090 cycles
- Task 0 at 1990.000 ms, 3018 cycles


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
     Finished in 7.71s
```
