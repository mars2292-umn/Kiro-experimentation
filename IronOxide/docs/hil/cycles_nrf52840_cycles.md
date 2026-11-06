# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:29ed1d0cc41b81b189200d08c01f78901f4895a23ad4e1f32923a920c0eb4793 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 11.2 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77097 cycles (1.205 ms), log mean 82474 cycles (1.289 ms): difference 5377 cycles (6.97 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5377 cycles (0.084 ms) per log Job over its 49 Jobs, interrupted by 151 Kernel entries: 1745 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74922 max 82312, log min 80486 max 86836 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 147215 ticks (588860 cycles) against its largest execution 86836 cycles; control's largest response 41361 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4143 / 64.7 | 3351 | 2^12:196 2^13:3 |
| 2 | 99 | 3555 / 55.5 | 3163 | 2^12:99 |
| 3 | 49 | 3046 / 47.6 | 3011 | 2^12:49 |
| 5 | 79 | 4143 / 64.7 | 3441 | 2^12:76 2^13:3 |
| 6 | 19 | 3606 / 56.3 | 3571 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.585 | 0 | 0 | 0 |
| 2 | 99 | 7.688 | 0 | 0 | 0 |
| 3 | 49 | 9.201 | 0 | 0 | 0 |
| 4 | 111 | 6.348 | 111 | 1974 | 1846 |
| 5 | 80 | 7.915 | 0 | 0 | 0 |
| 6 | 20 | 1.873 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82312 / 1.286 (min 74922, mean 77097) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 401695 / 6.276 (min 397617, mean 398342) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 86836 / 1.357 (min 80486, mean 82474) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 111 | 6691 / 0.105 (min 3017, mean 4107) | 0 above the Budget; 2^12:77 2^13:34 |
| Task 5 entry | 80 | 12837 / 0.201 (min 5785, mean 6817) | 0 above the Budget; 2^13:61 2^14:19 |
| Task 6 entry | 20 | 6817 / 0.107 (min 6797, mean 6802) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 385332 / 6.021 | |
| Task 2 on Resource 1 | 198 | 267 / 0.004 | |
| Task 5 on Resource 2 | 80 | 3705 / 0.058 | |
| Task 6 on Resource 2 | 20 | 276 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 559 | 2772 / 43.31 | 2621 |
| timer | 650 | 3425 / 53.52 | 2876 |
| intake | 111 | 2043 / 31.92 | 1914 |
| lock | 397 | 447 / 6.98 | 446 |
| unlock | 397 | 1723 / 26.92 | 1546 |
| job_end | 559 | 1921 / 30.02 | 1675 |
| now | 67883 | 238 / 3.72 | 235 |
| dma | 20 | 707 / 11.05 | 707 |
| epilogue | 559 | 1209 / 18.89 | 673 |
| view | 754 | 327 / 5.11 | 297 |

Entries that interrupted an executing Job: Task 0: 123, Task 2: 285, Task 3: 151, Task 4: 34, Task 5: 21

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3483 cycles
- Task 0 at 1930.000 ms, 3559 cycles
- Task 0 at 1940.000 ms, 3006 cycles
- Task 2 at 1945.000 ms, 2998 cycles
- Task 0 at 1950.000 ms, 3655 cycles
- Task 5 at 1950.000 ms, 3655 cycles
- Task 0 at 1960.000 ms, 3006 cycles
- Task 2 at 1965.000 ms, 3002 cycles
- Task 0 at 1970.000 ms, 3307 cycles
- Task 5 at 1975.000 ms, 2966 cycles
- Task 0 at 1980.000 ms, 2970 cycles
- Task 0 at 1990.000 ms, 2926 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=111
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
     Finished in 6.70s
```
