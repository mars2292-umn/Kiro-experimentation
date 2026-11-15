# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:e1faf204ae173e8765f2f5c1ee1724400a4644cd28482c4dc9f98838e5420e93 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.9 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77426 cycles (1.210 ms), log mean 83860 cycles (1.310 ms): difference 6435 cycles (8.31 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 6435 cycles (0.101 ms) per log Job over its 49 Jobs, interrupted by 152 Kernel entries: 2074 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75266 max 83029, log min 81430 max 88842 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148909 ticks (595636 cycles) against its largest execution 88842 cycles; control's largest response 41467 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4281 / 66.9 | 3510 | 2^12:189 2^13:10 |
| 2 | 99 | 3685 / 57.6 | 3307 | 2^12:99 |
| 3 | 49 | 3208 / 50.1 | 3158 | 2^12:49 |
| 5 | 79 | 4281 / 66.9 | 3592 | 2^12:69 2^13:10 |
| 6 | 19 | 3760 / 58.8 | 3716 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.592 | 0 | 0 | 0 |
| 2 | 99 | 7.711 | 0 | 0 | 0 |
| 3 | 49 | 9.307 | 0 | 0 | 0 |
| 4 | 110 | 6.214 | 111 | 2148 | 2042 |
| 5 | 80 | 8.004 | 0 | 0 | 0 |
| 6 | 20 | 1.941 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 83029 / 1.297 (min 75266, mean 77426) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 404205 / 6.316 (min 399994, mean 400657) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 88842 / 1.388 (min 81430, mean 83860) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 7157 / 0.112 (min 3410, mean 4817) | 0 above the Budget; 2^12:67 2^13:43 |
| Task 5 entry | 80 | 10463 / 0.163 (min 6596, mean 7747) | 0 above the Budget; 2^13:57 2^14:23 |
| Task 6 entry | 20 | 11374 / 0.178 (min 7826, mean 8187) | 0 above the Budget; 2^13:18 2^14:2 |
| Task 2 on Resource 0 | 99 | 389221 / 6.082 | |
| Task 2 on Resource 1 | 198 | 479 / 0.007 | |
| Task 5 on Resource 2 | 80 | 506 / 0.008 | |
| Task 6 on Resource 2 | 20 | 3966 / 0.062 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 3060 / 47.81 | 2895 |
| timer | 650 | 3515 / 54.92 | 2949 |
| intake | 111 | 2260 / 35.31 | 2155 |
| lock | 397 | 453 / 7.08 | 451 |
| unlock | 397 | 1833 / 28.64 | 1621 |
| job_end | 558 | 2003 / 31.30 | 1743 |
| now | 49082 | 229 / 3.58 | 226 |
| dma | 20 | 706 / 11.03 | 706 |
| epilogue | 558 | 1248 / 19.50 | 710 |
| view | 753 | 329 / 5.14 | 300 |

Entries that interrupted an executing Job: Task 0: 112, Task 2: 277, Task 3: 152, Task 4: 43, Task 5: 23, Task 6: 2

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3605 cycles
- Task 0 at 1930.000 ms, 3781 cycles
- Task 0 at 1940.000 ms, 3152 cycles
- Task 2 at 1945.000 ms, 3156 cycles
- Task 0 at 1950.000 ms, 4185 cycles
- Task 5 at 1950.000 ms, 4185 cycles
- Task 0 at 1960.000 ms, 3152 cycles
- Task 2 at 1965.000 ms, 3116 cycles
- Task 0 at 1970.000 ms, 3449 cycles
- Task 5 at 1975.000 ms, 3100 cycles
- Task 0 at 1980.000 ms, 3112 cycles
- Task 0 at 1990.000 ms, 3040 cycles


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
     Finished in 8.06s
```
