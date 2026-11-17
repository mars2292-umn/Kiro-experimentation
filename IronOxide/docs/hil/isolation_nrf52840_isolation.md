# HIL isolation suite: the MPU access matrix and the EasyDMA cases (task 7.4, R16.8, R17.7)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:c11748c98227f4ebfe14d508e5b8c03ab78a341c4cf023229713b83e68343d71 (the isolation configuration `nrf52840_isolation`).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | the report ran after 28.2 s (operating duration 3.6 s) |
| MPU access matrix (R16.8, R16.4) | pass | part0.control: 26 attempts (25 with an address), 25 fault events (23 MemManage, 2 BusFault), 25 followed by END_JOB \| part1.log: 26 attempts (23 with an address), 23 fault events (21 MemManage, 2 BusFault), 23 followed by END_JOB \| part2.sampler: 26 attempts (24 with an address), 24 fault events (22 MemManage, 2 BusFault), 24 followed by END_JOB \| fault record: every one of the 72 attempts has its fault with the attempted address and class (IACCVIOL for fetches, DACCVIOL with MMFAR for data, a BusFault for the PPB) |
| walkers' counters (every attempt made, none returned) | pass | control 26/26 attempts, survived 0; log 26/26, survived 0; sampler's counters are since part2's restart at 65 ms and judged by its events |
| EasyDMA cases (R17.7, R17.1, R17.2) | pass | 5 refused requests (stack buffer, another Partition's RAM, Kernel RAM, a buffer past the RAM's end, an absent channel; 5 expected), 5 ended the Job; 2 accepted (2 expected); the buffer at 0x20019045 inside part2's RAM above its stack; SPIM0 after the run: RXD.PTR 536973381 RXD.MAXCNT 16 TXD.PTR 0 TXD.MAXCNT 0 RXD.LIST 0 TXD.LIST 0 ENABLE 0 STARTED 0 END 0 RXD.AMOUNT 0 (the accepted request's pointer and count, nothing else written, no transfer started); sampler at case 35 (at least 33 expected: every case and attempt made, then idle Jobs) |
| Partition restart stops its EasyDMA channels first (R17.6) | pass | part2 restarted at 71 ms; actions in order: stop 0x40003014, stop 0x40003014, reinit part2; the stop task (0x40003014, SPIM0.TASKS_STOP) precedes the re-initialization |
| Kernel events | pass | 159 events mirrored (159 held, 0 of them the post-detach reboot's): memmanage of Task 0: 23, memmanage of Task 3: 21, memmanage of Task 6: 22, busfault of Task 0: 2, busfault of Task 3: 2, busfault of Task 6: 2, overrun of Task 5: 1, operating_end of Task none: 1, dma_rejected of Task 6: 5, partition_restart of Task none: 1, safe_state of Task none: 1, end_job of Task 0: 25, end_job of Task 3: 23, end_job of Task 6: 29, debugger of Task none: 1; the trace never froze during the run |
| register patterns across preemptions (HW-06, HW-10, R7.5) | pass | 141 probe runs of filter (FPU Task), 0 with a changed register, mask 0x0; FPSCR of the first Job 0x1000000 |
| Hardware_Model differential tests (R49.3) | pass | 14 rules matched, 0 mismatched |

## The access matrix

### part0.control (Task 0, Partition 0)

| # | Attempt | Address | Expected | Fault recorded |
|---|---|---|---|---|
| 0 | read part1's code | 0x00038100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100 |
| 1 | write part1's code | 0x00038100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100 |
| 2 | fetch from part1's code | 0x00038100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00038100 |
| 3 | read part1's data | 0x20011100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100 |
| 4 | write part1's data | 0x20011100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100 |
| 5 | write into part1's stack | 0x20010010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20010010 |
| 6 | read part1's peripheral window | 0x40006000 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000 |
| 7 | write part1's peripheral window | 0x40006000 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000 |
| 8 | read part2's code | 0x00040100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100 |
| 9 | write part2's code | 0x00040100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100 |
| 10 | fetch from part2's code | 0x00040100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00040100 |
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
| 22 | read the Kernel-owned peripheral CLOCK (R16.2) | 0x40000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100 |
| 23 | write the Kernel-owned peripheral CLOCK (R16.2) | 0x40000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100 |
| 24 | write the word below the own stack (R16.5) | 0x20007ffc | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20007ffc |
| 25 | write the own read-only window, an EasyDMA pointer register (DD-08, R17.3) | skipped (no window) | no attempt | - |

### part1.log (Task 3, Partition 1)

| # | Attempt | Address | Expected | Fault recorded |
|---|---|---|---|---|
| 0 | read part0's code | 0x00030100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100 |
| 1 | write part0's code | 0x00030100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100 |
| 2 | fetch from part0's code | 0x00030100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00030100 |
| 3 | read part0's data | 0x20009100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100 |
| 4 | write part0's data | 0x20009100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100 |
| 5 | write into part0's stack | 0x20008010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20008010 |
| 6 | read part0's peripheral window | skipped (no window) | no attempt | - |
| 7 | write part0's peripheral window | skipped (no window) | no attempt | - |
| 8 | read part2's code | 0x00040100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100 |
| 9 | write part2's code | 0x00040100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100 |
| 10 | fetch from part2's code | 0x00040100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00040100 |
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
| 22 | read the Kernel-owned peripheral CLOCK (R16.2) | 0x40000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100 |
| 23 | write the Kernel-owned peripheral CLOCK (R16.2) | 0x40000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100 |
| 24 | write the word below the own stack (R16.5) | 0x2000fffc | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x2000fffc |
| 25 | write the own read-only window, an EasyDMA pointer register (DD-08, R17.3) | skipped (no window) | no attempt | - |

### part2.sampler (Task 6, Partition 2)

| # | Attempt | Address | Expected | Fault recorded |
|---|---|---|---|---|
| 0 | read part0's code | 0x00030100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100 |
| 1 | write part0's code | 0x00030100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100 |
| 2 | fetch from part0's code | 0x00030100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00030100 |
| 3 | read part0's data | 0x20009100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100 |
| 4 | write part0's data | 0x20009100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100 |
| 5 | write into part0's stack | 0x20008010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20008010 |
| 6 | read part0's peripheral window | skipped (no window) | no attempt | - |
| 7 | write part0's peripheral window | skipped (no window) | no attempt | - |
| 8 | read part1's code | 0x00038100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100 |
| 9 | write part1's code | 0x00038100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100 |
| 10 | fetch from part1's code | 0x00038100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00038100 |
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
| 22 | read the Kernel-owned peripheral CLOCK (R16.2) | 0x40000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100 |
| 23 | write the Kernel-owned peripheral CLOCK (R16.2) | 0x40000100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100 |
| 24 | write the word below the own stack (R16.5) | 0x20017ffc | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20017ffc |
| 25 | write the own read-only window, an EasyDMA pointer register (DD-08, R17.3) | 0x40003534 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003534 |

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 6.606 | memmanage | 0 | 0 | 0x0 |
| 2 | 6.606 | end_job | 0 | 0 | 0x2 |
| 3 | 12.070 | memmanage | 0 | 0 | 0x0 |
| 4 | 12.070 | end_job | 0 | 0 | 0x2 |
| 5 | 12.070 | memmanage | 3 | 1 | 0x0 |
| 6 | 12.070 | end_job | 3 | 1 | 0x2 |
| 7 | 20.010 | memmanage | 0 | 0 | 0x0 |
| 8 | 20.010 | end_job | 0 | 0 | 0x2 |
| 9 | 30.010 | memmanage | 0 | 0 | 0x0 |
| 10 | 30.010 | end_job | 0 | 0 | 0x2 |
| 11 | 40.010 | memmanage | 0 | 0 | 0x0 |
| 12 | 40.010 | end_job | 0 | 0 | 0x2 |
| 13 | 40.010 | memmanage | 3 | 1 | 0x0 |
| 14 | 40.010 | end_job | 3 | 1 | 0x2 |
| 15 | 50.010 | memmanage | 0 | 0 | 0x0 |
| 16 | 50.010 | end_job | 0 | 0 | 0x2 |
| 17 | 60.014 | memmanage | 0 | 0 | 0x0 |
| 18 | 60.014 | end_job | 0 | 0 | 0x2 |
| 19 | 70.018 | memmanage | 0 | 0 | 0x0 |
| 20 | 70.018 | end_job | 0 | 0 | 0x2 |
| 21 | 71.227 | overrun | 5 | 2 | 0xea914 |
| 22 | 71.227 | partition_restart | - | 2 | 0x6 |
| 23 | 80.010 | memmanage | 0 | 0 | 0x0 |
| 24 | 80.010 | end_job | 0 | 0 | 0x2 |
| 25 | 80.010 | memmanage | 3 | 1 | 0x0 |
| 26 | 80.010 | end_job | 3 | 1 | 0x2 |
| 27 | 90.020 | memmanage | 0 | 0 | 0x0 |
| 28 | 90.020 | end_job | 0 | 0 | 0x2 |
| 29 | 100.010 | memmanage | 0 | 0 | 0x0 |
| 30 | 100.010 | end_job | 0 | 0 | 0x2 |
| 31 | 110.011 | memmanage | 0 | 0 | 0x0 |
| 32 | 110.011 | end_job | 0 | 0 | 0x2 |
| 33 | 120.010 | memmanage | 0 | 0 | 0x0 |
| 34 | 120.010 | end_job | 0 | 0 | 0x2 |
| 35 | 120.010 | memmanage | 3 | 1 | 0x0 |
| 36 | 120.010 | end_job | 3 | 1 | 0x2 |
| 37 | 130.017 | memmanage | 0 | 0 | 0x0 |
| 38 | 130.017 | end_job | 0 | 0 | 0x2 |
| 39 | 140.010 | memmanage | 0 | 0 | 0x0 |
| 40 | 140.010 | end_job | 0 | 0 | 0x2 |
| 41 | 150.010 | memmanage | 0 | 0 | 0x0 |
| 42 | 150.010 | end_job | 0 | 0 | 0x2 |
| 43 | 160.010 | memmanage | 0 | 0 | 0x0 |
| 44 | 160.010 | end_job | 0 | 0 | 0x2 |
| 45 | 160.010 | memmanage | 3 | 1 | 0x0 |
| 46 | 160.010 | end_job | 3 | 1 | 0x2 |
| 47 | 170.010 | memmanage | 0 | 0 | 0x0 |
| 48 | 170.010 | end_job | 0 | 0 | 0x2 |
| 49 | 180.010 | memmanage | 0 | 0 | 0x0 |
| 50 | 180.010 | end_job | 0 | 0 | 0x2 |
| 51 | 190.010 | memmanage | 0 | 0 | 0x0 |
| 52 | 190.010 | end_job | 0 | 0 | 0x2 |
| 53 | 200.010 | busfault | 0 | 0 | 0x0 |
| 54 | 200.010 | end_job | 0 | 0 | 0x3 |
| 55 | 200.010 | memmanage | 3 | 1 | 0x0 |
| 56 | 200.010 | end_job | 3 | 1 | 0x2 |
| 57 | 203.717 | dma_rejected | 6 | 2 | 0x0 |
| 58 | 203.717 | end_job | 6 | 2 | 0xd |
| 59 | 210.010 | busfault | 0 | 0 | 0x0 |
| 60 | 210.010 | end_job | 0 | 0 | 0x3 |
| 61 | 220.010 | memmanage | 0 | 0 | 0x0 |
| 62 | 220.010 | end_job | 0 | 0 | 0x2 |
| 63 | 230.011 | memmanage | 0 | 0 | 0x0 |
| 64 | 230.011 | end_job | 0 | 0 | 0x2 |
| 65 | 240.010 | memmanage | 0 | 0 | 0x0 |
| 66 | 240.010 | end_job | 0 | 0 | 0x2 |
| 67 | 303.896 | dma_rejected | 6 | 2 | 0x0 |
| 68 | 303.896 | end_job | 6 | 2 | 0xd |
| 69 | 320.303 | memmanage | 3 | 1 | 0x0 |
| 70 | 320.303 | end_job | 3 | 1 | 0x2 |
| 71 | 360.243 | memmanage | 3 | 1 | 0x0 |
| 72 | 360.243 | end_job | 3 | 1 | 0x2 |
| 73 | 400.010 | memmanage | 3 | 1 | 0x0 |
| 74 | 400.010 | end_job | 3 | 1 | 0x2 |
| 75 | 403.820 | dma_rejected | 6 | 2 | 0x0 |
| 76 | 403.820 | end_job | 6 | 2 | 0xd |
| 77 | 440.246 | memmanage | 3 | 1 | 0x0 |
| 78 | 440.246 | end_job | 3 | 1 | 0x2 |
| 79 | 480.243 | memmanage | 3 | 1 | 0x0 |
| 80 | 480.243 | end_job | 3 | 1 | 0x2 |
| 81 | 503.982 | dma_rejected | 6 | 2 | 0x0 |
| 82 | 503.982 | end_job | 6 | 2 | 0xd |
| 83 | 520.010 | memmanage | 3 | 1 | 0x0 |
| 84 | 520.010 | end_job | 3 | 1 | 0x2 |
| 85 | 560.246 | memmanage | 3 | 1 | 0x0 |
| 86 | 560.246 | end_job | 3 | 1 | 0x2 |
| 87 | 600.265 | memmanage | 3 | 1 | 0x0 |
| 88 | 600.265 | end_job | 3 | 1 | 0x2 |
| 89 | 603.973 | dma_rejected | 6 | 2 | 0x0 |
| 90 | 603.973 | end_job | 6 | 2 | 0xd |
| 91 | 640.010 | memmanage | 3 | 1 | 0x0 |
| 92 | 640.010 | end_job | 3 | 1 | 0x2 |
| 93 | 680.249 | memmanage | 3 | 1 | 0x0 |
| 94 | 680.249 | end_job | 3 | 1 | 0x2 |
| 95 | 720.243 | memmanage | 3 | 1 | 0x0 |
| 96 | 720.243 | end_job | 3 | 1 | 0x2 |
| 97 | 760.010 | memmanage | 3 | 1 | 0x0 |
| 98 | 760.010 | end_job | 3 | 1 | 0x2 |
| 99 | 800.268 | busfault | 3 | 1 | 0x0 |
| 100 | 800.268 | end_job | 3 | 1 | 0x3 |
| 101 | 803.872 | memmanage | 6 | 2 | 0x0 |
| 102 | 803.872 | end_job | 6 | 2 | 0x2 |
| 103 | 840.243 | busfault | 3 | 1 | 0x0 |
| 104 | 840.243 | end_job | 3 | 1 | 0x3 |
| 105 | 880.010 | memmanage | 3 | 1 | 0x0 |
| 106 | 880.010 | end_job | 3 | 1 | 0x2 |
| 107 | 903.833 | memmanage | 6 | 2 | 0x0 |
| 108 | 903.833 | end_job | 6 | 2 | 0x2 |
| 109 | 920.299 | memmanage | 3 | 1 | 0x0 |
| 110 | 920.299 | end_job | 3 | 1 | 0x2 |
| 111 | 960.243 | memmanage | 3 | 1 | 0x0 |
| 112 | 960.243 | end_job | 3 | 1 | 0x2 |
| 113 | 1010.443 | memmanage | 6 | 2 | 0x0 |
| 114 | 1010.443 | end_job | 6 | 2 | 0x2 |
| 115 | 1103.738 | memmanage | 6 | 2 | 0x0 |
| 116 | 1103.738 | end_job | 6 | 2 | 0x2 |
| 117 | 1210.669 | memmanage | 6 | 2 | 0x0 |
| 118 | 1210.669 | end_job | 6 | 2 | 0x2 |
| 119 | 1303.681 | memmanage | 6 | 2 | 0x0 |
| 120 | 1303.681 | end_job | 6 | 2 | 0x2 |
| 121 | 1610.445 | memmanage | 6 | 2 | 0x0 |
| 122 | 1610.445 | end_job | 6 | 2 | 0x2 |
| 123 | 1703.738 | memmanage | 6 | 2 | 0x0 |
| 124 | 1703.738 | end_job | 6 | 2 | 0x2 |
| 125 | 1810.756 | memmanage | 6 | 2 | 0x0 |
| 126 | 1810.756 | end_job | 6 | 2 | 0x2 |
| 127 | 1903.681 | memmanage | 6 | 2 | 0x0 |
| 128 | 1903.681 | end_job | 6 | 2 | 0x2 |
| 129 | 2010.439 | memmanage | 6 | 2 | 0x0 |
| 130 | 2010.439 | end_job | 6 | 2 | 0x2 |
| 131 | 2103.896 | memmanage | 6 | 2 | 0x0 |
| 132 | 2103.896 | end_job | 6 | 2 | 0x2 |
| 133 | 2210.451 | memmanage | 6 | 2 | 0x0 |
| 134 | 2210.451 | end_job | 6 | 2 | 0x2 |
| 135 | 2303.923 | memmanage | 6 | 2 | 0x0 |
| 136 | 2303.923 | end_job | 6 | 2 | 0x2 |
| 137 | 2410.767 | memmanage | 6 | 2 | 0x0 |
| 138 | 2410.767 | end_job | 6 | 2 | 0x2 |
| 139 | 2503.927 | memmanage | 6 | 2 | 0x0 |
| 140 | 2503.927 | end_job | 6 | 2 | 0x2 |
| 141 | 2610.443 | memmanage | 6 | 2 | 0x0 |
| 142 | 2610.443 | end_job | 6 | 2 | 0x2 |
| 143 | 2703.833 | memmanage | 6 | 2 | 0x0 |
| 144 | 2703.833 | end_job | 6 | 2 | 0x2 |
| 145 | 2810.450 | busfault | 6 | 2 | 0x0 |
| 146 | 2810.450 | end_job | 6 | 2 | 0x3 |
| 147 | 2903.738 | busfault | 6 | 2 | 0x0 |
| 148 | 2903.738 | end_job | 6 | 2 | 0x3 |
| 149 | 3010.672 | memmanage | 6 | 2 | 0x0 |
| 150 | 3010.672 | end_job | 6 | 2 | 0x2 |
| 151 | 3103.681 | memmanage | 6 | 2 | 0x0 |
| 152 | 3103.681 | end_job | 6 | 2 | 0x2 |
| 153 | 3210.439 | memmanage | 6 | 2 | 0x0 |
| 154 | 3210.439 | end_job | 6 | 2 | 0x2 |
| 155 | 3303.833 | memmanage | 6 | 2 | 0x0 |
| 156 | 3303.833 | end_job | 6 | 2 | 0x2 |
| 157 | 3600.000 | operating_end | - | - | 0x0 |
| 158 | 3600.000 | safe_state | - | - | 0x0 |

## The fault record

```
72 faults recorded (72 held)
   0: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100
   1: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100
   2: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100
   3: cfsr 0x00000001 (IACCVIOL) at 0x00038100
   4: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100
   5: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100
   6: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100
   7: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20010010
   8: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000
   9: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000
  10: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100
  11: cfsr 0x00000001 (IACCVIOL) at 0x00030100
  12: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100
  13: cfsr 0x00000001 (IACCVIOL) at 0x00040100
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
  26: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20008010
  27: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  28: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100
  29: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100
  30: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20007ffc
  31: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100
  32: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100
  33: cfsr 0x00000001 (IACCVIOL) at 0x00040100
  34: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100
  35: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100
  36: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20018010
  37: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003000
  38: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003000
  39: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100
  40: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100
  41: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00000100
  42: cfsr 0x00000001 (IACCVIOL) at 0x00000100
  43: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  44: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100
  45: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  46: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100
  47: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100
  48: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100
  49: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x2000fffc
  50: cfsr 0x00000001 (IACCVIOL) at 0x00030100
  51: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100
  52: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100
  53: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20008010
  54: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100
  55: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100
  56: cfsr 0x00000001 (IACCVIOL) at 0x00038100
  57: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100
  58: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100
  59: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20010010
  60: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000
  61: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000
  62: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100
  63: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100
  64: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00000100
  65: cfsr 0x00000001 (IACCVIOL) at 0x00000100
  66: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  67: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  68: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100
  69: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100
  70: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20017ffc
  71: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003534
```

## The actions record

stop 0x40003014, stop 0x40003014, reinit part2


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/isolation/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=335
sensor=179
log=67
pulse=199
echo=111
blocked_releases=167
filter=141
filter_state_bits=1120403452
sampler=6
dma_ok=2
control_matrix=26
control_survived=0
log_matrix=26
log_survived=0
sampler_dma_case=35
sampler_dma_ptr=536973381
sampler_survived=0
filter_pattern_runs=141
filter_pattern_fail=0
filter_pattern_mask=0
filter_fpscr=16777216
spim0_rxd_ptr=536973381
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
first_memmanage_mmfar=229632
events=32
event=2
  task=6
  at_ms=1903
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=1903
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2010
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2010
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2103
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2103
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2210
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2210
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2303
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2303
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2410
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2410
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2503
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2503
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2610
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2610
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2703
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2703
  detail=2
  prs_lo=0
  prs_hi=0
event=3
  task=6
  at_ms=2810
  detail=0
  prs_lo=4194304
  prs_hi=0
event=19
  task=6
  at_ms=2810
  detail=3
  prs_lo=0
  prs_hi=0
event=3
  task=6
  at_ms=2903
  detail=0
  prs_lo=4194304
  prs_hi=0
event=19
  task=6
  at_ms=2903
  detail=3
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=3010
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=3010
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=3103
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=3103
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=3210
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=3210
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=3303
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=3303
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
     Finished in 8.40s
```
