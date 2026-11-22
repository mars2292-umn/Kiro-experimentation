# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:0365e1d1c77b2bfe712e0779e6b4d869b095a811d237aee91b22bf5d0dd3c721 (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 13.1 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77494 cycles (1.211 ms), filter mean 82121 cycles (1.283 ms): difference 4626 cycles (5.97 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 4626 cycles (0.072 ms) per log Job over its 49 Jobs, interrupted by 98 Kernel entries: 2313 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75488 max 79544, log min 82114 max 82198 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 144979 ticks (579916 cycles) against its largest execution 82198 cycles; control's largest response 41733 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 3688 / 57.6 | 3352 | 2^12:199 |
| 2 | 99 | 3496 / 54.6 | 3230 | 2^12:99 |
| 3 | 49 | 3339 / 52.2 | 3339 | 2^12:49 |
| 5 | 49 | 3339 / 52.2 | 3339 | 2^12:49 |
| 6 | 19 | 3323 / 51.9 | 3317 | 2^12:19 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.608 | 0 | 0 | 0 |
| 2 | 99 | 7.737 | 0 | 0 | 0 |
| 3 | 49 | 10.298 | 0 | 0 | 0 |
| 5 | 49 | 9.061 | 0 | 0 | 0 |
| 6 | 20 | 1.811 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 79544 / 1.243 (min 75488, mean 77494) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 406175 / 6.346 (min 401795, mean 402322) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 75653 / 1.182 (min 75593, mean 75594) | 0 above the Budget; 2^15:49 |
| Task 5 entry | 49 | 82198 / 1.284 (min 82114, mean 82121) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 12510 / 0.195 (min 10778, mean 10875) | 0 above the Budget; 2^14:20 |
| Task 2 on Resource 0 | 99 | 386250 / 6.035 | |
| Task 2 on Resource 1 | 198 | 657 / 0.010 | |
| Task 6 on Resource 2 | 20 | 663 / 0.010 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 417 | 2990 / 46.72 | 2842 |
| timer | 483 | 3134 / 48.97 | 2871 |
| lock | 317 | 457 / 7.14 | 456 |
| unlock | 317 | 1769 / 27.64 | 1632 |
| job_end | 417 | 1871 / 29.23 | 1713 |
| now | 40673 | 227 / 3.55 | 224 |
| dma | 1 | 723 / 11.30 | 723 |
| epilogue | 417 | 1236 / 19.31 | 720 |
| view | 565 | 327 / 5.11 | 286 |

Entries that interrupted an executing Job: Task 0: 101, Task 2: 204, Task 5: 98, Task 6: 20

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 1 (timer), 2874 cycles; Task 2: 3 (dispatch, timer), 8642 cycles; Task 3: none; Task 5: 2 (dispatch, timer), 6068 cycles; Task 6: 1 (timer), 2714 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3339 cycles
- Task 5 at 1924.000 ms, 3339 cycles
- Task 2 at 1925.000 ms, 3420 cycles
- Task 0 at 1930.000 ms, 3612 cycles
- Task 0 at 1940.000 ms, 3051 cycles
- Task 2 at 1945.000 ms, 3043 cycles
- Task 0 at 1950.000 ms, 3604 cycles
- Task 0 at 1960.000 ms, 3059 cycles
- Task 2 at 1965.000 ms, 3007 cycles
- Task 0 at 1970.000 ms, 3480 cycles
- Task 0 at 1980.000 ms, 3015 cycles
- Task 0 at 1990.000 ms, 2983 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles-fpu/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=0
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
     Finished in 8.36s
```
