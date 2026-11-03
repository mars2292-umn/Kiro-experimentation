# Hardware_Model differential tests on the nRF52840-DK (task 3.3, R49.3)

Verdict: **pass** (14 rules matched, 0 mismatched, 3 with untested parts)

Target 1366:1060:001050217737; binary sha256:17bc07eae6d6936e26b488f809baf7477a75f54d7a3154b292425041b4931d8b. The record `DT_RECORD` is written by the Kernel's two boot phases (feature `hw-dt`); the Task-side rows come from the isolation configuration's probes and the access matrix.

## Per rule

| Rule | Behaviour | Verdict |
|---|---|---|
| HW-01 | priority encoding, 3 implemented bits, PRIGROUP | match |
| HW-02 | execution priority | match |
| HW-03 | BASEPRI and BASEPRI_MAX | match |
| HW-04 | PRIMASK; CPS and MSR only when privileged | match |
| HW-05 | NVIC arbitration among pending exceptions | match |
| HW-06 | exception entry stacks R0-R3, R12, LR, PC, xPSR on the selected stack | match |
| HW-07 | exception return deactivates; NONBASETHRDENA | match |
| HW-08 | Handler mode is privileged; Thread privilege is CONTROL.nPRIV | match |
| HW-09 | MPU region match, overlap precedence, background map, subregions | match |
| HW-10 | eager FP stacking and FPDSCR defaults | match |
| HW-11 | precise BusFaults with DISDEFWBUF | match |
| HW-12 | TIMER compare semantics and the near-counter rule | match |
| HW-13 | DWT CYCCNT | match |
| HW-14 | tail-chaining and late arrival | match |

## Observations

