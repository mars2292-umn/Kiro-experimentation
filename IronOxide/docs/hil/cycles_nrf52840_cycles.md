# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:d090ddad3e5d5dcc7d746b87295332cbaecde4c21352f47b3ea31d84ba8ddb55 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.1 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77441 cycles (1.210 ms), log mean 82728 cycles (1.293 ms): difference 5287 cycles (6.83 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5287 cycles (0.083 ms) per log Job over its 49 Jobs, interrupted by 153 Kernel entries: 1693 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74979 max 82418, log min 80574 max 87040 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148340 ticks (593360 cycles) against its largest execution 87040 cycles; control's largest response 40778 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4088 / 63.9 | 3361 | 2^12:199 |
| 2 | 99 | 3604 / 56.3 | 3231 | 2^12:99 |
| 3 | 49 | 3127 / 48.9 | 3078 | 2^12:49 |
| 5 | 79 | 4088 / 63.9 | 3471 | 2^12:79 |
| 6 | 19 | 3659 / 57.2 | 3618 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.549 | 0 | 0 | 0 |
| 2 | 99 | 7.656 | 0 | 0 | 0 |
| 3 | 49 | 9.271 | 0 | 0 | 0 |
| 4 | 110 | 6.604 | 111 | 2070 | 1958 |
| 5 | 80 | 7.982 | 0 | 0 | 0 |
| 6 | 20 | 1.886 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82418 / 1.288 (min 74979, mean 77441) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 402182 / 6.284 (min 397839, mean 398390) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 87040 / 1.360 (min 80574, mean 82728) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 6731 / 0.105 (min 3044, mean 4129) | 0 above the Budget; 2^12:77 2^13:33 |
| Task 5 entry | 80 | 9673 / 0.151 (min 5894, mean 6806) | 0 above the Budget; 2^13:62 2^14:18 |
| Task 6 entry | 20 | 6920 / 0.108 (min 6884, mean 6893) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 385321 / 6.021 | |
| Task 2 on Resource 1 | 198 | 269 / 0.004 | |
| Task 5 on Resource 2 | 80 | 297 / 0.005 | |
| Task 6 on Resource 2 | 20 | 278 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 2811 / 43.92 | 2662 |
| timer | 650 | 3448 / 53.88 | 2936 |
| intake | 111 | 2142 / 33.47 | 2029 |
| lock | 397 | 458 / 7.16 | 456 |
| unlock | 397 | 1835 / 28.67 | 1626 |
| job_end | 558 | 2017 / 31.52 | 1752 |
| now | 68876 | 233 / 3.64 | 232 |
| dma | 20 | 690 / 10.78 | 690 |
| epilogue | 558 | 1226 / 19.16 | 685 |
| view | 753 | 327 / 5.11 | 297 |

Entries that interrupted an executing Job: Task 0: 140, Task 2: 277, Task 3: 153, Task 4: 33, Task 5: 18

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3532 cycles
- Task 0 at 1930.000 ms, 3384 cycles
- Task 0 at 1940.000 ms, 3071 cycles
- Task 2 at 1945.000 ms, 3071 cycles
- Task 0 at 1950.000 ms, 3436 cycles
- Task 5 at 1950.000 ms, 3436 cycles
- Task 0 at 1960.000 ms, 3071 cycles
- Task 2 at 1965.000 ms, 3031 cycles
- Task 0 at 1970.000 ms, 3680 cycles
- Task 5 at 1975.000 ms, 3027 cycles
- Task 0 at 1980.000 ms, 3035 cycles
- Task 0 at 1990.000 ms, 3063 cycles


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
     Finished in 7.35s
```
