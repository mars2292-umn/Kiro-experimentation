# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:3c67f61ac461dbca73f440aa538618e67ddde7e53be668aa4506f97f26e8725d (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.2 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77150 cycles (1.205 ms), filter mean 82424 cycles (1.288 ms): difference 5274 cycles (6.84 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5274 cycles (0.082 ms) per log Job over its 49 Jobs, interrupted by 136 Kernel entries: 1900 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74907 max 85244, log min 80801 max 88774 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 146743 ticks (586972 cycles) against its largest execution 88774 cycles; control's largest response 41477 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 6298 / 98.4 | 3402 | 2^12:198 2^13:1 |
| 2 | 99 | 8252 / 128.9 | 3332 | 2^12:97 2^13:1 2^14:1 |
| 3 | 49 | 3550 / 55.5 | 3379 | 2^12:49 |
| 5 | 49 | 3550 / 55.5 | 3379 | 2^12:49 |
| 6 | 19 | 3398 / 53.1 | 3386 | 2^12:19 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.592 | 0 | 0 | 0 |
| 2 | 99 | 7.700 | 0 | 0 | 0 |
| 3 | 49 | 10.390 | 0 | 0 | 0 |
| 4 | 131 | 7.808 | 132 | 2106 | 1957 |
| 5 | 49 | 9.171 | 0 | 0 | 0 |
| 6 | 20 | 1.737 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 85244 / 1.332 (min 74907, mean 77150) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 405097 / 6.330 (min 398020, mean 398589) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 82783 / 1.293 (min 75070, mean 75945) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 131 | 11969 / 0.187 (min 3074, mean 4502) | 0 above the Budget; 2^12:80 2^13:50 2^14:1 |
| Task 5 entry | 49 | 88774 / 1.387 (min 80801, mean 82424) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 10418 / 0.163 (min 6918, mean 10171) | 0 above the Budget; 2^13:1 2^14:19 |
| Task 2 on Resource 0 | 99 | 391455 / 6.116 | |
| Task 2 on Resource 1 | 198 | 264 / 0.004 | |
| Task 6 on Resource 2 | 20 | 273 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 548 | 2800 / 43.75 | 2656 |
| timer | 664 | 3335 / 52.11 | 2972 |
| intake | 132 | 2174 / 33.97 | 2024 |
| lock | 317 | 463 / 7.23 | 462 |
| unlock | 317 | 1855 / 28.98 | 1664 |
| job_end | 548 | 1977 / 30.89 | 1764 |
| now | 67810 | 239 / 3.73 | 239 |
| dma | 20 | 701 / 10.95 | 701 |
| epilogue | 548 | 1230 / 19.22 | 675 |
| view | 730 | 328 / 5.12 | 295 |

Entries that interrupted an executing Job: Task 0: 118, Task 2: 292, Task 3: 17, Task 4: 53, Task 5: 136, Task 6: 19

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3374 cycles
- Task 5 at 1924.000 ms, 3374 cycles
- Task 2 at 1925.000 ms, 3412 cycles
- Task 0 at 1930.000 ms, 3976 cycles
- Task 0 at 1940.000 ms, 3106 cycles
- Task 2 at 1945.000 ms, 3098 cycles
- Task 0 at 1950.000 ms, 3424 cycles
- Task 0 at 1960.000 ms, 3106 cycles
- Task 2 at 1965.000 ms, 3022 cycles
- Task 0 at 1970.000 ms, 3168 cycles
- Task 0 at 1980.000 ms, 3034 cycles
- Task 0 at 1990.000 ms, 2954 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles-fpu/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=131
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
     Finished in 7.37s
```
