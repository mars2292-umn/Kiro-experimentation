# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:671df398dd26ed3cce69c1ff186a4af8d1efc1dd8d68ff1a58de84cab9234434 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.8 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 78011 cycles (1.219 ms), log mean 84669 cycles (1.323 ms): difference 6658 cycles (8.53 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 6658 cycles (0.104 ms) per log Job over its 49 Jobs, interrupted by 160 Kernel entries: 2039 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75401 max 83471, log min 81692 max 92728 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 150976 ticks (603904 cycles) against its largest execution 92728 cycles; control's largest response 42331 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4342 / 67.8 | 3549 | 2^12:196 2^13:3 |
| 2 | 99 | 3846 / 60.1 | 3432 | 2^12:99 |
| 3 | 49 | 3313 / 51.8 | 3271 | 2^12:49 |
| 5 | 79 | 4342 / 67.8 | 3671 | 2^12:78 2^13:1 |
| 6 | 19 | 3877 / 60.6 | 3833 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.646 | 0 | 0 | 0 |
| 2 | 99 | 7.766 | 0 | 0 | 0 |
| 3 | 49 | 9.436 | 0 | 0 | 0 |
| 4 | 110 | 6.567 | 111 | 2219 | 2086 |
| 5 | 80 | 8.131 | 0 | 0 | 0 |
| 6 | 20 | 1.955 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 83471 / 1.304 (min 75401, mean 78011) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 404500 / 6.320 (min 400010, mean 400713) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 92728 / 1.449 (min 81692, mean 84669) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 7475 / 0.117 (min 3485, mean 4859) | 0 above the Budget; 2^12:71 2^13:39 |
| Task 5 entry | 80 | 10833 / 0.169 (min 6753, mean 7873) | 0 above the Budget; 2^13:59 2^14:21 |
| Task 6 entry | 20 | 8045 / 0.126 (min 7997, mean 8009) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 389081 / 6.079 | |
| Task 2 on Resource 1 | 198 | 479 / 0.007 | |
| Task 5 on Resource 2 | 80 | 4277 / 0.067 | |
| Task 6 on Resource 2 | 20 | 488 / 0.008 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 3226 / 50.41 | 3016 |
| timer | 665 | 3635 / 56.80 | 3082 |
| intake | 111 | 2330 / 36.41 | 2196 |
| lock | 397 | 453 / 7.08 | 451 |
| unlock | 397 | 1901 / 29.70 | 1678 |
| job_end | 558 | 2070 / 32.34 | 1768 |
| now | 49304 | 227 / 3.55 | 225 |
| dma | 20 | 718 / 11.22 | 718 |
| epilogue | 558 | 1266 / 19.78 | 720 |
| view | 754 | 329 / 5.14 | 300 |

Entries that interrupted an executing Job: Task 0: 134, Task 2: 270, Task 3: 160, Task 4: 39, Task 5: 21

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3770 cycles
- Task 0 at 1930.000 ms, 3630 cycles
- Task 0 at 1940.000 ms, 3317 cycles
- Task 2 at 1945.000 ms, 3249 cycles
- Task 0 at 1950.000 ms, 4026 cycles
- Task 5 at 1950.000 ms, 4026 cycles
- Task 0 at 1960.000 ms, 3317 cycles
- Task 2 at 1965.000 ms, 3213 cycles
- Task 0 at 1970.000 ms, 3618 cycles
- Task 5 at 1975.000 ms, 3221 cycles
- Task 0 at 1980.000 ms, 3225 cycles
- Task 0 at 1990.000 ms, 3185 cycles


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
