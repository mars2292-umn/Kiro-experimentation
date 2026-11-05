# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:ae1d348c3aadd0d6cc92025b386e81f45ba189d90a9d40a76811984afaccbbb5 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 11.8 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77262 cycles (1.207 ms), log mean 82989 cycles (1.297 ms): difference 5727 cycles (7.41 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5727 cycles (0.089 ms) per log Job over its 49 Jobs, interrupted by 159 Kernel entries: 1765 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74901 max 85123, log min 80586 max 90480 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148886 ticks (595544 cycles) against its largest execution 90480 cycles; control's largest response 41423 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 5964 / 93.2 | 3471 | 2^12:195 2^13:4 |
| 2 | 99 | 8031 / 125.5 | 3362 | 2^12:98 2^13:1 |
| 3 | 49 | 3183 / 49.7 | 3140 | 2^12:49 |
| 5 | 79 | 5140 / 80.3 | 3576 | 2^12:76 2^13:3 |
| 6 | 19 | 4123 / 64.4 | 3714 | 2^12:18 2^13:1 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.589 | 0 | 0 | 0 |
| 2 | 99 | 7.694 | 0 | 0 | 0 |
| 3 | 49 | 9.305 | 0 | 0 | 0 |
| 4 | 132 | 7.754 | 133 | 2019 | 1872 |
| 5 | 80 | 8.018 | 0 | 0 | 0 |
| 6 | 20 | 1.879 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 85123 / 1.330 (min 74901, mean 77262) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 404301 / 6.317 (min 397669, mean 398261) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 90480 / 1.414 (min 80586, mean 82989) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 132 | 6798 / 0.106 (min 2992, mean 4519) | 0 above the Budget; 2^12:77 2^13:55 |
| Task 5 entry | 80 | 9566 / 0.149 (min 5786, mean 6756) | 0 above the Budget; 2^13:61 2^14:19 |
| Task 6 entry | 20 | 10233 / 0.160 (min 6777, mean 6951) | 0 above the Budget; 2^13:19 2^14:1 |
| Task 2 on Resource 0 | 99 | 391089 / 6.111 | |
| Task 2 on Resource 1 | 198 | 259 / 0.004 | |
| Task 5 on Resource 2 | 80 | 287 / 0.004 | |
| Task 6 on Resource 2 | 20 | 268 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 580 | 2753 / 43.02 | 2603 |
| timer | 686 | 3576 / 55.88 | 2998 |
| intake | 133 | 2091 / 32.67 | 1943 |
| lock | 397 | 447 / 6.98 | 445 |
| unlock | 397 | 1717 / 26.83 | 1580 |
| job_end | 580 | 1960 / 30.62 | 1716 |
| now | 69822 | 234 / 3.66 | 231 |
| dma | 20 | 705 / 11.02 | 705 |
| epilogue | 580 | 1211 / 18.92 | 672 |
| view | 779 | 327 / 5.11 | 298 |

Entries that interrupted an executing Job: Task 0: 128, Task 2: 292, Task 3: 159, Task 4: 55, Task 5: 19, Task 6: 1

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3644 cycles
- Task 0 at 1930.000 ms, 3800 cycles
- Task 0 at 1940.000 ms, 3143 cycles
- Task 2 at 1945.000 ms, 3143 cycles
- Task 0 at 1950.000 ms, 3804 cycles
- Task 5 at 1950.000 ms, 3804 cycles
- Task 0 at 1960.000 ms, 3143 cycles
- Task 2 at 1965.000 ms, 3103 cycles
- Task 0 at 1970.000 ms, 3364 cycles
- Task 5 at 1975.000 ms, 3099 cycles
- Task 0 at 1980.000 ms, 3107 cycles
- Task 0 at 1990.000 ms, 3071 cycles


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
     Finished in 7.02s
```
