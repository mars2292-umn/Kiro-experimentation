# HIL coverage run (R34.4, R53.3) over the isolation suite: the MPU access matrix and the EasyDMA cases (task 7.4, R16.8, R17.7)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:6b8969095a85b368f664625b7e6d8cb27c9c134dc8e9a4925da199dd53f551b3 (the isolation configuration `nrf52840_isolation`).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | the report ran after 29.6 s (operating duration 3.6 s) |
| MPU access matrix (R16.8, R16.4) | pass | part0.control: 27 attempts (25 with an address), 25 fault events (23 MemManage, 2 BusFault), 25 followed by END_JOB \| part1.log: 27 attempts (24 with an address), 24 fault events (22 MemManage, 2 BusFault), 24 followed by END_JOB \| part2.sampler: 27 attempts (24 with an address), 24 fault events (22 MemManage, 2 BusFault), 24 followed by END_JOB \| fault record: every one of the 73 attempts has its fault with the attempted address and class (IACCVIOL for fetches, DACCVIOL with MMFAR for data, a BusFault for the PPB) |
| walkers' counters (every attempt made, none returned) | pass | control 27/27 attempts, survived 0; log 27/27, survived 0; sampler's counters are since part2's restart at 65 ms and judged by its events |
| EasyDMA cases (R17.7, R17.1, R17.2) | pass | 5 refused requests (stack buffer, another Partition's RAM, Kernel RAM, a buffer past the RAM's end, an absent channel; 5 expected), 5 ended the Job; 2 accepted (2 expected); the buffer at 0x20011045 inside part2's RAM above its stack; SPIM0 after the run: RXD.PTR 536940613 RXD.MAXCNT 16 TXD.PTR 0 TXD.MAXCNT 0 RXD.LIST 0 TXD.LIST 0 ENABLE 0 STARTED 0 END 0 RXD.AMOUNT 0 (the accepted request's pointer and count, nothing else written, no transfer started); sampler at case 35 (at least 34 expected: every case and attempt made, then idle Jobs) |
| Partition restart stops its EasyDMA channels first (R17.6) | pass | part2 restarted at 71 ms; actions in order: stop 0x40003014, stop 0x40003014, reinit part2; the stop task (0x40003014, SPIM0.TASKS_STOP) precedes the re-initialization |
| Kernel events | pass | 161 events mirrored (161 held, 0 of them the post-detach reboot's): memmanage of Task 0: 23, memmanage of Task 3: 22, memmanage of Task 6: 22, busfault of Task 0: 2, busfault of Task 3: 2, busfault of Task 6: 2, overrun of Task 5: 1, operating_end of Task none: 1, dma_rejected of Task 6: 5, partition_restart of Task none: 1, safe_state of Task none: 1, end_job of Task 0: 25, end_job of Task 3: 24, end_job of Task 6: 29, debugger of Task none: 1; the trace never froze during the run |
| register patterns across preemptions (HW-06, HW-10, R7.5) | pass | 141 probe runs of filter (FPU Task), 0 with a changed register, mask 0x0; FPSCR of the first Job 0x1000000 |
| Hardware_Model differential tests (R49.3) | pass | 14 rules matched, 0 mismatched |

## The access matrix

### part0.control (Task 0, Partition 0)

| # | Attempt | Address | Expected | Fault recorded |
|---|---|---|---|---|
| 0 | read part1's code | 0x00040100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100 |
| 1 | write part1's code | 0x00040100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100 |
| 2 | fetch from part1's code | 0x00040100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00040100 |
| 3 | read part1's data | 0x20019100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100 |
| 4 | write part1's data | 0x20019100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100 |
| 5 | write into part1's stack | 0x20018010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20018010 |
| 6 | read part1's peripheral window | 0x40006000 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000 |
| 7 | write part1's peripheral window | 0x40006000 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000 |
| 8 | read part2's code | 0x00038100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100 |
| 9 | write part2's code | 0x00038100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100 |
| 10 | fetch from part2's code | 0x00038100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00038100 |
| 11 | read part2's data | 0x20011100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100 |
| 12 | write part2's data | 0x20011100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100 |
| 13 | write into part2's stack | 0x20010010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20010010 |
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
| 26 | write a disabled subregion of the own RAM region (HW-09) | skipped (no window) | no attempt | - |

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
| 8 | read part2's code | 0x00038100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100 |
| 9 | write part2's code | 0x00038100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100 |
| 10 | fetch from part2's code | 0x00038100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00038100 |
| 11 | read part2's data | 0x20011100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100 |
| 12 | write part2's data | 0x20011100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100 |
| 13 | write into part2's stack | 0x20010010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20010010 |
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
| 24 | write the word below the own stack (R16.5) | 0x20017ffc | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20017ffc |
| 25 | write the own read-only window, an EasyDMA pointer register (DD-08, R17.3) | skipped (no window) | no attempt | - |
| 26 | write a disabled subregion of the own RAM region (HW-09) | 0x2001e010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x2001e010 |

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
| 8 | read part1's code | 0x00040100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100 |
| 9 | write part1's code | 0x00040100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100 |
| 10 | fetch from part1's code | 0x00040100 | memmanage of the Partition, then END_JOB | cfsr 0x00000001 (IACCVIOL) at 0x00040100 |
| 11 | read part1's data | 0x20019100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100 |
| 12 | write part1's data | 0x20019100 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100 |
| 13 | write into part1's stack | 0x20018010 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20018010 |
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
| 24 | write the word below the own stack (R16.5) | 0x2000fffc | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x2000fffc |
| 25 | write the own read-only window, an EasyDMA pointer register (DD-08, R17.3) | 0x40003534 | memmanage of the Partition, then END_JOB | cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003534 |
| 26 | write a disabled subregion of the own RAM region (HW-09) | skipped (no window) | no attempt | - |

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 6.606 | memmanage | 0 | 0 | 0x0 |
| 2 | 6.606 | end_job | 0 | 0 | 0x2 |
| 3 | 12.087 | memmanage | 0 | 0 | 0x0 |
| 4 | 12.087 | end_job | 0 | 0 | 0x2 |
| 5 | 12.087 | memmanage | 3 | 1 | 0x0 |
| 6 | 12.087 | end_job | 3 | 1 | 0x2 |
| 7 | 20.010 | memmanage | 0 | 0 | 0x0 |
| 8 | 20.010 | end_job | 0 | 0 | 0x2 |
| 9 | 30.011 | memmanage | 0 | 0 | 0x0 |
| 10 | 30.011 | end_job | 0 | 0 | 0x2 |
| 11 | 40.010 | memmanage | 0 | 0 | 0x0 |
| 12 | 40.010 | end_job | 0 | 0 | 0x2 |
| 13 | 40.010 | memmanage | 3 | 1 | 0x0 |
| 14 | 40.010 | end_job | 3 | 1 | 0x2 |
| 15 | 50.015 | memmanage | 0 | 0 | 0x0 |
| 16 | 50.015 | end_job | 0 | 0 | 0x2 |
| 17 | 60.023 | memmanage | 0 | 0 | 0x0 |
| 18 | 60.023 | end_job | 0 | 0 | 0x2 |
| 19 | 70.024 | memmanage | 0 | 0 | 0x0 |
| 20 | 70.024 | end_job | 0 | 0 | 0x2 |
| 21 | 71.191 | overrun | 5 | 2 | 0xea9bc |
| 22 | 71.191 | partition_restart | - | 2 | 0x6 |
| 23 | 80.011 | memmanage | 0 | 0 | 0x0 |
| 24 | 80.011 | end_job | 0 | 0 | 0x2 |
| 25 | 80.011 | memmanage | 3 | 1 | 0x0 |
| 26 | 80.011 | end_job | 3 | 1 | 0x2 |
| 27 | 90.021 | memmanage | 0 | 0 | 0x0 |
| 28 | 90.021 | end_job | 0 | 0 | 0x2 |
| 29 | 100.010 | memmanage | 0 | 0 | 0x0 |
| 30 | 100.010 | end_job | 0 | 0 | 0x2 |
| 31 | 110.020 | memmanage | 0 | 0 | 0x0 |
| 32 | 110.020 | end_job | 0 | 0 | 0x2 |
| 33 | 120.010 | memmanage | 0 | 0 | 0x0 |
| 34 | 120.010 | end_job | 0 | 0 | 0x2 |
| 35 | 120.010 | memmanage | 3 | 1 | 0x0 |
| 36 | 120.010 | end_job | 3 | 1 | 0x2 |
| 37 | 130.016 | memmanage | 0 | 0 | 0x0 |
| 38 | 130.016 | end_job | 0 | 0 | 0x2 |
| 39 | 140.010 | memmanage | 0 | 0 | 0x0 |
| 40 | 140.010 | end_job | 0 | 0 | 0x2 |
| 41 | 150.015 | memmanage | 0 | 0 | 0x0 |
| 42 | 150.015 | end_job | 0 | 0 | 0x2 |
| 43 | 160.010 | memmanage | 0 | 0 | 0x0 |
| 44 | 160.010 | end_job | 0 | 0 | 0x2 |
| 45 | 160.010 | memmanage | 3 | 1 | 0x0 |
| 46 | 160.010 | end_job | 3 | 1 | 0x2 |
| 47 | 170.015 | memmanage | 0 | 0 | 0x0 |
| 48 | 170.015 | end_job | 0 | 0 | 0x2 |
| 49 | 180.011 | memmanage | 0 | 0 | 0x0 |
| 50 | 180.011 | end_job | 0 | 0 | 0x2 |
| 51 | 190.015 | memmanage | 0 | 0 | 0x0 |
| 52 | 190.015 | end_job | 0 | 0 | 0x2 |
| 53 | 200.010 | busfault | 0 | 0 | 0x0 |
| 54 | 200.010 | end_job | 0 | 0 | 0x3 |
| 55 | 210.013 | busfault | 0 | 0 | 0x0 |
| 56 | 210.013 | end_job | 0 | 0 | 0x3 |
| 57 | 210.013 | memmanage | 3 | 1 | 0x0 |
| 58 | 210.013 | end_job | 3 | 1 | 0x2 |
| 59 | 211.822 | dma_rejected | 6 | 2 | 0x0 |
| 60 | 211.822 | end_job | 6 | 2 | 0xd |
| 61 | 220.010 | memmanage | 0 | 0 | 0x0 |
| 62 | 220.010 | end_job | 0 | 0 | 0x2 |
| 63 | 230.011 | memmanage | 0 | 0 | 0x0 |
| 64 | 230.011 | end_job | 0 | 0 | 0x2 |
| 65 | 240.010 | memmanage | 0 | 0 | 0x0 |
| 66 | 240.010 | end_job | 0 | 0 | 0x2 |
| 67 | 312.070 | dma_rejected | 6 | 2 | 0x0 |
| 68 | 312.070 | end_job | 6 | 2 | 0xd |
| 69 | 320.309 | memmanage | 3 | 1 | 0x0 |
| 70 | 320.309 | end_job | 3 | 1 | 0x2 |
| 71 | 360.256 | memmanage | 3 | 1 | 0x0 |
| 72 | 360.256 | end_job | 3 | 1 | 0x2 |
| 73 | 410.463 | memmanage | 3 | 1 | 0x0 |
| 74 | 410.463 | end_job | 3 | 1 | 0x2 |
| 75 | 411.905 | dma_rejected | 6 | 2 | 0x0 |
| 76 | 411.905 | end_job | 6 | 2 | 0xd |
| 77 | 440.255 | memmanage | 3 | 1 | 0x0 |
| 78 | 440.255 | end_job | 3 | 1 | 0x2 |
| 79 | 480.262 | memmanage | 3 | 1 | 0x0 |
| 80 | 480.262 | end_job | 3 | 1 | 0x2 |
| 81 | 512.192 | dma_rejected | 6 | 2 | 0x0 |
| 82 | 512.192 | end_job | 6 | 2 | 0xd |
| 83 | 520.010 | memmanage | 3 | 1 | 0x0 |
| 84 | 520.010 | end_job | 3 | 1 | 0x2 |
| 85 | 560.258 | memmanage | 3 | 1 | 0x0 |
| 86 | 560.258 | end_job | 3 | 1 | 0x2 |
| 87 | 611.819 | memmanage | 3 | 1 | 0x0 |
| 88 | 611.819 | end_job | 3 | 1 | 0x2 |
| 89 | 611.819 | dma_rejected | 6 | 2 | 0x0 |
| 90 | 611.819 | end_job | 6 | 2 | 0xd |
| 91 | 640.010 | memmanage | 3 | 1 | 0x0 |
| 92 | 640.010 | end_job | 3 | 1 | 0x2 |
| 93 | 680.264 | memmanage | 3 | 1 | 0x0 |
| 94 | 680.264 | end_job | 3 | 1 | 0x2 |
| 95 | 720.256 | memmanage | 3 | 1 | 0x0 |
| 96 | 720.256 | end_job | 3 | 1 | 0x2 |
| 97 | 760.010 | memmanage | 3 | 1 | 0x0 |
| 98 | 760.010 | end_job | 3 | 1 | 0x2 |
| 99 | 810.459 | busfault | 3 | 1 | 0x0 |
| 100 | 810.459 | end_job | 3 | 1 | 0x3 |
| 101 | 812.084 | memmanage | 6 | 2 | 0x0 |
| 102 | 812.084 | end_job | 6 | 2 | 0x2 |
| 103 | 840.252 | busfault | 3 | 1 | 0x0 |
| 104 | 840.252 | end_job | 3 | 1 | 0x3 |
| 105 | 880.011 | memmanage | 3 | 1 | 0x0 |
| 106 | 880.011 | end_job | 3 | 1 | 0x2 |
| 107 | 912.056 | memmanage | 6 | 2 | 0x0 |
| 108 | 912.056 | end_job | 6 | 2 | 0x2 |
| 109 | 920.314 | memmanage | 3 | 1 | 0x0 |
| 110 | 920.314 | end_job | 3 | 1 | 0x2 |
| 111 | 960.252 | memmanage | 3 | 1 | 0x0 |
| 112 | 960.252 | end_job | 3 | 1 | 0x2 |
| 113 | 1015.167 | memmanage | 6 | 2 | 0x0 |
| 114 | 1015.167 | end_job | 6 | 2 | 0x2 |
| 115 | 1040.255 | memmanage | 3 | 1 | 0x0 |
| 116 | 1040.255 | end_job | 3 | 1 | 0x2 |
| 117 | 1112.126 | memmanage | 6 | 2 | 0x0 |
| 118 | 1112.126 | end_job | 6 | 2 | 0x2 |
| 119 | 1215.215 | memmanage | 6 | 2 | 0x0 |
| 120 | 1215.215 | end_job | 6 | 2 | 0x2 |
| 121 | 1311.758 | memmanage | 6 | 2 | 0x0 |
| 122 | 1311.758 | end_job | 6 | 2 | 0x2 |
| 123 | 1615.166 | memmanage | 6 | 2 | 0x0 |
| 124 | 1615.166 | end_job | 6 | 2 | 0x2 |
| 125 | 1711.953 | memmanage | 6 | 2 | 0x0 |
| 126 | 1711.953 | end_job | 6 | 2 | 0x2 |
| 127 | 1815.214 | memmanage | 6 | 2 | 0x0 |
| 128 | 1815.214 | end_job | 6 | 2 | 0x2 |
| 129 | 1911.964 | memmanage | 6 | 2 | 0x0 |
| 130 | 1911.964 | end_job | 6 | 2 | 0x2 |
| 131 | 2015.344 | memmanage | 6 | 2 | 0x0 |
| 132 | 2015.344 | end_job | 6 | 2 | 0x2 |
| 133 | 2112.067 | memmanage | 6 | 2 | 0x0 |
| 134 | 2112.067 | end_job | 6 | 2 | 0x2 |
| 135 | 2214.923 | memmanage | 6 | 2 | 0x0 |
| 136 | 2214.923 | end_job | 6 | 2 | 0x2 |
| 137 | 2312.134 | memmanage | 6 | 2 | 0x0 |
| 138 | 2312.134 | end_job | 6 | 2 | 0x2 |
| 139 | 2414.794 | memmanage | 6 | 2 | 0x0 |
| 140 | 2414.794 | end_job | 6 | 2 | 0x2 |
| 141 | 2512.065 | memmanage | 6 | 2 | 0x0 |
| 142 | 2512.065 | end_job | 6 | 2 | 0x2 |
| 143 | 2615.107 | memmanage | 6 | 2 | 0x0 |
| 144 | 2615.107 | end_job | 6 | 2 | 0x2 |
| 145 | 2712.056 | memmanage | 6 | 2 | 0x0 |
| 146 | 2712.056 | end_job | 6 | 2 | 0x2 |
| 147 | 2815.107 | busfault | 6 | 2 | 0x0 |
| 148 | 2815.107 | end_job | 6 | 2 | 0x3 |
| 149 | 2912.126 | busfault | 6 | 2 | 0x0 |
| 150 | 2912.126 | end_job | 6 | 2 | 0x3 |
| 151 | 3015.283 | memmanage | 6 | 2 | 0x0 |
| 152 | 3015.283 | end_job | 6 | 2 | 0x2 |
| 153 | 3111.758 | memmanage | 6 | 2 | 0x0 |
| 154 | 3111.758 | end_job | 6 | 2 | 0x2 |
| 155 | 3215.297 | memmanage | 6 | 2 | 0x0 |
| 156 | 3215.297 | end_job | 6 | 2 | 0x2 |
| 157 | 3311.803 | memmanage | 6 | 2 | 0x0 |
| 158 | 3311.803 | end_job | 6 | 2 | 0x2 |
| 159 | 3600.000 | operating_end | - | - | 0x0 |
| 160 | 3600.000 | safe_state | - | - | 0x0 |

## The fault record

```
73 faults recorded (73 held)
   0: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100
   1: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100
   2: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100
   3: cfsr 0x00000001 (IACCVIOL) at 0x00040100
   4: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100
   5: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100
   6: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00030100
   7: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20018010
   8: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000
   9: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000
  10: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100
  11: cfsr 0x00000001 (IACCVIOL) at 0x00030100
  12: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100
  13: cfsr 0x00000001 (IACCVIOL) at 0x00038100
  14: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100
  15: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100
  16: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100
  17: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20010010
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
  28: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100
  29: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100
  30: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20007ffc
  31: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100
  32: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00038100
  33: cfsr 0x00000001 (IACCVIOL) at 0x00038100
  34: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100
  35: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20011100
  36: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20010010
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
  49: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20017ffc
  50: cfsr 0x00000001 (IACCVIOL) at 0x00030100
  51: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x2001e010
  52: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100
  53: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20009100
  54: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20008010
  55: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100
  56: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00040100
  57: cfsr 0x00000001 (IACCVIOL) at 0x00040100
  58: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100
  59: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20019100
  60: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20018010
  61: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000
  62: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40006000
  63: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100
  64: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x20000100
  65: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x00000100
  66: cfsr 0x00000001 (IACCVIOL) at 0x00000100
  67: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  68: cfsr 0x00008200 (PRECISERR|BFARVALID) at 0xe000e100
  69: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100
  70: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40000100
  71: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x2000fffc
  72: cfsr 0x00000082 (DACCVIOL|MMARVALID) at 0x40003534
```

## The actions record

stop 0x40003014, stop 0x40003014, reinit part2


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_isolation/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=335
sensor=179
log=66
pulse=199
echo=111
blocked_releases=167
filter=141
filter_state_bits=1120403452
sampler=6
dma_ok=2
control_matrix=27
control_survived=0
log_matrix=27
log_survived=0
sampler_dma_case=35
sampler_dma_ptr=536940613
sampler_survived=0
filter_pattern_runs=141
filter_pattern_fail=0
filter_pattern_mask=0
filter_fpscr=16777216
filter_fpscr_jobs=141
spim0_rxd_ptr=536940613
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
first_memmanage_mmfar=262400
events=32
event=2
  task=6
  at_ms=1911
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=1911
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
  at_ms=2214
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2214
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
  at_ms=2414
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2414
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
event=2
  task=6
  at_ms=3111
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=3111
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=3215
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=3215
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=3311
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=3311
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
     Finished in 9.07s
```

## Branch coverage of the Task entries under the suite (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 87 % (7 of 8 branch outcomes), part0.echo 75 % (3 of 4 branch outcomes), part0.sensor 75 % (6 of 8 branch outcomes), part1.log 100 % (14 of 14 branch outcomes), part1.pulse 100 % (8 of 8 branch outcomes), part2.filter 68 % (22 of 32 branch outcomes), part2.sampler 65 % (13 of 20 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 8 | 8 | 7 | 87 % | pass |
| `part0.control#cs:state` | 8 | 8 | 7 | 87 % | pass |
| `part0.control#cs:in_cs` | 8 | 8 | 7 | 87 % | pass |
| `part0.echo` | 6 | 4 | 3 | 75 % | pass |
| `part0.sensor` | 9 | 8 | 6 | 75 % | pass |
| `part0.sensor#cs:state` | 9 | 8 | 6 | 75 % | pass |
| `part0.sensor#cs:in_cs` | 9 | 8 | 6 | 75 % | pass |
| `part1.log` | 8 | 14 | 14 | 100 % | pass |
| `part1.pulse` | 8 | 8 | 8 | 100 % | pass |
| `part2.filter` | 23 | 32 | 22 | 68 % | pass |
| `part2.filter#cs:stats` | 23 | 32 | 22 | 68 % | pass |
| `part2.sampler` | 21 | 20 | 13 | 65 % | pass |
| `part2.sampler#cs:stats` | 21 | 20 | 13 | 65 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `p1_demo_part2::address_of` | 0 | 0 |  |
| `p1_demo_part2::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part2::isolation_step` | 10 | 6 | 353:20 true, 359:16 false, 382:20 false, 393:16 false |
| `p1_demo_part2::isolation_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::isolation_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::isolation_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::isolation_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::isolation_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::isolation_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::isolation_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::Stats::record` | 2 | 1 | 423:12 true |
| `p1_demo_part2::filter` | 14 | 10 | 477:19 false, 478:16 true, 487:12 true, 670:54 true |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::inject::window` | 2 | 2 |  |
| `p1_demo_part2::inject::spin_ms` | 4 | 3 | 38:84 false |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 2 | 1 | 728:8 true |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::filter` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::sampler` | 0 | 0 |  |
| `p1_demo_part2::validate_req` | 0 | 0 |  |
| `p1_demo_part2::Stats::new` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part1::matrix_step` | 2 | 2 |  |
| `p1_demo_part1::matrix_step::{closure}` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::acknowledge` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::start` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::respond` | 6 | 6 |  |
| `p1_demo_part1::log` | 6 | 6 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `p1_demo_part0::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part0::matrix_step` | 4 | 3 | 67:8 true |
| `p1_demo_part0::matrix_step::{closure}` | 0 | 0 |  |
| `p1_demo_part0::echo` | 0 | 0 |  |
| `p1_demo_part0::echo::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 405:80 false |
| `p1_demo_part0::control` | 4 | 4 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::echo` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::sensor` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control` | 0 | 0 |  |
| `p1_demo_part0::validate_rep` | 0 | 0 |  |
