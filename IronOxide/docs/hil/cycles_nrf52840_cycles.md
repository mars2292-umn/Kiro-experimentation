# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:19963bb54f236883ce9eec3b93e94b04724d5443f1476d122d7f6c6e4b84dbd3 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.6 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77292 cycles (1.208 ms), log mean 83406 cycles (1.303 ms): difference 6113 cycles (7.91 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 6113 cycles (0.096 ms) per log Job over its 49 Jobs, interrupted by 151 Kernel entries: 1984 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75090 max 82994, log min 81160 max 88322 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 149104 ticks (596416 cycles) against its largest execution 88322 cycles; control's largest response 41246 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4204 / 65.7 | 3512 | 2^12:190 2^13:9 |
| 2 | 99 | 3692 / 57.7 | 3311 | 2^12:99 |
| 3 | 49 | 3203 / 50.0 | 3158 | 2^12:49 |
| 5 | 79 | 4204 / 65.7 | 3578 | 2^12:70 2^13:9 |
| 6 | 19 | 3727 / 58.2 | 3686 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.578 | 0 | 0 | 0 |
| 2 | 99 | 7.695 | 0 | 0 | 0 |
| 3 | 49 | 9.319 | 0 | 0 | 0 |
| 4 | 110 | 6.226 | 111 | 2126 | 2031 |
| 5 | 80 | 7.984 | 0 | 0 | 0 |
| 6 | 20 | 1.926 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82994 / 1.297 (min 75090, mean 77292) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 403590 / 6.306 (min 399318, mean 400260) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 88322 / 1.380 (min 81160, mean 83406) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 7064 / 0.110 (min 3236, mean 4692) | 0 above the Budget; 2^12:66 2^13:44 |
| Task 5 entry | 80 | 10282 / 0.161 (min 6346, mean 7469) | 0 above the Budget; 2^13:58 2^14:22 |
| Task 6 entry | 20 | 11063 / 0.173 (min 7515, mean 7875) | 0 above the Budget; 2^13:18 2^14:2 |
| Task 2 on Resource 0 | 99 | 388790 / 6.075 | |
| Task 2 on Resource 1 | 198 | 402 / 0.006 | |
| Task 5 on Resource 2 | 80 | 429 / 0.007 | |
| Task 6 on Resource 2 | 20 | 3894 / 0.061 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 2870 / 44.84 | 2738 |
| timer | 655 | 3487 / 54.48 | 2980 |
| intake | 111 | 2251 / 35.17 | 2157 |
| lock | 397 | 465 / 7.27 | 463 |
| unlock | 397 | 1844 / 28.81 | 1641 |
| job_end | 558 | 2033 / 31.77 | 1770 |
| now | 54151 | 240 / 3.75 | 240 |
| dma | 20 | 734 / 11.47 | 734 |
| epilogue | 558 | 1225 / 19.14 | 686 |
| view | 753 | 330 / 5.16 | 301 |

Entries that interrupted an executing Job: Task 0: 112, Task 2: 286, Task 3: 151, Task 4: 44, Task 5: 22, Task 6: 2

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3616 cycles
- Task 0 at 1930.000 ms, 3784 cycles
- Task 0 at 1940.000 ms, 3147 cycles
- Task 2 at 1945.000 ms, 3151 cycles
- Task 0 at 1950.000 ms, 3564 cycles
- Task 5 at 1950.000 ms, 3564 cycles
- Task 0 at 1960.000 ms, 3147 cycles
- Task 2 at 1965.000 ms, 3115 cycles
- Task 0 at 1970.000 ms, 3760 cycles
- Task 5 at 1975.000 ms, 3107 cycles
- Task 0 at 1980.000 ms, 3111 cycles
- Task 0 at 1990.000 ms, 3039 cycles


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
     Finished in 7.69s
```
