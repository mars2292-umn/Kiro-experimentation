# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:d1627d1a2a1b09557e1d62a80132957a30f2bfd9c21a5515357c82cd558f059d (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.2 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77341 cycles (1.208 ms), log mean 82432 cycles (1.288 ms): difference 5091 cycles (6.58 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5091 cycles (0.080 ms) per log Job over its 49 Jobs, interrupted by 146 Kernel entries: 1709 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74946 max 82291, log min 80496 max 87686 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148779 ticks (595116 cycles) against its largest execution 87686 cycles; control's largest response 41380 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4109 / 64.2 | 3326 | 2^12:197 2^13:2 |
| 2 | 99 | 3545 / 55.4 | 3159 | 2^12:99 |
| 3 | 49 | 3039 / 47.5 | 2996 | 2^12:49 |
| 5 | 79 | 4109 / 64.2 | 3423 | 2^12:77 2^13:2 |
| 6 | 19 | 3583 / 56.0 | 3547 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.586 | 0 | 0 | 0 |
| 2 | 99 | 7.691 | 0 | 0 | 0 |
| 3 | 49 | 9.299 | 0 | 0 | 0 |
| 4 | 110 | 7.391 | 111 | 1997 | 1839 |
| 5 | 80 | 8.013 | 0 | 0 | 0 |
| 6 | 20 | 1.874 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82291 / 1.286 (min 74946, mean 77341) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 401948 / 6.280 (min 397651, mean 398131) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 87686 / 1.370 (min 80496, mean 82432) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 6714 / 0.105 (min 3037, mean 4448) | 0 above the Budget; 2^12:66 2^13:44 |
| Task 5 entry | 80 | 13690 / 0.214 (min 5841, mean 6954) | 0 above the Budget; 2^13:59 2^14:21 |
| Task 6 entry | 20 | 6845 / 0.107 (min 6825, mean 6828) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 385368 / 6.021 | |
| Task 2 on Resource 1 | 198 | 272 / 0.004 | |
| Task 5 on Resource 2 | 80 | 300 / 0.005 | |
| Task 6 on Resource 2 | 20 | 281 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 2789 / 43.58 | 2637 |
| timer | 657 | 3425 / 53.52 | 2879 |
| intake | 111 | 2067 / 32.30 | 1908 |
| lock | 397 | 451 / 7.05 | 450 |
| unlock | 397 | 1729 / 27.02 | 1557 |
| job_end | 558 | 1913 / 29.89 | 1672 |
| now | 67435 | 242 / 3.78 | 242 |
| dma | 20 | 707 / 11.05 | 707 |
| epilogue | 558 | 1219 / 19.05 | 677 |
| view | 750 | 327 / 5.11 | 297 |

Entries that interrupted an executing Job: Task 0: 133, Task 2: 270, Task 3: 146, Task 4: 44, Task 5: 25

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3473 cycles
- Task 0 at 1930.000 ms, 3373 cycles
- Task 0 at 1940.000 ms, 3035 cycles
- Task 2 at 1945.000 ms, 2987 cycles
- Task 0 at 1950.000 ms, 3405 cycles
- Task 5 at 1950.000 ms, 3405 cycles
- Task 0 at 1960.000 ms, 2995 cycles
- Task 2 at 1965.000 ms, 2947 cycles
- Task 0 at 1970.000 ms, 3557 cycles
- Task 5 at 1975.000 ms, 2995 cycles
- Task 0 at 1980.000 ms, 2959 cycles
- Task 0 at 1990.000 ms, 2915 cycles


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
     Finished in 7.30s
```
