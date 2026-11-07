# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:a65b47fc1e6c14fb30b4c74b701389864475a8b18334226e9e3035042babd9ae (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.1 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77427 cycles (1.210 ms), log mean 82509 cycles (1.289 ms): difference 5082 cycles (6.56 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5082 cycles (0.079 ms) per log Job over its 49 Jobs, interrupted by 145 Kernel entries: 1717 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74977 max 82395, log min 80610 max 87896 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148380 ticks (593520 cycles) against its largest execution 87896 cycles; control's largest response 40843 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4136 / 64.6 | 3323 | 2^12:198 2^13:1 |
| 2 | 99 | 3672 / 57.4 | 3228 | 2^12:99 |
| 3 | 49 | 3120 / 48.8 | 3058 | 2^12:49 |
| 5 | 79 | 4136 / 64.6 | 3438 | 2^12:78 2^13:1 |
| 6 | 19 | 3644 / 56.9 | 3587 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.553 | 0 | 0 | 0 |
| 2 | 99 | 7.663 | 0 | 0 | 0 |
| 3 | 49 | 9.274 | 0 | 0 | 0 |
| 4 | 110 | 7.495 | 111 | 2095 | 1956 |
| 5 | 80 | 7.979 | 0 | 0 | 0 |
| 6 | 20 | 1.892 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82395 / 1.287 (min 74977, mean 77427) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 402304 / 6.286 (min 398334, mean 398655) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 87896 / 1.373 (min 80610, mean 82509) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 6774 / 0.106 (min 3064, mean 4455) | 0 above the Budget; 2^12:67 2^13:43 |
| Task 5 entry | 80 | 9674 / 0.151 (min 5962, mean 6921) | 0 above the Budget; 2^13:61 2^14:19 |
| Task 6 entry | 20 | 7022 / 0.110 (min 6982, mean 6988) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 385360 / 6.021 | |
| Task 2 on Resource 1 | 198 | 275 / 0.004 | |
| Task 5 on Resource 2 | 80 | 303 / 0.005 | |
| Task 6 on Resource 2 | 20 | 284 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 2848 / 44.50 | 2699 |
| timer | 654 | 3467 / 54.17 | 2925 |
| intake | 111 | 2223 / 34.73 | 2082 |
| lock | 397 | 466 / 7.28 | 464 |
| unlock | 397 | 1860 / 29.06 | 1651 |
| job_end | 558 | 2048 / 32.00 | 1775 |
| now | 66479 | 243 / 3.80 | 242 |
| dma | 20 | 709 / 11.08 | 709 |
| epilogue | 558 | 1229 / 19.20 | 672 |
| view | 748 | 327 / 5.11 | 297 |

Entries that interrupted an executing Job: Task 0: 134, Task 2: 270, Task 3: 145, Task 4: 43, Task 5: 19

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3592 cycles
- Task 0 at 1930.000 ms, 3564 cycles
- Task 0 at 1940.000 ms, 3052 cycles
- Task 2 at 1945.000 ms, 3056 cycles
- Task 0 at 1950.000 ms, 3608 cycles
- Task 5 at 1950.000 ms, 3608 cycles
- Task 0 at 1960.000 ms, 3116 cycles
- Task 2 at 1965.000 ms, 3016 cycles
- Task 0 at 1970.000 ms, 3216 cycles
- Task 5 at 1975.000 ms, 3012 cycles
- Task 0 at 1980.000 ms, 3016 cycles
- Task 0 at 1990.000 ms, 2976 cycles


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
     Finished in 7.36s
```
