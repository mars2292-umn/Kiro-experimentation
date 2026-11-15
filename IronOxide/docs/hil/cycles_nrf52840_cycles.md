# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **FAIL**

Target 1366:1060:001050217737; binary sha256:1ddfab763fd978db5c4b1c5098f452bcc0c8f1c07fcdfc8b62a67953cf9fb099 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.9 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | FAIL | control mean 75984 cycles (1.187 ms), log mean 83671 cycles (1.307 ms): difference 7688 cycles (10.12 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 7688 cycles (0.120 ms) per log Job over its 49 Jobs, interrupted by 146 Kernel entries: 2580 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75290 max 79030, log min 81430 max 89506 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 133530 ticks (534120 cycles) against its largest execution 89506 cycles; control's largest response 25127 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4205 / 65.7 | 3491 | 2^12:191 2^13:8 |
| 2 | 99 | 3685 / 57.6 | 3315 | 2^12:99 |
| 3 | 49 | 3208 / 50.1 | 3152 | 2^12:49 |
| 5 | 79 | 4205 / 65.7 | 3581 | 2^12:71 2^13:8 |
| 6 | 19 | 3760 / 58.8 | 3710 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 1.570 | 0 | 0 | 0 |
| 2 | 99 | 6.688 | 0 | 0 | 0 |
| 3 | 49 | 8.346 | 0 | 0 | 0 |
| 4 | 110 | 6.678 | 111 | 2148 | 2038 |
| 5 | 80 | 7.044 | 0 | 0 | 0 |
| 6 | 20 | 1.941 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 79030 / 1.235 (min 75290, mean 75984) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 339993 / 5.312 (min 335926, mean 336762) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 89506 / 1.399 (min 81430, mean 83671) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 7165 / 0.112 (min 3382, mean 4849) | 0 above the Budget; 2^12:66 2^13:44 |
| Task 5 entry | 80 | 11513 / 0.180 (min 6640, mean 8701) | 0 above the Budget; 2^13:36 2^14:44 |
| Task 6 entry | 20 | 11314 / 0.177 (min 7826, mean 8181) | 0 above the Budget; 2^13:18 2^14:2 |
| Task 2 on Resource 0 | 99 | 325009 / 5.078 | |
| Task 2 on Resource 1 | 198 | 3286 / 0.051 | |
| Task 5 on Resource 2 | 80 | 506 / 0.008 | |
| Task 6 on Resource 2 | 20 | 488 / 0.008 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 3060 / 47.81 | 2894 |
| timer | 580 | 3515 / 54.92 | 2946 |
| intake | 111 | 2260 / 35.31 | 2151 |
| lock | 397 | 453 / 7.08 | 451 |
| unlock | 397 | 1833 / 28.64 | 1623 |
| job_end | 558 | 2003 / 31.30 | 1742 |
| now | 40836 | 229 / 3.58 | 226 |
| dma | 20 | 707 / 11.05 | 706 |
| epilogue | 558 | 1248 / 19.50 | 705 |
| view | 750 | 329 / 5.14 | 300 |

Entries that interrupted an executing Job: Task 0: 36, Task 2: 268, Task 3: 146, Task 4: 44, Task 5: 46, Task 6: 2

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3605 cycles
- Task 0 at 1930.000 ms, 3893 cycles
- Task 0 at 1940.000 ms, 3208 cycles
- Task 2 at 1945.000 ms, 3156 cycles
- Task 0 at 1950.000 ms, 3705 cycles
- Task 5 at 1950.000 ms, 3705 cycles
- Task 0 at 1960.000 ms, 3152 cycles
- Task 2 at 1965.000 ms, 3116 cycles
- Task 0 at 1970.000 ms, 3357 cycles
- Task 5 at 1975.000 ms, 3164 cycles
- Task 0 at 1980.000 ms, 3112 cycles
- Task 0 at 1990.000 ms, 3076 cycles


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
     Finished in 8.05s
```
