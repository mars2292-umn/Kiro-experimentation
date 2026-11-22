# Audit of the Task limit

**PACEMAKER on rsk, task 2.1 (requirement PM32.2)**

Date: 2026-11-22. Read-only: no rsk file was changed. Source revision: `a8ea7620` with the uncommitted pacemaker documents.

## Result

Seven Tasks is a choice of the Target table, not a limit of the Kernel. The `nrf52840` has interrupt lines that could be added as dispatch vectors, and every fixed size the number of Tasks reaches in the Kernel holds at eight Tasks without change. Above eight, one fixed size (the dispatch-deferral record) has to grow with its proof. Raising the limit at all costs re-measurement and re-proof at the larger configuration (PM32.6), which is why the Application is designed to seven (PM32.1).

The Application of `design.md` has six Tasks. Nothing in this audit is needed unless task 22.7 wants more than the seventh.

## 1. Interrupt lines that could serve as dispatch vectors

The table is from the chip's register description (`nrf52840.svd` of the `nrf-pac` checkout on this machine): 48 lines, 0 to 47.

**In use as dispatch vectors today** (rsk-gen `target.rs`, `dispatch_irqs`): 20, 21, 22, 23, 24, 25 (the software interrupts, shared with EGU0 to EGU5) and 40 (UARTE1).

**Lines with no interrupt source at all:**

| Line | Note |
|---|---|
| 30 | Taken: the HIL differential tests use it as a probe vector (`unsafe_module/dt.rs`, `VECTOR_A`) |
| 31 | Taken: the second probe vector (`VECTOR_B`) |
| 43 | Free |
| 44 | Free |
| 46 | Free |

Lines 30 and 31 are free in a Flight build; the `hw-dt` feature that uses them exists in HIL builds only. Using them as dispatch vectors would need the probes moved.

**Lines whose only source is a peripheral neither rsk's Target table nor this Application uses.** A line is usable as a dispatch vector when no Partition can enable its source, which holds for every peripheral that is not in the Target table: its registers are in no Partition's window.

| Line | Source | Line | Source |
|---|---|---|---|
| 5 | NFCT | 32 | MWU |
| 12 | TEMP | 33 | PWM1 |
| 13 | RNG | 34 | PWM2 |
| 14 | ECB | 35 | SPIM2 |
| 15 | AAR, CCM | 36 | RTC2 |
| 18 | QDEC | 37 | I2S |
| 19 | COMP, LPCOMP | 39 | USBD |
| 27 | TIMER4 | 41 | QSPI |
| 29 | PDM | 42 | CRYPTOCELL |
| 45 | PWM3 | 47 | SPIM3 |

Line 38 (FPU) is left out: its source is the core's floating-point exception flags, which a Task using the FPU can raise.

**Lines this Application needs for their own peripheral, and so not available:** 4 (SPIM1), 6 (GPIOTE), 9 (TIMER1), 10 (TIMER2), 26 (TIMER3). The Kernel's own: 0 (CLOCK), 8 (TIMER0), 16 (WDT).

**The order matters (DD-03, PM32.4).** The list must stay strictly ascending, and Tasks take vectors in declaration order, so an added line changes which Task gets which vector unless it is above 40. The free lines above 40 are 43, 44 and 46 (no source) and 41, 42, 45, 47 (unused peripherals). Appending 43, 44 and 46 would give ten vectors with no existing assignment moved.

## 2. Fixed sizes the number of Tasks can reach

| Size | Where | Bounded by | At 8 Tasks | Above 8 |
|---|---|---|---|---|
| Dispatch vectors: 7 | rsk-gen `target.rs` `dispatch_irqs`; the Generator rejects a declaration with more Tasks (`validate.rs`, diagnostic DD-01) | Tasks | The list must be lengthened: this is the limit | The same |
| Dispatch-deferral record: 8 slots | rsk-kernel `plan.rs` `ArchState::deferred`; exhaustion is a Kernel fault into the safe state (`entry.rs`) | Tasks: one slot per distinct deferred vector, a vector already recorded reuses its slot | Sufficient: at most 8 distinct vectors | Must grow, with the loop bounds and invariants of the entry that fills it |
| Level records: 8 | rsk-kernel `plan.rs` `ArchState::levels` | Priority levels (0 to 7), one Job in progress per level | Unaffected | Unaffected while the levels stay 7 (PM32.7) |
| Stack floors: 16 | rsk-kernel `entry.rs` `MAX_FLOORS` | Partitions | Unaffected | Unaffected |
| Timed-event slots: at most 253, held as a `u8` | rsk-gen `derive_v::slot_plan`, `emit.rs`; rsk-confcheck `image_v.rs` | Tasks: up to 3 per Task plus 2 | 26 at most | Sufficient to 83 Tasks |
| Tasks: at most 255 | rsk-gen `validate.rs` (PR-14); rsk-logic invariants (`NT <= 255`); rsk-confcheck `image_v.rs` | Tasks | Sufficient | Sufficient |
| Tasks: at most 256 | Lean `Sched/Link.lean` (`hNT`) | Tasks | Sufficient | Sufficient |
| Endpoints: 8, roles per Endpoint: 4 | rsk-kernel `plan.rs` `MAX_ENDPOINTS`, `MAX_ROLES` | Endpoints, not Tasks | Unaffected | Unaffected |
| Kani harness sizes | The logic harnesses run at the demo's sizes; `xtask` check `kani_bounds.rs` (R45.8) fails a harness bound below the configuration's | The configuration under verification | The harnesses must be at the larger sizes | The same |
| Per-configuration table proofs | Creusot HIL members, the generated tables | The configuration | Re-proven for the configuration | The same |

Tasks sharing a Priority: with more than seven Tasks, at least two share a level. The level record is per level and holds one Job in progress, which the start rule guarantees (a Job starts only above the running Job's Priority). The order of equal-Priority Tasks is DD-03's, by vector number. No configuration measured so far has two Tasks at one Priority, so PM32.5 asks for a rig demonstration first. The P1 demo has seven Tasks on seven vectors; this audit did not check whether any two of them share a Priority.

## 3. What raising the limit would cost

From the table: one line appended to `dispatch_irqs` per added Task (43, 44, 46 first), nothing else in the Kernel up to eight Tasks, and the deferral record enlarged from the ninth. Beyond the code:

- the Kernel_Timing_Parameters, the hold, the release lag and the WCET_Records measured again, since each Task adds up to three timed-event slots and the slot scans are the largest share of a Kernel entry (PM32.6);
- the Kani harnesses and the per-configuration proofs at the larger sizes;
- the equal-Priority order shown on the rig (PM32.5).

## 4. What was not checked

- Whether a line with an unused peripheral behind it can be pended and taken exactly as a software-interrupt line. Line 40 (UARTE1) already serves as one in every measured configuration, and the probes on 30 and 31 are pended by software, so there is evidence for both kinds; lines 43, 44 and 46 themselves were not exercised.
- The QEMU Target, which lists seven of its twelve free lines.
- Sizes in the Analyzer and in the UPPAAL export.
