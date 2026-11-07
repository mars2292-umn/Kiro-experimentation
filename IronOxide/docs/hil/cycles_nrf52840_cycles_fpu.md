# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:43bf3c0e2a0625c9699cbb205da250beb2b25c9eca7a28962c2dabeedca0da0d (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.2 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 76989 cycles (1.203 ms), filter mean 82297 cycles (1.286 ms): difference 5309 cycles (6.90 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5309 cycles (0.083 ms) per log Job over its 49 Jobs, interrupted by 131 Kernel entries: 1986 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74890 max 81549, log min 80732 max 89213 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 145852 ticks (583408 cycles) against its largest execution 89213 cycles; control's largest response 40652 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 3943 / 61.6 | 3289 | 2^12:199 |
| 2 | 99 | 3415 / 53.4 | 3184 | 2^12:99 |
| 3 | 49 | 3340 / 52.2 | 3291 | 2^12:49 |
| 5 | 49 | 3340 / 52.2 | 3291 | 2^12:49 |
| 6 | 19 | 3344 / 52.2 | 3295 | 2^12:19 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.541 | 0 | 0 | 0 |
| 2 | 99 | 7.645 | 0 | 0 | 0 |
| 3 | 49 | 10.333 | 0 | 0 | 0 |
| 4 | 109 | 6.934 | 110 | 2051 | 1892 |
| 5 | 49 | 9.116 | 0 | 0 | 0 |
| 6 | 20 | 1.732 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 81549 / 1.274 (min 74890, mean 76989) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 401943 / 6.280 (min 397752, mean 398312) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 79360 / 1.240 (min 75031, mean 75492) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 109 | 6768 / 0.106 (min 3061, mean 4386) | 0 above the Budget; 2^12:68 2^13:41 |
| Task 5 entry | 49 | 89213 / 1.394 (min 80732, mean 82297) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 10279 / 0.161 (min 10195, mean 10216) | 0 above the Budget; 2^14:20 |
| Task 2 on Resource 0 | 99 | 385355 / 6.021 | |
| Task 2 on Resource 1 | 198 | 265 / 0.004 | |
| Task 6 on Resource 2 | 20 | 274 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 526 | 2803 / 43.80 | 2646 |
| timer | 623 | 3223 / 50.36 | 2902 |
| intake | 110 | 2178 / 34.03 | 2017 |
| lock | 317 | 464 / 7.25 | 463 |
| unlock | 317 | 1780 / 27.81 | 1601 |
| job_end | 526 | 1920 / 30.00 | 1703 |
| now | 67939 | 242 / 3.78 | 241 |
| dma | 20 | 712 / 11.12 | 712 |
| epilogue | 526 | 1219 / 19.05 | 667 |
| view | 700 | 328 / 5.12 | 294 |

Entries that interrupted an executing Job: Task 0: 111, Task 2: 269, Task 3: 10, Task 4: 41, Task 5: 131, Task 6: 20

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3284 cycles
- Task 5 at 1924.000 ms, 3284 cycles
- Task 2 at 1925.000 ms, 3343 cycles
- Task 0 at 1930.000 ms, 3859 cycles
- Task 0 at 1940.000 ms, 3004 cycles
- Task 2 at 1945.000 ms, 3004 cycles
- Task 0 at 1950.000 ms, 3595 cycles
- Task 0 at 1960.000 ms, 3004 cycles
- Task 2 at 1965.000 ms, 2984 cycles
- Task 0 at 1970.000 ms, 3363 cycles
- Task 0 at 1980.000 ms, 2928 cycles
- Task 0 at 1990.000 ms, 2852 cycles


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
     Finished in 7.38s
```
