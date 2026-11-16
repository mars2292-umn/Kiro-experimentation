# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:d4faae210fcad55506e286ec2510e80a1398acb827111fa97f1d4dd36f9080fc (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.9 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 78014 cycles (1.219 ms), filter mean 83313 cycles (1.302 ms): difference 5299 cycles (6.79 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5299 cycles (0.083 ms) per log Job over its 49 Jobs, interrupted by 130 Kernel entries: 1997 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75377 max 83459, log min 81904 max 86986 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 147439 ticks (589756 cycles) against its largest execution 86986 cycles; control's largest response 41502 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4050 / 63.3 | 3549 | 2^12:199 |
| 2 | 99 | 3662 / 57.2 | 3423 | 2^12:99 |
| 3 | 49 | 3597 / 56.2 | 3552 | 2^12:49 |
| 5 | 49 | 3597 / 56.2 | 3552 | 2^12:49 |
| 6 | 19 | 3609 / 56.4 | 3550 | 2^12:19 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.594 | 0 | 0 | 0 |
| 2 | 99 | 7.714 | 0 | 0 | 0 |
| 3 | 49 | 10.445 | 0 | 0 | 0 |
| 4 | 110 | 6.588 | 111 | 2259 | 2088 |
| 5 | 49 | 9.215 | 0 | 0 | 0 |
| 6 | 20 | 1.784 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 83459 / 1.304 (min 75377, mean 78014) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 404514 / 6.321 (min 400150, mean 400681) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 79201 / 1.238 (min 75489, mean 75887) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 7475 / 0.117 (min 3461, mean 4868) | 0 above the Budget; 2^12:71 2^13:39 |
| Task 5 entry | 49 | 86986 / 1.359 (min 81904, mean 83313) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 11745 / 0.184 (min 7997, mean 10748) | 0 above the Budget; 2^13:5 2^14:15 |
| Task 2 on Resource 0 | 99 | 385985 / 6.031 | |
| Task 2 on Resource 1 | 198 | 479 / 0.007 | |
| Task 6 on Resource 2 | 20 | 488 / 0.008 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 527 | 3226 / 50.41 | 3021 |
| timer | 633 | 3367 / 52.61 | 3089 |
| intake | 111 | 2370 / 37.03 | 2198 |
| lock | 317 | 452 / 7.06 | 451 |
| unlock | 317 | 1897 / 29.64 | 1700 |
| job_end | 527 | 2017 / 31.52 | 1772 |
| now | 49307 | 226 / 3.53 | 225 |
| dma | 20 | 718 / 11.22 | 717 |
| epilogue | 527 | 1241 / 19.39 | 700 |
| view | 702 | 329 / 5.14 | 296 |

Entries that interrupted an executing Job: Task 0: 134, Task 2: 270, Task 3: 5, Task 4: 39, Task 5: 130, Task 6: 15

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3537 cycles
- Task 5 at 1924.000 ms, 3537 cycles
- Task 2 at 1925.000 ms, 3586 cycles
- Task 0 at 1930.000 ms, 3878 cycles
- Task 0 at 1940.000 ms, 3317 cycles
- Task 2 at 1945.000 ms, 3249 cycles
- Task 0 at 1950.000 ms, 3742 cycles
- Task 0 at 1960.000 ms, 3257 cycles
- Task 2 at 1965.000 ms, 3181 cycles
- Task 0 at 1970.000 ms, 3502 cycles
- Task 0 at 1980.000 ms, 3193 cycles
- Task 0 at 1990.000 ms, 3157 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles-fpu/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=110
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
     Finished in 8.08s
```
