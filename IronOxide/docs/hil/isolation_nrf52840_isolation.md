# HIL isolation suite: the MPU access matrix and the EasyDMA cases (task 7.4, R16.8, R17.7)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:26fdf8e4058f775742563aa7c241f16a218e682bb8942f850b10a671417143aa (the isolation configuration `nrf52840_isolation`).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | the report ran after 27.9 s (operating duration 3.6 s) |
| MPU access matrix (R16.8, R16.4) | pass | part0.control: 23 attempts (23 with an address), 23 fault events (21 MemManage, 2 BusFault), 23 followed by END_JOB \| part1.log: 23 attempts (21 with an address), 21 fault events (19 MemManage, 2 BusFault), 21 followed by END_JOB \| part2.sampler: 23 attempts (21 with an address), 21 fault events (19 MemManage, 2 BusFault), 21 followed by END_JOB \| fault record: every one of the 65 attempts has its fault with the attempted address and class (IACCVIOL for fetches, DACCVIOL with MMFAR for data, a BusFault for the PPB) |
| walkers' counters (every attempt made, none returned) | pass | control 23/23 attempts, survived 0; log 23/23, survived 0; sampler's counters are since part2's restart at 65 ms and judged by its events |
| EasyDMA cases (R17.7, R17.1, R17.2) | pass | 5 refused requests (stack buffer, another Partition's RAM, Kernel RAM, a buffer past the RAM's end, an absent channel; 5 expected), 5 ended the Job; 2 accepted (2 expected); the buffer at 0x20019034 inside part2's RAM above its stack; SPIM0 after the run: RXD.PTR 536973364 RXD.MAXCNT 16 TXD.PTR 0 TXD.MAXCNT 0 RXD.LIST 0 TXD.LIST 0 ENABLE 0 STARTED 0 END 0 RXD.AMOUNT 0 (the accepted request's pointer and count, nothing else written, no transfer started); sampler at case 35 (at least 30 expected: every case and attempt made, then idle Jobs) |
| Partition restart stops its EasyDMA channels first (R17.6) | pass | part2 restarted at 73 ms; actions in order: stop 0x40003014, stop 0x40003014, reinit part2; the stop task (0x40003014, SPIM0.TASKS_STOP) precedes the re-initialization |
| Kernel events | pass | 145 events mirrored (145 held, 0 of them the post-detach reboot's): memmanage of Task 0: 21, memmanage of Task 3: 19, memmanage of Task 6: 19, busfault of Task 0: 2, busfault of Task 3: 2, busfault of Task 6: 2, overrun of Task 5: 1, operating_end of Task none: 1, dma_rejected of Task 6: 5, partition_restart of Task none: 1, safe_state of Task none: 1, end_job of Task 0: 23, end_job of Task 3: 21, end_job of Task 6: 26, debugger of Task none: 1; the trace never froze during the run |
| register patterns across preemptions (HW-06, HW-10, R7.5) | pass | 141 probe runs of filter (FPU Task), 0 with a changed register, mask 0x0; FPSCR of the first Job 0x1000000 |
| Hardware_Model differential tests (R49.3) | pass | 14 rules matched, 0 mismatched |

## The access matrix

### part0.control (Task 0, Partition 0)

| # | Attempt | Address | Expected | Fault recorded |
|---|---|---|---|---|
| 0 | read part1's code | 0x00028100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00028100 |
| 1 | write part1's code | 0x00028100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00028100 |
| 2 | fetch from part1's code | 0x00028100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00028100 |
| 3 | read part1's data | 0x20011100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100 |
| 4 | write part1's data | 0x20011100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100 |
| 5 | write into part1's stack | 0x20010010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20010010 |
| 6 | read part1's peripheral window | 0x40006000 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000 |
| 7 | write part1's peripheral window | 0x40006000 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000 |
| 8 | read part2's code | 0x00030100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100 |
| 9 | write part2's code | 0x00030100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100 |
| 10 | fetch from part2's code | 0x00030100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00030100 |
| 11 | read part2's data | 0x20019100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100 |
| 12 | write part2's data | 0x20019100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100 |
| 13 | write into part2's stack | 0x20018010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20018010 |
| 14 | read part2's peripheral window | 0x40003000 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003000 |
| 15 | write part2's peripheral window | 0x40003000 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003000 |
| 16 | read the Kernel's RAM | 0x20000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100 |
| 17 | write the Kernel's RAM | 0x20000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100 |
| 18 | read the Kernel's flash | 0x00000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00000100 |
| 19 | fetch from the Kernel's flash | 0x00000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00000100 |
| 20 | read the PPB (NVIC_ISER0) | 0xe000e100 | busfault of the Partition, then END_JOB | cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100 |
| 21 | write the PPB (NVIC_ISER0) | 0xe000e100 | busfault of the Partition, then END_JOB | cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100 |
| 22 | write the word below the own stack (R16.5) | 0x20007ffc | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20007ffc |

### part1.log (Task 3, Partition 1)

| # | Attempt | Address | Expected | Fault recorded |
|---|---|---|---|---|
| 0 | read part0's code | 0x00020100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00020100 |
| 1 | write part0's code | 0x00020100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00020100 |
| 2 | fetch from part0's code | 0x00020100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00020100 |
| 3 | read part0's data | 0x20009100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100 |
| 4 | write part0's data | 0x20009100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100 |
| 5 | write into part0's stack | 0x20008010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20008010 |
| 6 | read part0's peripheral window | skipped (no window) | no attempt | - |
| 7 | write part0's peripheral window | skipped (no window) | no attempt | - |
| 8 | read part2's code | 0x00030100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100 |
| 9 | write part2's code | 0x00030100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100 |
| 10 | fetch from part2's code | 0x00030100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00030100 |
| 11 | read part2's data | 0x20019100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100 |
| 12 | write part2's data | 0x20019100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100 |
| 13 | write into part2's stack | 0x20018010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20018010 |
| 14 | read part2's peripheral window | 0x40003000 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003000 |
| 15 | write part2's peripheral window | 0x40003000 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003000 |
| 16 | read the Kernel's RAM | 0x20000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100 |
| 17 | write the Kernel's RAM | 0x20000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100 |
| 18 | read the Kernel's flash | 0x00000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00000100 |
| 19 | fetch from the Kernel's flash | 0x00000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00000100 |
| 20 | read the PPB (NVIC_ISER0) | 0xe000e100 | busfault of the Partition, then END_JOB | cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100 |
| 21 | write the PPB (NVIC_ISER0) | 0xe000e100 | busfault of the Partition, then END_JOB | cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100 |
| 22 | write the word below the own stack (R16.5) | 0x2000fffc | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x2000fffc |

### part2.sampler (Task 6, Partition 2)

| # | Attempt | Address | Expected | Fault recorded |
|---|---|---|---|---|
| 0 | read part0's code | 0x00020100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00020100 |
| 1 | write part0's code | 0x00020100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00020100 |
| 2 | fetch from part0's code | 0x00020100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00020100 |
| 3 | read part0's data | 0x20009100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100 |
| 4 | write part0's data | 0x20009100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100 |
| 5 | write into part0's stack | 0x20008010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20008010 |
| 6 | read part0's peripheral window | skipped (no window) | no attempt | - |
| 7 | write part0's peripheral window | skipped (no window) | no attempt | - |
| 8 | read part1's code | 0x00028100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00028100 |
| 9 | write part1's code | 0x00028100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00028100 |
| 10 | fetch from part1's code | 0x00028100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00028100 |
| 11 | read part1's data | 0x20011100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100 |
| 12 | write part1's data | 0x20011100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100 |
| 13 | write into part1's stack | 0x20010010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20010010 |
| 14 | read part1's peripheral window | 0x40006000 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000 |
| 15 | write part1's peripheral window | 0x40006000 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000 |
| 16 | read the Kernel's RAM | 0x20000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100 |
| 17 | write the Kernel's RAM | 0x20000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100 |
| 18 | read the Kernel's flash | 0x00000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00000100 |
| 19 | fetch from the Kernel's flash | 0x00000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00000100 |
| 20 | read the PPB (NVIC_ISER0) | 0xe000e100 | busfault of the Partition, then END_JOB | cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100 |
| 21 | write the PPB (NVIC_ISER0) | 0xe000e100 | busfault of the Partition, then END_JOB | cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100 |
| 22 | write the word below the own stack (R16.5) | 0x20017ffc | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20017ffc |

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 6.499 | memmanage | 0 | 0 | 0x0 |
| 2 | 6.499 | end_job | 0 | 0 | 0x2 |
| 3 | 12.873 | memmanage | 0 | 0 | 0x0 |
| 4 | 12.873 | end_job | 0 | 0 | 0x2 |
| 5 | 12.873 | memmanage | 3 | 1 | 0x0 |
| 6 | 12.873 | end_job | 3 | 1 | 0x2 |
| 7 | 20.008 | memmanage | 0 | 0 | 0x0 |
| 8 | 20.008 | end_job | 0 | 0 | 0x2 |
| 9 | 31.263 | memmanage | 0 | 0 | 0x0 |
| 10 | 31.263 | end_job | 0 | 0 | 0x2 |
| 11 | 40.008 | memmanage | 0 | 0 | 0x0 |
| 12 | 40.008 | end_job | 0 | 0 | 0x2 |
| 13 | 40.008 | memmanage | 3 | 1 | 0x0 |
| 14 | 40.008 | end_job | 3 | 1 | 0x2 |
| 15 | 51.254 | memmanage | 0 | 0 | 0x0 |
| 16 | 51.254 | end_job | 0 | 0 | 0x2 |
| 17 | 60.008 | memmanage | 0 | 0 | 0x0 |
| 18 | 60.008 | end_job | 0 | 0 | 0x2 |
| 19 | 71.261 | memmanage | 0 | 0 | 0x0 |
| 20 | 71.261 | end_job | 0 | 0 | 0x2 |
| 21 | 73.110 | overrun | 5 | 2 | 0xea8a0 |
| 22 | 73.110 | partition_restart | - | 2 | 0x6 |
| 23 | 80.008 | memmanage | 0 | 0 | 0x0 |
| 24 | 80.008 | end_job | 0 | 0 | 0x2 |
| 25 | 80.008 | memmanage | 3 | 1 | 0x0 |
| 26 | 80.008 | end_job | 3 | 1 | 0x2 |
| 27 | 91.253 | memmanage | 0 | 0 | 0x0 |
| 28 | 91.253 | end_job | 0 | 0 | 0x2 |
| 29 | 100.008 | memmanage | 0 | 0 | 0x0 |
| 30 | 100.008 | end_job | 0 | 0 | 0x2 |
| 31 | 111.261 | memmanage | 0 | 0 | 0x0 |
| 32 | 111.261 | end_job | 0 | 0 | 0x2 |
| 33 | 120.008 | memmanage | 0 | 0 | 0x0 |
| 34 | 120.008 | end_job | 0 | 0 | 0x2 |
| 35 | 120.008 | memmanage | 3 | 1 | 0x0 |
| 36 | 120.008 | end_job | 3 | 1 | 0x2 |
| 37 | 131.267 | memmanage | 0 | 0 | 0x0 |
| 38 | 131.267 | end_job | 0 | 0 | 0x2 |
| 39 | 140.008 | memmanage | 0 | 0 | 0x0 |
| 40 | 140.008 | end_job | 0 | 0 | 0x2 |
| 41 | 151.305 | memmanage | 0 | 0 | 0x0 |
| 42 | 151.305 | end_job | 0 | 0 | 0x2 |
| 43 | 160.008 | memmanage | 0 | 0 | 0x0 |
| 44 | 160.008 | end_job | 0 | 0 | 0x2 |
| 45 | 160.008 | memmanage | 3 | 1 | 0x0 |
| 46 | 160.008 | end_job | 3 | 1 | 0x2 |
| 47 | 171.252 | memmanage | 0 | 0 | 0x0 |
| 48 | 171.252 | end_job | 0 | 0 | 0x2 |
| 49 | 180.008 | memmanage | 0 | 0 | 0x0 |
| 50 | 180.008 | end_job | 0 | 0 | 0x2 |
| 51 | 190.268 | memmanage | 0 | 0 | 0x0 |
| 52 | 190.268 | end_job | 0 | 0 | 0x2 |
| 53 | 200.008 | busfault | 0 | 0 | 0x0 |
| 54 | 200.008 | end_job | 0 | 0 | 0x3 |
| 55 | 211.266 | busfault | 0 | 0 | 0x0 |
| 56 | 211.266 | end_job | 0 | 0 | 0x3 |
| 57 | 211.266 | memmanage | 3 | 1 | 0x0 |
| 58 | 211.266 | end_job | 3 | 1 | 0x2 |
| 59 | 212.542 | dma_rejected | 6 | 2 | 0x0 |
| 60 | 212.542 | end_job | 6 | 2 | 0xd |
| 61 | 220.008 | memmanage | 0 | 0 | 0x0 |
| 62 | 220.008 | end_job | 0 | 0 | 0x2 |
| 63 | 312.666 | dma_rejected | 6 | 2 | 0x0 |
| 64 | 312.666 | end_job | 6 | 2 | 0xd |
| 65 | 320.008 | memmanage | 3 | 1 | 0x0 |
| 66 | 320.008 | end_job | 3 | 1 | 0x2 |
| 67 | 360.205 | memmanage | 3 | 1 | 0x0 |
| 68 | 360.205 | end_job | 3 | 1 | 0x2 |
| 69 | 411.268 | memmanage | 3 | 1 | 0x0 |
| 70 | 411.268 | end_job | 3 | 1 | 0x2 |
| 71 | 412.888 | dma_rejected | 6 | 2 | 0x0 |
| 72 | 412.888 | end_job | 6 | 2 | 0xd |
| 73 | 440.008 | memmanage | 3 | 1 | 0x0 |
| 74 | 440.008 | end_job | 3 | 1 | 0x2 |
| 75 | 480.213 | memmanage | 3 | 1 | 0x0 |
| 76 | 480.213 | end_job | 3 | 1 | 0x2 |
| 77 | 512.592 | dma_rejected | 6 | 2 | 0x0 |
| 78 | 512.592 | end_job | 6 | 2 | 0xd |
| 79 | 520.202 | memmanage | 3 | 1 | 0x0 |
| 80 | 520.202 | end_job | 3 | 1 | 0x2 |
| 81 | 560.008 | memmanage | 3 | 1 | 0x0 |
| 82 | 560.008 | end_job | 3 | 1 | 0x2 |
| 83 | 611.410 | memmanage | 3 | 1 | 0x0 |
| 84 | 611.410 | end_job | 3 | 1 | 0x2 |
| 85 | 612.994 | dma_rejected | 6 | 2 | 0x0 |
| 86 | 612.994 | end_job | 6 | 2 | 0xd |
| 87 | 640.281 | memmanage | 3 | 1 | 0x0 |
| 88 | 640.281 | end_job | 3 | 1 | 0x2 |
| 89 | 680.008 | memmanage | 3 | 1 | 0x0 |
| 90 | 680.008 | end_job | 3 | 1 | 0x2 |
| 91 | 720.248 | memmanage | 3 | 1 | 0x0 |
| 92 | 720.248 | end_job | 3 | 1 | 0x2 |
| 93 | 760.202 | memmanage | 3 | 1 | 0x0 |
| 94 | 760.202 | end_job | 3 | 1 | 0x2 |
| 95 | 811.416 | busfault | 3 | 1 | 0x0 |
| 96 | 811.416 | end_job | 3 | 1 | 0x3 |
| 97 | 812.799 | memmanage | 6 | 2 | 0x0 |
| 98 | 812.799 | end_job | 6 | 2 | 0x2 |
| 99 | 840.205 | busfault | 3 | 1 | 0x0 |
| 100 | 840.205 | end_job | 3 | 1 | 0x3 |
| 101 | 880.210 | memmanage | 3 | 1 | 0x0 |
| 102 | 880.210 | end_job | 3 | 1 | 0x2 |
| 103 | 912.774 | memmanage | 6 | 2 | 0x0 |
| 104 | 912.774 | end_job | 6 | 2 | 0x2 |
| 105 | 1015.909 | memmanage | 6 | 2 | 0x0 |
| 106 | 1015.909 | end_job | 6 | 2 | 0x2 |
| 107 | 1112.638 | memmanage | 6 | 2 | 0x0 |
| 108 | 1112.638 | end_job | 6 | 2 | 0x2 |
| 109 | 1215.792 | memmanage | 6 | 2 | 0x0 |
| 110 | 1215.792 | end_job | 6 | 2 | 0x2 |
| 111 | 1312.762 | memmanage | 6 | 2 | 0x0 |
| 112 | 1312.762 | end_job | 6 | 2 | 0x2 |
| 113 | 1615.876 | memmanage | 6 | 2 | 0x0 |
| 114 | 1615.876 | end_job | 6 | 2 | 0x2 |
| 115 | 1712.679 | memmanage | 6 | 2 | 0x0 |
| 116 | 1712.679 | end_job | 6 | 2 | 0x2 |
| 117 | 1815.901 | memmanage | 6 | 2 | 0x0 |
| 118 | 1815.901 | end_job | 6 | 2 | 0x2 |
| 119 | 1912.731 | memmanage | 6 | 2 | 0x0 |
| 120 | 1912.731 | end_job | 6 | 2 | 0x2 |
| 121 | 2015.814 | memmanage | 6 | 2 | 0x0 |
| 122 | 2015.814 | end_job | 6 | 2 | 0x2 |
| 123 | 2112.666 | memmanage | 6 | 2 | 0x0 |
| 124 | 2112.666 | end_job | 6 | 2 | 0x2 |
| 125 | 2215.886 | memmanage | 6 | 2 | 0x0 |
| 126 | 2215.886 | end_job | 6 | 2 | 0x2 |
| 127 | 2312.539 | memmanage | 6 | 2 | 0x0 |
| 128 | 2312.539 | end_job | 6 | 2 | 0x2 |
| 129 | 2415.992 | memmanage | 6 | 2 | 0x0 |
| 130 | 2415.992 | end_job | 6 | 2 | 0x2 |
| 131 | 2512.805 | memmanage | 6 | 2 | 0x0 |
| 132 | 2512.805 | end_job | 6 | 2 | 0x2 |
| 133 | 2615.801 | memmanage | 6 | 2 | 0x0 |
| 134 | 2615.801 | end_job | 6 | 2 | 0x2 |
| 135 | 2712.773 | memmanage | 6 | 2 | 0x0 |
| 136 | 2712.773 | end_job | 6 | 2 | 0x2 |
| 137 | 2815.856 | busfault | 6 | 2 | 0x0 |
| 138 | 2815.856 | end_job | 6 | 2 | 0x3 |
| 139 | 2912.638 | busfault | 6 | 2 | 0x0 |
| 140 | 2912.638 | end_job | 6 | 2 | 0x3 |
| 141 | 3015.846 | memmanage | 6 | 2 | 0x0 |
| 142 | 3015.846 | end_job | 6 | 2 | 0x2 |
| 143 | 3600.000 | operating_end | - | - | 0x0 |
| 144 | 3600.000 | safe_state | - | - | 0x0 |

## The fault record

```
65 faults recorded (65 held)
   0: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00028100
   1: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00028100
   2: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00020100
   3: cfsr 0x00000001 (IACCVIOL) at 0x00028100
   4: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100
   5: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100
   6: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00020100
   7: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20010010
   8: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000
   9: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000
  10: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100
  11: cfsr 0x00000001 (IACCVIOL) at 0x00020100
  12: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100
  13: cfsr 0x00000001 (IACCVIOL) at 0x00030100
  14: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100
  15: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100
  16: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100
  17: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20018010
  18: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003000
  19: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003000
  20: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100
  21: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100
  22: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100
  23: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00000100
  24: cfsr 0x00000001 (IACCVIOL) at 0x00000100
  25: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  26: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  27: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20008010
  28: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20007ffc
  29: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100
  30: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100
  31: cfsr 0x00000001 (IACCVIOL) at 0x00030100
  32: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100
  33: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100
  34: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20018010
  35: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003000
  36: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003000
  37: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100
  38: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100
  39: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00000100
  40: cfsr 0x00000001 (IACCVIOL) at 0x00000100
  41: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  42: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00020100
  43: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  44: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x2000fffc
  45: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00020100
  46: cfsr 0x00000001 (IACCVIOL) at 0x00020100
  47: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100
  48: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100
  49: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20008010
  50: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00028100
  51: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00028100
  52: cfsr 0x00000001 (IACCVIOL) at 0x00028100
  53: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100
  54: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100
  55: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20010010
  56: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000
  57: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000
  58: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100
  59: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100
  60: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00000100
  61: cfsr 0x00000001 (IACCVIOL) at 0x00000100
  62: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  63: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  64: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20017ffc
```

## The actions record

stop 0x40003014, stop 0x40003014, reinit part2


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/isolation/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=337
sensor=179
log=69
pulse=198
echo=111
blocked_releases=168
filter=141
filter_state_bits=1120403452
sampler=9
dma_ok=2
control_matrix=23
control_survived=0
log_matrix=23
log_survived=0
sampler_dma_case=35
sampler_dma_ptr=536973364
sampler_survived=0
filter_pattern_runs=141
filter_pattern_fail=0
filter_pattern_mask=0
filter_fpscr=16777216
spim0_rxd_ptr=536973364
spim0_rxd_maxcnt=16
spim0_rxd_amount=0
spim0_rxd_list=0
spim0_txd_ptr=0
spim0_txd_maxcnt=0
spim0_txd_list=0
spim0_enable=0
spim0_events_started=0
spim0_events_end=0
spim0_events_stopped=0
first_memmanage_cfsr=2147483778
first_memmanage_mmfar=164096
events=32
event=2
  task=6
  at_ms=1615
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=1615
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=1712
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=1712
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=1815
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=1815
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=1912
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=1912
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2015
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2015
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2112
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2112
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2215
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2215
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2312
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2312
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2415
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2415
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2512
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2512
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2615
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2615
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2712
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2712
  detail=2
  prs_lo=0
  prs_hi=0
event=3
  task=6
  at_ms=2815
  detail=0
  prs_lo=4194304
  prs_hi=0
event=19
  task=6
  at_ms=2815
  detail=3
  prs_lo=0
  prs_hi=0
event=3
  task=6
  at_ms=2912
  detail=0
  prs_lo=4194304
  prs_hi=0
event=19
  task=6
  at_ms=2912
  detail=3
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=3015
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=3015
  detail=2
  prs_lo=0
  prs_hi=0
event=10
  task=255
  at_ms=3600
  detail=0
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=3600
  detail=0
  prs_lo=0
  prs_hi=0
RSK-OK
Firmware exited successfully
     Finished in 7.38s
```