| Rule | Test | Predicted (Hardware_Model) | Observed (Target) | Result |
|---|---|---|---|---|
| HW-01 | IPR read-back after writing 0xFF | 0xe0 | 0xe0 | match |
| HW-01 | AIRCR.PRIGROUP at reset, then after the Kernel's boot (every implemented bit a group bit) | 0, then 4 | 0, then 4 | match |
| HW-01 | IPR read-back of encode_level(1..7) | 0xe0 0xc0 0xa0 0x80 0x60 0x40 0x20 | 0xe0 0xc0 0xa0 0x80 0x60 0x40 0x20 | match |
| HW-02 | with BASEPRI 0x80: B (0x60) taken, A (0xA0) held pending (ISPR bit 30 set, bit 31 clear) | log 1 entry; ISPR bit 30 only | log 1 entries; ISPR 0x40000000 | match |
| HW-03 | BASEPRI after MSR 0x80, MSR BASEPRI_MAX 0xA0, MSR BASEPRI_MAX 0x60, MSR 0 | 0x80 0x80 0x60 0x0 | 0x80 0x80 0x60 0x0 | match |
| HW-03 | BASEPRI 0 lets the held A through | log 2 entries | log 2 entries | match |
| HW-04 | PRIMASK read-back after CPSID I; both pended exceptions held (ISPR bits 30 and 31, log empty) | 1; ISPR bits 30 and 31; log 0 | 1; ISPR 0xc0000000; log 0 | match |
| HW-04 | CPSID I from unprivileged Thread mode ignored: PRIMASK still clear at the SVC that follows (an SVC under PRIMASK would escalate to HardFault), and A, held by BASEPRI, not yet taken | PRIMASK 0; log 8 entries at SVC 15 | PRIMASK 0; log 8 entries at SVC 15 | match |
| HW-04 | MSR PRIMASK/BASEPRI from unprivileged Thread mode ignored | no effect | not tested on silicon: the Link_Checker excludes msr/mrs/cps from every Task's code (PR-22, R16.10), so no Job can execute them; the CPSIE I case above is the one the Kernel_Proofs rely on (`lemma_job_cannot_set_mask`) | not tested (disposition in the observed column) |
| HW-05 | handler order: test 1 (B more urgent) B A; test 2 (A 0xC0 pended first, B 0x40) B A; test 3 (equal, B pended first) A B (lowest number); test 4 A A; unprivileged test A | [47, 46, 47, 46, 46, 47, 46, 46, 46] | [47, 46, 47, 46, 46, 47, 46, 46, 46] | match |
| HW-06 | SVC frame R0-R3, R12 hold the patterns loaded before the SVC | 0x11111111 0x22222222 0x33333333 0x44444444 0xcccccccc | 0x11111111 0x22222222 0x33333333 0x44444444 0xcccccccc | match |
| HW-06 | stacked PC is the instruction after the SVC; xPSR.T set; frame 8-byte aligned (MSP) | pc 0xc88; T; aligned | pc 0xc88; xpsr 0x21000000; frame at 0x20007ef8 | match |
| HW-06 | a Job's R2-R5, R8, R10, R12 preserved across preemptions (Property 2, R7.5): the FPU Task's pattern probe | 0 failures | 141 runs, 0 failures, mask 0x0 | match |
| HW-07 | after the handlers returned: IABR clear, ICSR.VECTACTIVE 0 | 0; 0 | 0x0; 0 | match |
| HW-07 | CCR.NONBASETHRDENA after the Kernel's boot | 0 | 0 | match |
| HW-08 | the handler of A read CONTROL.nPRIV set (Thread mode unprivileged), read ICSR with VECTACTIVE 46 (privileged in Handler mode), and Thread mode read CONTROL 0 afterwards | nPRIV 1; VECTACTIVE 46; CONTROL 0 | CONTROL 0x1; VECTACTIVE 46; CONTROL 0x0 | match |
| HW-08 | a Job's access to the Private Peripheral Bus faults (unprivileged Thread mode) | 6 BusFaults | 6 BusFaults | match |
| HW-09 | the access matrix (R16.8): every access outside the Partition's view refused | every attempt a MemManage fault of the Partition | part0.control: 23 attempts (23 with an address), 23 fault events (21 MemManage, 2 BusFault), 23 followed by END_JOB \| part1.log: 23 attempts (21 with an address), 21 fault events (19 MemManage, 2 BusFault), 21 followed by END_JOB \| part2.sampler: 23 attempts (21 with an address), 21 fault events (19 MemManage, 2 BusFault), 21 followed by END_JOB \| fault record: every one of the 65 attempts has its fault with the attempted address and class (IACCVIOL for fetches, DACCVIOL with MMFAR for data, a BusFault for the PPB) | match |
| HW-09 | a write to the word below the Partition's stack refused (R16.5) | MemManage | MemManage | match |
| HW-09 | MPU_CTRL after the Kernel's boot: ENABLE and PRIVDEFENA (background map for privileged code only) | 0x5 | 0x5 | match |
| HW-09 | subregion-disable bits and overlap precedence | a disabled subregion refuses; the highest-numbered region decides | not exercised: no view of this Application sets SRD bits (every region is a power-of-two region of exactly its size) and no view has two enabled regions covering one address; the encoding is checked at field level by the Kani harnesses `rasr_encoding_fields` and `rbar_encoding_fields` | not tested (disposition in the observed column) |
| HW-10 | FPCCR.ASPEN set, FPCCR.LSPEN clear (eager stacking) | ASPEN 1, LSPEN 0 | FPCCR 0x80000000 | match |
| HW-10 | FPSCR of a new FP context equals FPDSCR | 0x0 | 0x0 | match |
| HW-10 | S0-S31 preserved across preemptions (the hardware's S0-S15 under eager stacking, the stubs' S16-S31) | 0 failures | 141 runs, 0 failures, mask 0x0 | match |
| HW-11 | ACTLR.DISDEFWBUF after the Kernel's boot | 1 | 1 | match |
| HW-11 | every BusFault of the PPB attempts precise, with BFAR valid | 6 precise | 6 faults: PRECISERR+BFARVALID at 0xe000e100, PRECISERR+BFARVALID at 0xe000e100, PRECISERR+BFARVALID at 0xe000e100, PRECISERR+BFARVALID at 0xe000e100, PRECISERR+BFARVALID at 0xe000e100, PRECISERR+BFARVALID at 0xe000e100 | match |
| HW-12 | COMPARE fires for every distance of MIN_COMPARE_DISTANCE (16, revised from 2 by this test's first run) ticks or more from the counter captured before the write; below it the event is not guaranteed (ASM-19) | fired for every d >= 16; the model's former 2 refuted (0 to 7 never fire: the capture-to-write path is about 7 ticks) | smallest distance that fired: 8 ticks; d=0: no event after 12555 ticks; d=1: no event after 12426 ticks; d=2: no event after 12426 ticks; d=3: no event after 12554 ticks; d=4: no event after 12426 ticks; d=5: no event after 12426 ticks; d=6: no event after 12426 ticks; d=7: no event after 12554 ticks; d=8: fired after 14 ticks; d=10: fired after 17 ticks; d=12: fired after 20 ticks; d=16: fired after 23 ticks | match |
| HW-13 | CYCCNT counts up between two readings | second > first by a few cycles | 0x8 then 0xc | match |
| HW-13 | CYCCNT wraps at 2^32: a few dozen cycles after writing 0xFFFF_FFF0, two readings have wrapped past zero, count up, and lie within the model's `elapsed_cycles` of the written value | both below 0x2000; second > first; elapsed from 0xFFFF_FFF0 below 0x2000 | 0x3a then 0x3e (elapsed from the write 78) | match |
| HW-14 | tail-chaining: fewer cycles from one handler's exit to the next's entry when the next was already pending than when it is pended after the return | tail-chained gap below the separate gap | tail-chained 150 cycles, separate 230 cycles | match |
| HW-14 | late arrival (a higher-priority exception replacing the one being entered) | the stub sees the context actually preempted | not driven deliberately: the timing window is a few cycles wide and no probe can place an exception in it; observed in the field by the response suite of task 7.5 (a Job's SVC overtaken by the time base, `SVC_STALE`), where the stub's EXC_RETURN and frame described the context the hardware had preempted, as HW-14 states | not tested (disposition in the observed column) |

## The handler log

| # | Vector | CYCCNT at entry | at exit | CONTROL |
|---|---|---|---|---|
| 0 | 47 | 252 | 313 | 0x0 |
| 1 | 46 | 547 | 608 | 0x0 |
| 2 | 47 | 943 | 1004 | 0x0 |
| 3 | 46 | 1154 | 1215 | 0x0 |
| 4 | 46 | 1528 | 1589 | 0x0 |
| 5 | 47 | 1739 | 1800 | 0x0 |
| 6 | 46 | 2047 | 2108 | 0x0 |
| 7 | 46 | 2338 | 2399 | 0x0 |
| 8 | 46 | 2847 | 2922 | 0x1 |

## The record

```
  0: 52534b44 00000001 00000003 000000e0 00000000 000000e0 000000c0 000000a0
  8: 00000080 00000060 00000040 00000020 00000080 00000080 00000060 00000000
 16: 00000001 c0000000 00000000 40000000 00000001 00000002 00000000 00000000
 24: 00000008 00000001 0000082e 00000000 11111111 22222222 33333333 44444444
 32: cccccccc e000e41e 00000c88 21000000 20007ef8 00000c88 00000008 0000000c
 40: 0000003a 0000003e 00000096 000000e6 00000009 0000002f 000000fc 00000139
 48: 00000000 0000002e 00000223 00000260 00000000 0000002f 000003af 000003ec
 56: 00000000 0000002e 00000482 000004bf 00000000 0000002e 000005f8 00000635
 64: 00000000 0000002f 000006cb 00000708 00000000 0000002e 000007ff 0000083c
 72: 00000000 0000002e 00000922 0000095f 00000000 0000002e 00000b1f 00000b6a
 80: 00000001 00000000 00000000 00000000 00000000 00000000 00000000 00000000
 88: 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000
 96: 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000
104: 00000000 00000000 00000000 00000000 00000000 00000000 00000000 0000310b
112: 00000000 0000308a 00000000 0000308a 00000000 0000310a 00000000 0000308a
120: 00000000 0000308a 00000000 0000308a 00000000 0000310a 00000001 0000000e
128: 00000001 00000011 00000001 00000014 00000001 00000017 80000000 00000000
136: 00000002 00000200 00000005 00000004 00000000 00000000 00000000 00000000
144: 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000
152: 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000
```
