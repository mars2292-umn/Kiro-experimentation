# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:c9966efb6aeb79cc4e8024d3024a395c9044b6e7eefe7186fa28b2950b022aa3 (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.8 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77425 cycles (1.210 ms), filter mean 82840 cycles (1.294 ms): difference 5415 cycles (6.99 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5415 cycles (0.085 ms) per log Job over its 49 Jobs, interrupted by 125 Kernel entries: 2123 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75266 max 82217, log min 81578 max 87207 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 147108 ticks (588432 cycles) against its largest execution 87207 cycles; control's largest response 41133 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 6981 / 109.1 | 3442 | 2^12:195 2^13:4 |
| 2 | 99 | 5513 / 86.1 | 3480 | 2^12:88 2^13:11 |
| 3 | 49 | 6885 / 107.6 | 3501 | 2^12:48 2^13:1 |
| 5 | 49 | 6885 / 107.6 | 3501 | 2^12:48 2^13:1 |
| 6 | 19 | 6981 / 109.1 | 3632 | 2^12:18 2^13:1 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.571 | 0 | 0 | 0 |
| 2 | 99 | 7.688 | 0 | 0 | 0 |
| 3 | 49 | 10.423 | 0 | 0 | 0 |
| 4 | 110 | 6.859 | 111 | 2154 | 2038 |
| 5 | 49 | 9.194 | 0 | 0 | 0 |
| 6 | 20 | 1.774 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82217 / 1.285 (min 75266, mean 77425) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 404265 / 6.317 (min 399886, mean 400498) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 80301 / 1.255 (min 75380, mean 76005) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 13479 / 0.211 (min 3410, mean 5383) | 0 above the Budget; 2^12:55 2^13:52 2^14:3 |
| Task 5 entry | 49 | 87207 / 1.363 (min 81578, mean 82840) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 11426 / 0.179 (min 7826, mean 10991) | 0 above the Budget; 2^13:2 2^14:18 |
| Task 2 on Resource 0 | 99 | 385937 / 6.030 | |
| Task 2 on Resource 1 | 198 | 479 / 0.007 | |
| Task 6 on Resource 2 | 20 | 4030 / 0.063 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 527 | 3060 / 47.81 | 2894 |
| timer | 634 | 3311 / 51.73 | 2954 |
| intake | 111 | 2268 / 35.44 | 2151 |
| lock | 317 | 453 / 7.08 | 451 |
| unlock | 317 | 1836 / 28.69 | 1648 |
| job_end | 527 | 1944 / 30.38 | 1750 |
| now | 49091 | 229 / 3.58 | 226 |
| dma | 20 | 709 / 11.08 | 707 |
| epilogue | 527 | 1248 / 19.50 | 698 |
| view | 706 | 329 / 5.14 | 296 |

Entries that interrupted an executing Job: Task 0: 112, Task 2: 270, Task 3: 12, Task 4: 61, Task 5: 125, Task 6: 18

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3476 cycles
- Task 5 at 1924.000 ms, 3476 cycles
- Task 2 at 1925.000 ms, 3481 cycles
- Task 0 at 1930.000 ms, 3485 cycles
- Task 0 at 1940.000 ms, 3152 cycles
- Task 2 at 1945.000 ms, 3156 cycles
- Task 0 at 1950.000 ms, 3437 cycles
- Task 0 at 1960.000 ms, 3208 cycles
- Task 2 at 1965.000 ms, 3080 cycles
- Task 0 at 1970.000 ms, 3233 cycles
- Task 0 at 1980.000 ms, 6493 cycles
- Task 0 at 1990.000 ms, 3040 cycles


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
     Finished in 8.04s
```
