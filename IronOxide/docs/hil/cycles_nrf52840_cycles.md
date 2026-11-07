# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:40f109f47679f7cbfafd89f60eca60e1004e454f1809af1bf09eae96d8cc6085 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.1 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77391 cycles (1.209 ms), log mean 82398 cycles (1.287 ms): difference 5007 cycles (6.47 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5007 cycles (0.078 ms) per log Job over its 49 Jobs, interrupted by 144 Kernel entries: 1704 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74942 max 82377, log min 80557 max 87790 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148224 ticks (592896 cycles) against its largest execution 87790 cycles; control's largest response 40809 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4159 / 65.0 | 3298 | 2^12:197 2^13:2 |
| 2 | 99 | 3599 / 56.2 | 3193 | 2^12:99 |
| 3 | 49 | 3060 / 47.8 | 3011 | 2^12:49 |
| 5 | 79 | 4159 / 65.0 | 3425 | 2^12:77 2^13:2 |
| 6 | 19 | 3624 / 56.6 | 3574 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.551 | 0 | 0 | 0 |
| 2 | 99 | 7.658 | 0 | 0 | 0 |
| 3 | 49 | 9.264 | 0 | 0 | 0 |
| 4 | 110 | 7.486 | 110 | 2041 | 1885 |
| 5 | 80 | 7.963 | 0 | 0 | 0 |
| 6 | 20 | 1.881 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82377 / 1.287 (min 74942, mean 77391) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 401915 / 6.280 (min 397760, mean 398259) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 87790 / 1.372 (min 80557, mean 82398) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 6760 / 0.106 (min 3037, mean 4395) | 0 above the Budget; 2^12:68 2^13:42 |
| Task 5 entry | 80 | 9526 / 0.149 (min 5834, mean 6794) | 0 above the Budget; 2^13:61 2^14:19 |
| Task 6 entry | 20 | 6875 / 0.107 (min 6847, mean 6851) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 385343 / 6.021 | |
| Task 2 on Resource 1 | 198 | 265 / 0.004 | |
| Task 5 on Resource 2 | 80 | 293 / 0.005 | |
| Task 6 on Resource 2 | 20 | 274 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 2803 / 43.80 | 2646 |
| timer | 653 | 3439 / 53.73 | 2901 |
| intake | 110 | 2166 / 33.84 | 2010 |
| lock | 397 | 464 / 7.25 | 463 |
| unlock | 397 | 1773 / 27.70 | 1580 |
| job_end | 558 | 1962 / 30.66 | 1700 |
| now | 67915 | 242 / 3.78 | 241 |
| dma | 20 | 711 / 11.11 | 711 |
| epilogue | 558 | 1222 / 19.09 | 678 |
| view | 747 | 328 / 5.12 | 297 |

Entries that interrupted an executing Job: Task 0: 134, Task 2: 270, Task 3: 144, Task 4: 42, Task 5: 19

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3527 cycles
- Task 0 at 1930.000 ms, 3643 cycles
- Task 0 at 1940.000 ms, 3004 cycles
- Task 2 at 1945.000 ms, 3060 cycles
- Task 0 at 1950.000 ms, 3959 cycles
- Task 5 at 1950.000 ms, 3959 cycles
- Task 0 at 1960.000 ms, 3004 cycles
- Task 2 at 1965.000 ms, 3024 cycles
- Task 0 at 1970.000 ms, 3523 cycles
- Task 5 at 1975.000 ms, 2964 cycles
- Task 0 at 1980.000 ms, 2964 cycles
- Task 0 at 1990.000 ms, 2924 cycles


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
     Finished in 7.33s
```
