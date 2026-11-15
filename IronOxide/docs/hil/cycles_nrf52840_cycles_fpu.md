# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:c9966efb6aeb79cc4e8024d3024a395c9044b6e7eefe7186fa28b2950b022aa3 (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.8 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77589 cycles (1.212 ms), filter mean 82936 cycles (1.296 ms): difference 5348 cycles (6.89 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5348 cycles (0.084 ms) per log Job over its 49 Jobs, interrupted by 129 Kernel entries: 2031 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75238 max 83033, log min 81578 max 86591 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 146245 ticks (584980 cycles) against its largest execution 86591 cycles; control's largest response 41370 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4049 / 63.3 | 3400 | 2^12:199 |
| 2 | 99 | 3497 / 54.6 | 3295 | 2^12:99 |
| 3 | 49 | 3476 / 54.3 | 3426 | 2^12:49 |
| 5 | 49 | 3476 / 54.3 | 3426 | 2^12:49 |
| 6 | 19 | 3492 / 54.6 | 3445 | 2^12:19 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.586 | 0 | 0 | 0 |
| 2 | 99 | 7.704 | 0 | 0 | 0 |
| 3 | 49 | 10.369 | 0 | 0 | 0 |
| 4 | 109 | 6.370 | 110 | 2188 | 2049 |
| 5 | 49 | 9.140 | 0 | 0 | 0 |
| 6 | 20 | 1.774 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 83033 / 1.297 (min 75238, mean 77589) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 404233 / 6.316 (min 399886, mean 400690) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 78940 / 1.233 (min 75380, mean 75838) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 109 | 7157 / 0.112 (min 3410, mean 4699) | 0 above the Budget; 2^12:70 2^13:39 |
| Task 5 entry | 49 | 86591 / 1.353 (min 81578, mean 82936) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 14802 / 0.231 (min 7826, mean 11330) | 0 above the Budget; 2^13:2 2^14:18 |
| Task 2 on Resource 0 | 99 | 385949 / 6.030 | |
| Task 2 on Resource 1 | 198 | 479 / 0.007 | |
| Task 6 on Resource 2 | 20 | 488 / 0.008 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 526 | 3060 / 47.81 | 2893 |
| timer | 622 | 3247 / 50.73 | 2954 |
| intake | 110 | 2300 / 35.94 | 2163 |
| lock | 317 | 453 / 7.08 | 451 |
| unlock | 317 | 1836 / 28.69 | 1643 |
| job_end | 526 | 1944 / 30.38 | 1748 |
| now | 49103 | 229 / 3.58 | 226 |
| dma | 20 | 709 / 11.08 | 707 |
| epilogue | 526 | 1223 / 19.11 | 690 |
| view | 700 | 329 / 5.14 | 296 |

Entries that interrupted an executing Job: Task 0: 123, Task 2: 273, Task 3: 6, Task 4: 39, Task 5: 129, Task 6: 20

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3412 cycles
- Task 5 at 1924.000 ms, 3412 cycles
- Task 2 at 1925.000 ms, 3425 cycles
- Task 0 at 1930.000 ms, 4049 cycles
- Task 0 at 1940.000 ms, 3152 cycles
- Task 2 at 1945.000 ms, 3156 cycles
- Task 0 at 1950.000 ms, 3917 cycles
- Task 0 at 1960.000 ms, 3152 cycles
- Task 2 at 1965.000 ms, 3080 cycles
- Task 0 at 1970.000 ms, 3349 cycles
- Task 0 at 1980.000 ms, 3076 cycles
- Task 0 at 1990.000 ms, 3004 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles-fpu/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=109
echo=0
blocked_releases=0
filter=49
filter_state_bits=1120384580
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
     Finished in 8.06s
```
