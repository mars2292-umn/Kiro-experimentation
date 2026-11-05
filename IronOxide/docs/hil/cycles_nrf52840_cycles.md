# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:993f6b71480dc049ecbec1a8e86a73a2c4675413ffaef9864fac80368ea0d4f3 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 11.8 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77048 cycles (1.204 ms), log mean 82400 cycles (1.288 ms): difference 5352 cycles (6.95 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5352 cycles (0.084 ms) per log Job over its 49 Jobs, interrupted by 145 Kernel entries: 1809 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74877 max 82241, log min 80577 max 87827 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148267 ticks (593068 cycles) against its largest execution 87827 cycles; control's largest response 40807 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4219 / 65.9 | 3592 | 2^12:186 2^13:13 |
| 2 | 99 | 3815 / 59.6 | 3408 | 2^12:99 |
| 3 | 49 | 3268 / 51.1 | 3227 | 2^12:49 |
| 5 | 79 | 4219 / 65.9 | 3673 | 2^12:71 2^13:8 |
| 6 | 19 | 3824 / 59.8 | 3777 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.550 | 0 | 0 | 0 |
| 2 | 99 | 7.659 | 0 | 0 | 0 |
| 3 | 49 | 9.267 | 0 | 0 | 0 |
| 4 | 110 | 7.241 | 111 | 2010 | 1857 |
| 5 | 80 | 7.978 | 0 | 0 | 0 |
| 6 | 20 | 1.880 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82241 / 1.285 (min 74877, mean 77048) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 401957 / 6.281 (min 397450, mean 398250) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 87827 / 1.372 (min 80577, mean 82400) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 6875 / 0.107 (min 2944, mean 4452) | 0 above the Budget; 2^12:66 2^13:44 |
| Task 5 entry | 80 | 9719 / 0.152 (min 5768, mean 7020) | 0 above the Budget; 2^13:55 2^14:25 |
| Task 6 entry | 20 | 10818 / 0.169 (min 6729, mean 7144) | 0 above the Budget; 2^13:18 2^14:2 |
| Task 2 on Resource 0 | 99 | 388590 / 6.072 | |
| Task 2 on Resource 1 | 198 | 268 / 0.004 | |
| Task 5 on Resource 2 | 80 | 296 / 0.005 | |
| Task 6 on Resource 2 | 20 | 277 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 2711 / 42.36 | 2554 |
| timer | 651 | 3690 / 57.66 | 3107 |
| intake | 111 | 2078 / 32.47 | 1924 |
| lock | 397 | 452 / 7.06 | 450 |
| unlock | 397 | 1757 / 27.45 | 1579 |
| job_end | 558 | 1961 / 30.64 | 1716 |
| now | 69384 | 226 / 3.53 | 225 |
| dma | 20 | 695 / 10.86 | 693 |
| epilogue | 558 | 1162 / 18.16 | 656 |

Entries that interrupted an executing Job: Task 0: 114, Task 2: 275, Task 3: 145, Task 4: 44, Task 5: 25, Task 6: 4

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3731 cycles
- Task 0 at 1930.000 ms, 4099 cycles
- Task 0 at 1940.000 ms, 3224 cycles
- Task 2 at 1945.000 ms, 3224 cycles
- Task 0 at 1950.000 ms, 4047 cycles
- Task 5 at 1950.000 ms, 4047 cycles
- Task 0 at 1960.000 ms, 3224 cycles
- Task 2 at 1965.000 ms, 3188 cycles
- Task 0 at 1970.000 ms, 3655 cycles
- Task 5 at 1975.000 ms, 3228 cycles
- Task 0 at 1980.000 ms, 3188 cycles
- Task 0 at 1990.000 ms, 3148 cycles


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
     Finished in 6.99s
```
