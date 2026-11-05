# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:783d12dd5ea7eb13c0346f6fc53f6d5ebbbbb3153f454319642596c5ef1045b2 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 11.8 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 76697 cycles (1.198 ms), log mean 82195 cycles (1.284 ms): difference 5498 cycles (7.17 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5498 cycles (0.086 ms) per log Job over its 49 Jobs; the Analyzer's switching parameters must cover it per preemption |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74661 max 80971, log min 80137 max 87031 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 147390 ticks (589560 cycles) against its largest execution 87031 cycles; control's largest response 40296 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4092 / 63.9 | 3384 | 2^12:199 |
| 2 | 99 | 3724 / 58.2 | 3281 | 2^12:99 |
| 3 | 49 | 3168 / 49.5 | 3120 | 2^12:49 |
| 5 | 79 | 4092 / 63.9 | 3512 | 2^12:79 |
| 6 | 19 | 3732 / 58.3 | 3689 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.518 | 0 | 0 | 0 |
| 2 | 99 | 7.614 | 0 | 0 | 0 |
| 3 | 49 | 9.212 | 0 | 0 | 0 |
| 4 | 110 | 7.010 | 111 | 1986 | 1852 |
| 5 | 80 | 7.915 | 0 | 0 | 0 |
| 6 | 20 | 1.850 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 80971 / 1.265 (min 74661, mean 76697) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 400748 / 6.262 (min 396610, mean 396972) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 87031 / 1.360 (min 80137, mean 82195) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 6466 / 0.101 (min 2752, mean 4150) | 0 above the Budget; 2^12:67 2^13:43 |
| Task 5 entry | 80 | 9134 / 0.143 (min 5356, mean 6524) | 0 above the Budget; 2^13:56 2^14:24 |
| Task 6 entry | 20 | 9737 / 0.152 (min 6241, mean 6596) | 0 above the Budget; 2^13:18 2^14:2 |
| Task 2 on Resource 0 | 99 | 385128 / 6.018 | |
| Task 2 on Resource 1 | 198 | 280 / 0.004 | |
| Task 5 on Resource 2 | 80 | 308 / 0.005 | |
| Task 6 on Resource 2 | 20 | 289 / 0.005 | |

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3640 cycles
- Task 0 at 1930.000 ms, 3780 cycles
- Task 0 at 1940.000 ms, 3120 cycles
- Task 2 at 1945.000 ms, 3120 cycles
- Task 0 at 1950.000 ms, 3664 cycles
- Task 5 at 1950.000 ms, 3664 cycles
- Task 0 at 1960.000 ms, 3168 cycles
- Task 2 at 1965.000 ms, 3080 cycles
- Task 0 at 1970.000 ms, 3264 cycles
- Task 5 at 1975.000 ms, 3076 cycles
- Task 0 at 1980.000 ms, 3084 cycles
- Task 0 at 1990.000 ms, 3048 cycles


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
     Finished in 7.02s
```
