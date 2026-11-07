# HIL coverage run (R34.4, R53.3) over the isolation suite: the MPU access matrix and the EasyDMA cases (task 7.4, R16.8, R17.7)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:4c0496ec3e4163d57b21d71a980f21310203b39b540784a0506122e96b247837 (the isolation configuration `nrf52840_isolation`).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | the report ran after 28.6 s (operating duration 3.6 s) |
| MPU access matrix (R16.8, R16.4) | pass | part0.control: 25 attempts (25 with an address), 25 fault events (23 MemManage, 2 BusFault), 25 followed by END_JOB \| part1.log: 25 attempts (23 with an address), 23 fault events (21 MemManage, 2 BusFault), 23 followed by END_JOB \| part2.sampler: 25 attempts (23 with an address), 23 fault events (21 MemManage, 2 BusFault), 23 followed by END_JOB \| fault record: every one of the 71 attempts has its fault with the attempted address and class (IACCVIOL for fetches, DACCVIOL with MMFAR for data, a BusFault for the PPB) |
| walkers' counters (every attempt made, none returned) | pass | control 25/25 attempts, survived 0; log 25/25, survived 0; sampler's counters are since part2's restart at 65 ms and judged by its events |
| EasyDMA cases (R17.7, R17.1, R17.2) | pass | 5 refused requests (stack buffer, another Partition's RAM, Kernel RAM, a buffer past the RAM's end, an absent channel; 5 expected), 5 ended the Job; 2 accepted (2 expected); the buffer at 0x20019034 inside part2's RAM above its stack; SPIM0 after the run: RXD.PTR 536973364 RXD.MAXCNT 16 TXD.PTR 0 TXD.MAXCNT 0 RXD.LIST 0 TXD.LIST 0 ENABLE 0 STARTED 0 END 0 RXD.AMOUNT 0 (the accepted request's pointer and count, nothing else written, no transfer started); sampler at case 35 (at least 32 expected: every case and attempt made, then idle Jobs) |
| Partition restart stops its EasyDMA channels first (R17.6) | pass | part2 restarted at 73 ms; actions in order: stop 0x40003014, stop 0x40003014, reinit part2; the stop task (0x40003014, SPIM0.TASKS_STOP) precedes the re-initialization |
| Kernel events | pass | 157 events mirrored (157 held, 0 of them the post-detach reboot's): memmanage of Task 0: 23, memmanage of Task 3: 21, memmanage of Task 6: 21, busfault of Task 0: 2, busfault of Task 3: 2, busfault of Task 6: 2, overrun of Task 5: 1, operating_end of Task none: 1, dma_rejected of Task 6: 5, partition_restart of Task none: 1, safe_state of Task none: 1, end_job of Task 0: 25, end_job of Task 3: 23, end_job of Task 6: 28, debugger of Task none: 1; the trace never froze during the run |
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

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 6.592 | memmanage | 0 | 0 | 0x0 |
| 2 | 6.592 | end_job | 0 | 0 | 0x2 |
| 3 | 12.996 | memmanage | 0 | 0 | 0x0 |
| 4 | 12.996 | end_job | 0 | 0 | 0x2 |
| 5 | 12.996 | memmanage | 3 | 1 | 0x0 |
| 6 | 12.996 | end_job | 3 | 1 | 0x2 |
| 7 | 20.009 | memmanage | 0 | 0 | 0x0 |
| 8 | 20.009 | end_job | 0 | 0 | 0x2 |
| 9 | 31.294 | memmanage | 0 | 0 | 0x0 |
| 10 | 31.294 | end_job | 0 | 0 | 0x2 |
| 11 | 40.009 | memmanage | 0 | 0 | 0x0 |
| 12 | 40.009 | end_job | 0 | 0 | 0x2 |
| 13 | 40.009 | memmanage | 3 | 1 | 0x0 |
| 14 | 40.009 | end_job | 3 | 1 | 0x2 |
| 15 | 51.281 | memmanage | 0 | 0 | 0x0 |
| 16 | 51.281 | end_job | 0 | 0 | 0x2 |
| 17 | 60.012 | memmanage | 0 | 0 | 0x0 |
| 18 | 60.012 | end_job | 0 | 0 | 0x2 |
| 19 | 71.280 | memmanage | 0 | 0 | 0x0 |
| 20 | 71.280 | end_job | 0 | 0 | 0x2 |
| 21 | 73.211 | overrun | 5 | 2 | 0xea89c |
| 22 | 73.211 | partition_restart | - | 2 | 0x6 |
| 23 | 80.009 | memmanage | 0 | 0 | 0x0 |
| 24 | 80.009 | end_job | 0 | 0 | 0x2 |
| 25 | 80.009 | memmanage | 3 | 1 | 0x0 |
| 26 | 80.009 | end_job | 3 | 1 | 0x2 |
| 27 | 91.279 | memmanage | 0 | 0 | 0x0 |
| 28 | 91.279 | end_job | 0 | 0 | 0x2 |
| 29 | 100.009 | memmanage | 0 | 0 | 0x0 |
| 30 | 100.009 | end_job | 0 | 0 | 0x2 |
| 31 | 111.279 | memmanage | 0 | 0 | 0x0 |
| 32 | 111.279 | end_job | 0 | 0 | 0x2 |
| 33 | 120.009 | memmanage | 0 | 0 | 0x0 |
| 34 | 120.009 | end_job | 0 | 0 | 0x2 |
| 35 | 120.009 | memmanage | 3 | 1 | 0x0 |
| 36 | 120.009 | end_job | 3 | 1 | 0x2 |
| 37 | 131.292 | memmanage | 0 | 0 | 0x0 |
| 38 | 131.292 | end_job | 0 | 0 | 0x2 |
| 39 | 140.009 | memmanage | 0 | 0 | 0x0 |
| 40 | 140.009 | end_job | 0 | 0 | 0x2 |
| 41 | 151.278 | memmanage | 0 | 0 | 0x0 |
| 42 | 151.278 | end_job | 0 | 0 | 0x2 |
| 43 | 160.009 | memmanage | 0 | 0 | 0x0 |
| 44 | 160.009 | end_job | 0 | 0 | 0x2 |
| 45 | 160.009 | memmanage | 3 | 1 | 0x0 |
| 46 | 160.009 | end_job | 3 | 1 | 0x2 |
| 47 | 171.272 | memmanage | 0 | 0 | 0x0 |
| 48 | 171.272 | end_job | 0 | 0 | 0x2 |
| 49 | 180.009 | memmanage | 0 | 0 | 0x0 |
| 50 | 180.009 | end_job | 0 | 0 | 0x2 |
| 51 | 190.410 | memmanage | 0 | 0 | 0x0 |
| 52 | 190.410 | end_job | 0 | 0 | 0x2 |
| 53 | 200.009 | busfault | 0 | 0 | 0x0 |
| 54 | 200.009 | end_job | 0 | 0 | 0x3 |
| 55 | 200.009 | memmanage | 3 | 1 | 0x0 |
| 56 | 200.009 | end_job | 3 | 1 | 0x2 |
| 57 | 202.877 | dma_rejected | 6 | 2 | 0x0 |
| 58 | 202.877 | end_job | 6 | 2 | 0xd |
| 59 | 211.271 | busfault | 0 | 0 | 0x0 |
| 60 | 211.271 | end_job | 0 | 0 | 0x3 |
| 61 | 220.009 | memmanage | 0 | 0 | 0x0 |
| 62 | 220.009 | end_job | 0 | 0 | 0x2 |
| 63 | 231.284 | memmanage | 0 | 0 | 0x0 |
| 64 | 231.284 | end_job | 0 | 0 | 0x2 |
| 65 | 240.009 | memmanage | 0 | 0 | 0x0 |
| 66 | 240.009 | end_job | 0 | 0 | 0x2 |
| 67 | 302.975 | dma_rejected | 6 | 2 | 0x0 |
| 68 | 302.975 | end_job | 6 | 2 | 0xd |
| 69 | 320.269 | memmanage | 3 | 1 | 0x0 |
| 70 | 320.269 | end_job | 3 | 1 | 0x2 |
| 71 | 360.218 | memmanage | 3 | 1 | 0x0 |
| 72 | 360.218 | end_job | 3 | 1 | 0x2 |
| 73 | 400.009 | memmanage | 3 | 1 | 0x0 |
| 74 | 400.009 | end_job | 3 | 1 | 0x2 |
| 75 | 402.966 | dma_rejected | 6 | 2 | 0x0 |
| 76 | 402.966 | end_job | 6 | 2 | 0xd |
| 77 | 440.225 | memmanage | 3 | 1 | 0x0 |
| 78 | 440.225 | end_job | 3 | 1 | 0x2 |
| 79 | 480.218 | memmanage | 3 | 1 | 0x0 |
| 80 | 480.218 | end_job | 3 | 1 | 0x2 |
| 81 | 502.947 | dma_rejected | 6 | 2 | 0x0 |
| 82 | 502.947 | end_job | 6 | 2 | 0xd |
| 83 | 520.009 | memmanage | 3 | 1 | 0x0 |
| 84 | 520.009 | end_job | 3 | 1 | 0x2 |
| 85 | 560.222 | memmanage | 3 | 1 | 0x0 |
| 86 | 560.222 | end_job | 3 | 1 | 0x2 |
| 87 | 601.418 | memmanage | 3 | 1 | 0x0 |
| 88 | 601.418 | end_job | 3 | 1 | 0x2 |
| 89 | 603.160 | dma_rejected | 6 | 2 | 0x0 |
| 90 | 603.160 | end_job | 6 | 2 | 0xd |
| 91 | 640.009 | memmanage | 3 | 1 | 0x0 |
| 92 | 640.009 | end_job | 3 | 1 | 0x2 |
| 93 | 680.222 | memmanage | 3 | 1 | 0x0 |
| 94 | 680.222 | end_job | 3 | 1 | 0x2 |
| 95 | 720.218 | memmanage | 3 | 1 | 0x0 |
| 96 | 720.218 | end_job | 3 | 1 | 0x2 |
| 97 | 760.009 | memmanage | 3 | 1 | 0x0 |
| 98 | 760.009 | end_job | 3 | 1 | 0x2 |
| 99 | 802.406 | busfault | 3 | 1 | 0x0 |
| 100 | 802.406 | end_job | 3 | 1 | 0x3 |
| 101 | 803.183 | memmanage | 6 | 2 | 0x0 |
| 102 | 803.183 | end_job | 6 | 2 | 0x2 |
| 103 | 840.218 | busfault | 3 | 1 | 0x0 |
| 104 | 840.218 | end_job | 3 | 1 | 0x3 |
| 105 | 880.009 | memmanage | 3 | 1 | 0x0 |
| 106 | 880.009 | end_job | 3 | 1 | 0x2 |
| 107 | 902.975 | memmanage | 6 | 2 | 0x0 |
| 108 | 902.975 | end_job | 6 | 2 | 0x2 |
| 109 | 920.269 | memmanage | 3 | 1 | 0x0 |
| 110 | 920.269 | end_job | 3 | 1 | 0x2 |
| 111 | 960.218 | memmanage | 3 | 1 | 0x0 |
| 112 | 960.218 | end_job | 3 | 1 | 0x2 |
| 113 | 1011.451 | memmanage | 6 | 2 | 0x0 |
| 114 | 1011.451 | end_job | 6 | 2 | 0x2 |
| 115 | 1102.895 | memmanage | 6 | 2 | 0x0 |
| 116 | 1102.895 | end_job | 6 | 2 | 0x2 |
| 117 | 1211.289 | memmanage | 6 | 2 | 0x0 |
| 118 | 1211.289 | end_job | 6 | 2 | 0x2 |
| 119 | 1302.842 | memmanage | 6 | 2 | 0x0 |
| 120 | 1302.842 | end_job | 6 | 2 | 0x2 |
| 121 | 1611.515 | memmanage | 6 | 2 | 0x0 |
| 122 | 1611.515 | end_job | 6 | 2 | 0x2 |
| 123 | 1703.061 | memmanage | 6 | 2 | 0x0 |
| 124 | 1703.061 | end_job | 6 | 2 | 0x2 |
| 125 | 1811.609 | memmanage | 6 | 2 | 0x0 |
| 126 | 1811.609 | end_job | 6 | 2 | 0x2 |
| 127 | 1903.008 | memmanage | 6 | 2 | 0x0 |
| 128 | 1903.008 | end_job | 6 | 2 | 0x2 |
| 129 | 2012.002 | memmanage | 6 | 2 | 0x0 |
| 130 | 2012.002 | end_job | 6 | 2 | 0x2 |
| 131 | 2102.975 | memmanage | 6 | 2 | 0x0 |
| 132 | 2102.975 | end_job | 6 | 2 | 0x2 |
| 133 | 2211.452 | memmanage | 6 | 2 | 0x0 |
| 134 | 2211.452 | end_job | 6 | 2 | 0x2 |
| 135 | 2302.894 | memmanage | 6 | 2 | 0x0 |
| 136 | 2302.894 | end_job | 6 | 2 | 0x2 |
| 137 | 2411.294 | memmanage | 6 | 2 | 0x0 |
| 138 | 2411.294 | end_job | 6 | 2 | 0x2 |
| 139 | 2502.897 | memmanage | 6 | 2 | 0x0 |
| 140 | 2502.897 | end_job | 6 | 2 | 0x2 |
| 141 | 2611.443 | memmanage | 6 | 2 | 0x0 |
| 142 | 2611.443 | end_job | 6 | 2 | 0x2 |
| 143 | 2702.975 | memmanage | 6 | 2 | 0x0 |
| 144 | 2702.975 | end_job | 6 | 2 | 0x2 |
| 145 | 2811.447 | busfault | 6 | 2 | 0x0 |
| 146 | 2811.447 | end_job | 6 | 2 | 0x3 |
| 147 | 2902.894 | busfault | 6 | 2 | 0x0 |
| 148 | 2902.894 | end_job | 6 | 2 | 0x3 |
| 149 | 3011.291 | memmanage | 6 | 2 | 0x0 |
| 150 | 3011.291 | end_job | 6 | 2 | 0x2 |
| 151 | 3102.842 | memmanage | 6 | 2 | 0x0 |
| 152 | 3102.842 | end_job | 6 | 2 | 0x2 |
| 153 | 3211.496 | memmanage | 6 | 2 | 0x0 |
| 154 | 3211.496 | end_job | 6 | 2 | 0x2 |
| 155 | 3600.000 | operating_end | - | - | 0x0 |
| 156 | 3600.000 | safe_state | - | - | 0x0 |

## The fault record

```
71 faults recorded (71 held)
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
log=67
pulse=198
echo=111
blocked_releases=167
filter=141
filter_state_bits=1120403452
sampler=7
dma_ok=2
control_matrix=25
control_survived=0
log_matrix=25
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
first_memmanage_mmfar=229632
events=32
event=2
  task=6
  at_ms=1811
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=1811
  detail=2
  prs_lo=0
  prs_hi=0
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
  at_ms=2012
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2012
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2102
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2102
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2211
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2211
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2302
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2302
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2411
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2411
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2502
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2502
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2611
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2611
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=2702
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=2702
  detail=2
  prs_lo=0
  prs_hi=0
event=3
  task=6
  at_ms=2811
  detail=0
  prs_lo=4194304
  prs_hi=0
event=19
  task=6
  at_ms=2811
  detail=3
  prs_lo=0
  prs_hi=0
event=3
  task=6
  at_ms=2902
  detail=0
  prs_lo=4194304
  prs_hi=0
event=19
  task=6
  at_ms=2902
  detail=3
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=3011
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=3011
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=3102
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=3102
  detail=2
  prs_lo=0
  prs_hi=0
event=2
  task=6
  at_ms=3211
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=6
  at_ms=3211
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
     Finished in 8.39s
```

## Branch coverage of the Task entries under the suite (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 87 % (7 of 8 branch outcomes), part0.echo 75 % (3 of 4 branch outcomes), part0.sensor 75 % (6 of 8 branch outcomes), part1.log 100 % (14 of 14 branch outcomes), part1.pulse 100 % (8 of 8 branch outcomes), part2.filter 72 % (16 of 22 branch outcomes), part2.sampler 66 % (12 of 18 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 7 | 8 | 7 | 87 % | pass |
| `part0.control#cs:state` | 7 | 8 | 7 | 87 % | pass |
| `part0.control#cs:in_cs` | 7 | 8 | 7 | 87 % | pass |
| `part0.echo` | 5 | 4 | 3 | 75 % | pass |
| `part0.sensor` | 8 | 8 | 6 | 75 % | pass |
| `part0.sensor#cs:state` | 8 | 8 | 6 | 75 % | pass |
| `part0.sensor#cs:in_cs` | 8 | 8 | 6 | 75 % | pass |
| `part1.log` | 8 | 14 | 14 | 100 % | pass |
| `part1.pulse` | 8 | 8 | 8 | 100 % | pass |
| `part2.filter` | 19 | 22 | 16 | 72 % | pass |
| `part2.filter#cs:stats` | 19 | 22 | 16 | 72 % | pass |
| `part2.sampler` | 17 | 18 | 12 | 66 % | pass |
| `part2.sampler#cs:stats` | 17 | 18 | 12 | 66 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `p1_demo_part2::address_of` | 0 | 0 |  |
| `p1_demo_part2::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part2::isolation_step` | 8 | 5 | 187:20 true, 194:16 false, 223:16 false |
| `p1_demo_part2::isolation_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::isolation_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::isolation_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::isolation_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::Stats::record` | 2 | 1 | 253:12 true |
| `p1_demo_part2::filter` | 6 | 5 | 290:12 true |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::inject::window` | 2 | 2 |  |
| `p1_demo_part2::inject::spin_ms` | 4 | 3 | 38:84 false |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 2 | 1 | 407:8 true |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::filter` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::sampler` | 0 | 0 |  |
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
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 237:80 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control` | 4 | 4 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::echo` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::sensor` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control` | 0 | 0 |  |
