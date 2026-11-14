# IronOxide audit report: tag `ironoxide-eval-v2`

Audited tree: `/Users/mars2292/repos/Kiro-experimentation-evaltag/IronOxide` (HEAD 423a366). The pass was read-only; nothing was built, proved or run on hardware.

Path shorthand, all under that root:
- `K/` = `crates/rsk-kernel/src/arch/cm4/`
- `L/` = `crates/rsk-logic/src/logic/`
- `plan` = `docs/verification_plan.md`
- `design` = `.kiro/specs/rsk-ravenscar-profile/design.md`

For the nine contested findings I read the cited code and documents myself. I also re-read the code behind confirmed findings 1 to 8 and 11. The other rows rest on the two verifiers' agreeing reads.

## 1. Verdict

Yes: this revision should not be frozen as the one the paper evaluates until four items are fixed or the paper's claims are narrowed.

- **Findings 1 and 2 break the isolation claim from safe Partition code.** A level-B Partition can aim SPIM0's DMA at Kernel RAM, and a level-C Partition can restart or safe-state the level-A Partition with one public `rsk` call. Both work in the evaluated demo configurations with a one-line change to a Partition body.
- **Finding 3 is a wrong register write on every boot of every measured nRF52840 binary.** The Kernel enables all 32 PPI channels where the design says it clears them.
- **Finding 5 makes the event log undercount deadline misses.** The committed hour-long timing record already shows it: 144 release overlaps against 136 logged misses for `sensor`.

Findings 4 and 6 are rare timing defects in the stub and entry path, the area that produced the two earlier defects. Fixing them changes that path, so the hour-long campaigns should follow the fix.

Per both verifiers, none of findings 1 to 4 invalidates the timing or WCET numbers already measured, because the shipped demo bodies never perform the offending calls. Every fix changes the Kernel image, and most touch Verus-verified code. The cheapest order is fix, re-verify, re-tag, then measure.

The lead in section 2.1 should be examined before the freeze: it was not put through refutation, and it is the only item that may be reachable in the stress configuration without a rare window.

The rest are claim and evidence gaps. They need text changes or added contracts, not a new binary.

## 2. Confirmed findings, ranked

Reports of the same defect from several audit areas are merged. "Verified code" means code inside `verus!` (rsk-logic, `K/entry.rs`, `K/plan.rs`, `arch/bound.rs`), where the proofs must be re-run.

### Behavioural defects

| # | Finding | Location | Scenario | Consequence | Fix or disclosure | Touches verified code |
|---|---|---|---|---|---|---|
| 1 | EasyDMA windows are mapped read-write to the owning Partition, so DMA mediation can be bypassed | `L/mpu.rs:714`, `:755`; `crates/rsk-gen/src/emit.rs:144-151`; `K/unsafe_module/hw.rs:771`; `crates/rsk/src/lib.rs:597-600` | A part2 Job uses safe `rsk::mmio::write` to set SPIM0 RXD.PTR (0x40003534) to Kernel or part0 RAM, plus MAXCNT, PSEL.SCK, ENABLE and START; no fault is raised and `dma_request_core` is never consulted. | R17.1, R16.1 and DD-08 do not hold against a misbehaving owner; `summary.txt` still prints the window as "read-only". The physical DMA write was not demonstrated. | Carry a read-only bit through `PartitionLayout`, the LAYOUT image and CRC, the Generator and confcheck; add an owner-on-own-window HIL attempt. Otherwise disclose that R17 rests on Partition good behaviour. | Yes (mpu.rs view and lemmas, layout CRC); new tag |
| 2 | Any Endpoint role can impose a peer Partition's fault response with a fabricated rejection report | `K/entry.rs:1252-1267`; `L/kernel.rs:2894-2916`; `crates/rsk/src/lib.rs:465-468` | `part1.log` (level C) calls `rsk::report_rejected_message(0, 1, 0)`; `violation_core` checks only ranges, `from != role` and the caller's role binding, then calls `fault_of` on `part0.control`. | part0 (level A) is restarted, or the Application halts in `nrf52840_response` where part0 declares SAFE_STATE; repeatable every Job. Contradicts R18.7 and R42.4 (not R14.3). Form 2 of the SVC is not a defect: it is the recorded disposition of review finding 14(f). | Add a per-role validator or direction bit to `EndpointCfg` and a "delivered from `from`" latch, with API misuse otherwise. The Kernel cannot check the validator's verdict itself. | Yes (entry.rs, plan.rs, Generator); new tag |
| 3 | `ppi_disable_all` writes PPI.CHENSET, enabling all 32 channels at boot | `K/unsafe_module/hw.rs:66`, `:1069-1072`; same file `:1001` uses 0x504 as CHENSET; `K/unsafe_module/kani.rs:418` | `boot_core` → `dma_boot` → `PpiDisableAll` writes 0xFFFF_FFFF to 0x4001_F504. The local PAC gives CHENSET 0x504 and CHENCLR 0x508. | DD-08 and plan line 625 ("PPI channels are cleared at boot") are false on every nRF52840 binary. In the demo each TIMER0 COMPARE[0] triggers RADIO TXEN/RXEN, with no scheduling effect. A Partition owning RTC0 could clear the Kernel's TIMER0; that needs a non-demo configuration and LFCLK reaching RTC0, which is unconfirmed. The fixed-channel table is from the verifiers' memory of the product specification. | Set the constant to 0x4001_F508 and read CHEN back in a check. | No Verus proof; the Kani case asserts the same constant; image changes, new tag |
| 4 | The stale-SVC guard replays an already-served SVC on a resumed Job parked at its own SVC return | `K/unsafe_module/mod.rs:966-978`; `K/mod.rs:609-638`; `K/unsafe_module/stubs.rs:153-170`, `189-224` | Job L's SVC return tail-chains into a higher Job H; H is killed inside its own SVC-entry window (the interleaving at plan line 723); the still-pending SVCall lands on L's frame, PC-2 is L's own `svc`, and the service runs again with r0 = the previous result. | Send/recv: `EV_API_MISUSE` fault response on L's Partition. Lock: panic "lock refused", or a silent extra lock. Nested unlock: early release (R8.3). `now`: harmless, which masks it under stress. Contradicts R18.7; the "fixed" disposition is incomplete. In the demo part0 cannot be the victim except in the timing configurations. Rate is low and unmeasured. The BL-halfword remark in one report is wrong. | Clear SHCSR.SVCALLPENDED when a Job is ended, or record per level that an SVC is expected. | Depends: a shell or stub fix does not; a new `Action` in the kill paths does (entry.rs, `HwModel` step) |
| 5 | A held Release's deadline event is disarmed when the previous Job ends | `L/kernel.rs:1786` (`job_return`), `:3007-3011` (`end_top_release`), `:2595-2604` | Under RECORD_ONLY with D = T: Job k runs past the next instant, Release k+1 is held and the single deadline slot is re-armed; Job k's end disarms it; Job k+1 misses with nothing armed. | R9.6 misses of Jobs started from held Releases are not logged. `docs/hil/release_timing_nrf52840_timing.md:16` shows 144 overlaps against 136 misses; `coverage_nrf52840_timing_short.md:16` shows 18 against 16 with a pass. The contracts (`job_return` ensures 1747-1750) encode the loss. Only the miss record is lost; a zero-miss log is still sound. | Keep the slot when a Release is pending, or re-arm at dispatch from `job_due`. Until then do not report miss counts from the log after a chain's first miss. | Yes (rsk-logic contracts change) |
| 6 | `pending_unwind` is a single slot: a second Job-ending Kernel entry at the same stub-entry window overwrites the first owed unwind | `K/entry.rs:1823-1827` (store), `1681-1686` (consume); `K/plan.rs:150-153` | A Kernel-level entry preempts dispatch stub A at its entry instruction and ends J, storing level(J); a second level-0 entry tail-chains before A's `cpsid i`, ends J' (likeliest via the Budget slot `end_top` just armed) and overwrites the slot. | A unwinds level(J') over J's record: the resumed context gets another Job's R4-R11, one record stays on the MSP, and every later unwind is one off (R7.5, Property 2). Rare: the window was hit four times per hour under stress, and this needs a second ending event on top. | Queue owed unwinds, or chain the second into the first. | Yes (entry.rs, plan.rs) |
| 7 | An interrupt-bound Sporadic_Task whose Job is ended by END_JOB stays masked | `L/kernel.rs:2936-2955`, `2977-3038`, contrast `1790-1792`; `K/entry.rs:655-658` | `pulse` (GPIOTE, `overrun = END_JOB`) overruns once; `end_top_release` leaves `timing` unchanged, never calls `end_mit_window` and arms no unmask slot. | The source stays disabled until the Partition is next restarted. Contradicts design:585 ("next Release follows the normal schedule") and DD-06 step 4. `response_applied` requires `timing == old.timing`, so the proof mandates it. Latent: no evaluated run overruns `pulse`. | Call the MIT-window end on the END_JOB path and weaken the timing clause. | Yes (rsk-logic) |
| 8 | Kernel entry trusts an unvalidated PSP: the dispatch path writes a new Job's frame below it (see also contested A and D) | `K/entry.rs:1733-1737`, `1755`, `1774`; `K/plan.rs:195-218`; `K/unsafe_module/mod.rs:942-954` (UJ-007) | A Job's SP sits at the base of its own read-write peripheral window when a higher same-Partition Task is dispatched; `floor = psp` has no upper bound, and `plan_frame` proves only `bottom <= frame` and `frame + 32 <= floor`. | A privileged 32-byte write just below the window, outside every interval the Partition owns. The "inside the Partition's stack region" text in plan.rs:6-10 and UJ-007 is unsupported. Needs an SP that PR-20, PR-22 and the Link_Checker exclude statically, which ORQ-28 calls defence in depth. Hardware effect unverified. | Check `stack_bottom <= psp <= stack_top` at entry from a Job and attribute a violation to the Partition. | Yes (entry.rs) |
| 9 | A stacking fault on a level-0 entry need not reach MemManage first | `K/unsafe_module/hw.rs:687-695`; `K/unsafe_module/stubs.rs:155-160`; `K/entry.rs:1869-1883` | A Job issues `svc` with PSP 8 to 24 bytes above its stack bottom; MemManage and SVCall share priority 0, so the Armv7-M derived-exception rule pends MemManage and enters the SVC handler on an unstacked frame. | The Kernel writes result words below the Partition's stack (top of Kernel main stack for part0); with `SVC_JOB_END` the pended fault lands on the resumed context. The P-21 "Backstop proven" claim is unsupported for this mode. Silicon behaviour on the tie is unmeasured, and QEMU would hide it. | A differential test (svc with PSP at bottom+16) decides it; fix 8's bound check covers the SVC path too. Until then record it as an open assumption of HW-14 and R16.5. | Yes if fixed in the cores |
| 10 | The EasyDMA class table is wrong for UARTE, SAADC, PWM and RADIO, and the byte length is written as the count regardless of unit | `crates/rsk-gen/src/target.rs:39`, `41`, `53`, `55`; `K/entry.rs:883-898`, `917-960` | SAADC MAXCNT counts 16-bit samples (twice the validated bytes); RADIO has no count register; UARTE and SAADC stop offsets are wrong; PWM SEQ[1] is unmediated. | R17.1, R17.2 and R17.6 fail for any Application owning those peripherals, which the Generator accepts. Latent: every evaluated configuration owns SPIM0 only, and that entry is correct. | Set `dma: None` for the four classes so the existing DD-08 rule rejects them, or narrow the claim to SPIM. A real fix adds a unit to `DmaCfg`. | Rejecting: no (Generator only). Unit field: yes |
| 11 | END_JOB reached through an Endpoint send's or signal raise's MIT violation restarts the Partition | `L/kernel.rs:2936-2959`, `2198-2238`; `K/entry.rs:1528-1531`, `988` | `mit_violation` passes the released Task as `t`; `apply_response` tests `top == t`, the top is the sender, so control falls through to `restart_partition`. | RESTART where END_JOB was declared and the offending Job holds nothing; diverges from DD-10 and the code's own comment. Contained and conservative; no evaluated configuration reaches it. The claimed effect on the Analyzer's Δ_restart is wrong. | Record the escalation in DD-10, or pass the offending Task. | Doc-only, or yes (rsk-logic) |

### Unsupported claims and proof gaps (no run-time defect)

| # | Finding | Location | What is wrong | Fix or disclosure | Touches verified code |
|---|---|---|---|---|---|
| 12 | Facts A3, A4 and A6 are recorded as proven, but no Verus postcondition states them | `K/entry.rs:298`, `622-631`, `713-717`, `1643-1645`, `1991-1995`; `arch/bound.rs:380-381`; plan:140-143; design:741 | `HwModel.pending`, `priority` and the loaded view's value appear in no contract. By reading (not run), deleting the view restore at `entry.rs:2006-2012` would still verify. TBR-24 lists neither A4 nor A6 as by inspection. | Reword to "by inspection and test", or add postconditions with a mutant each. The paper should not call the pended vector, priorities or loaded view machine-checked. | Text: no. Contracts: proof-only |
| 13 | Endpoint copy confinement is called "both proven" but is in no contract, mutation or negative test | `K/entry.rs:1276-1309`, `335-344`, `1349-1377`; plan:243 | `endpoint_args` ensures nothing about `ptr`, `len`, the caller or `message_permitted`; `copy_in`/`copy_out` have no `requires`. The checks are correct at the tag. | Add the ensures and requires, three mutations and a foreign-pointer case; or reword 5.10. | Proof-only |
| 14 | A-P6's lemma for stop/restart re-initialization does not exist, and the cores' frame conditions are unstated | `K/entry.rs:1031-1094`, `685-692`, `1222-1268`, `1466-1586`; plan:317, :117 | Only `endpoint_reinit_one` has a postcondition. `endpoint_reinit`, `clear_channels` and `violation_core`'s calls are verified for bounds only. HIL coverage shows 0 of 16 branches of `endpoint_reinit`. The code is correct on reading. | Add postconditions, or list the stop/restart half of A-P6 as a standing assumption with test evidence. | Proof-only |
| 15 | Section 4 rows A4, A5 and A8 and the Hardware_Model's FAULTMASK text are stale after the 5.8a fix; "with PRIMASK held" is wrong for the Endpoint SVCs | plan:141, :142, :145, :241; `crates/rsk-logic/src/hw_model.rs:65-76`, `487-489`; `K/unsafe_module/stubs.rs:110`, `125`, `139`, `215`; `K/entry.rs:1668`, `1684`, `1706`, `1386-1387`; UJ-055 | The stubs execute `cpsid f` on every return; `irq_entry_core` disables and re-pends dispatch vectors; `rsk_svc_entry` has no `cpsid i`. No behavioural defect. The premise now rests on "exception return clears FAULTMASK", which has no Hardware_Model rule or differential test (R49.6). P-17 rests on A5 and A8; P-04 does not. | Restate A4, A5 and A8; correct the hw_model list; replace "PRIMASK held" by "at Kernel level 0"; optionally add the rule and a test. | No, unless the rule is added |
| 16 | Documents claim no, or five, assumption constructs; the tag has eight | plan:560, :20, :134; `docs/trust_base_register.toml` TBR-04, TBR-24; `docs/proof_assumptions.toml:19-33`; `K/entry.rs:422-542`; `arch/bound.rs:499` | Seven `external_body` items in `Cm4Hw` plus `corrupt_config_for_test`; `copy_in` and `copy_out` carry real contracts. All are in the register, so the R50.2 gate passes. | Update the counts and lists. | No |
| 17 | `docs/kani_stubs.toml` and the plan's Kani tables omit doubles and bounds that exist at the tag | `docs/kani_stubs.toml`; `K/unsafe_module/sim.rs:74`, `116-138`; `K/unsafe_module/stubs_host.rs:22-54`; plan 6.2, 6.3, row P-13 | `sim::ram_read`/`ram_write`, `partition_reinit` and six host SVC doubles are unlisted; there is no 6.3 row for the copy harness (len <= 32); P-13 says 5 kani / 38 review where the register has 8 / 46. Also (contested H): `copy_in_words`/`copy_out_words` have no harness and `Action::Copy` is applied by no core. | Update the register and plan; add a harness on the two wrappers. | No |
| 18 | `hard_fault` reads HFSR at an address outside the set UJ-001 says every `mmio_read` satisfies | `K/mod.rs:825-828`; `K/unsafe_module/hw.rs:71-76`, `100-125` | 0xE000_ED2C is not in `addr::ALL`; no harness reaches `hard_fault`. No hardware consequence. | Add HFSR to `addr::ALL`, or gate the read. | No |

### 2.1 Unconfirmed lead (raised by both verifiers of finding 6; not put through refutation)

One `timer_event` can end two successive tops through `end_top`, and the arch layer then unwinds only one level, with no entry window needed.

- `end_plan_core` (`K/entry.rs:1610-1628`) names only the level of `top_before`.
- `epilogue_core` (`K/entry.rs:1988-2029`) chains only when the resumed top `is_killed`.
- `end_top` arms the resumed Job's Budget slot at `now` when nothing remains (`L/kernel.rs:1716-1720`).
- `timer_event`'s second loop re-reads each slot, so a slot armed at a higher index is served in the same entry.

In `gen-nrf52840_stress`, `log` has Budget slot 11 and preempts `sampler`, whose Budget slot is 20; both Partitions declare END_JOB. I read these four functions and the structure is as described. Nobody traced it to a hardware outcome. It deserves its own pass before the freeze.

## 3. Contested findings and rulings

| | Finding | Ruling | Who was right, and why |
|---|---|---|---|
| A | `rsk_memmanage_entry` dereferences PSP+24 on a stacking fault; UJ-026 is false | **Upheld as claim-unsupported, low.** Same root cause as finding 8. | Verifier 1 on the claim, verifier 2 on reach. `K/unsafe_module/stubs.rs:263-267` loads `[PSP+24]` unconditionally, and the comment at 248-253 is circular: MSTKERR is delivered to this handler. UJ-026 was reviewed 2026-10-08; the read arrived on 2026-10-29 (commit 7785d00). The HardFault outcome needs PSP+24 at an unimplemented address, which no Link_Checker-accepted build produces. That is the precondition both verifiers still accepted for findings 8 and 9. Skipping the load when CFSR.MSTKERR is set removes the path. |
| B | The safe-state idle loop relies on WFI returning inside an active Kernel-level handler | **Upheld as claim-unsupported, low.** Not hardware-observed. | Verifier 1. The shells call `S::safe_state()` inside the handler (`K/mod.rs:533`, `599`, `831`, `852`), and the loop is `watchdog_feed(); wfi` (`K/unsafe_module/hw.rs:707-716`, `1375-1377`). No configurable exception outranks level 0, so the architecture gives no wake event. The comment at hw.rs:1356-1363 and plan:680 ("settles in the idle loop with the watchdog fed") are therefore unsupported for run-time entries. Verifier 2 is right that no requirement is violated, the Target still ends in the safe state, and no measurement is affected; the expected cost is one watchdog reset and the loss of the run's event log. |
| C | "With PRIMASK held" is false for the Endpoint SVCs | **Upheld as fact, informational; folded into finding 15.** | Both agree on the facts; verifier 2's classification (wording, not a defect) is right. `rsk_svc_entry` has no `cpsid i`, and atomicity follows from level 0 alone. The finding's hazard clause is dropped: it would matter only if SVCall itself left priority 0. |
| D | `svc_number` dereferences a PC read back from a frame in a device window | **Upheld at the level of finding 8 (medium, conditional), as the same family; the "defect-safety" label is too strong.** | Verifier 2. `K/mod.rs:612` calls `svc_number(frame[6])` on the raw PSP with no check between the stub and the privileged read, so UJ-008's "hardware fact" holds only for a frame in RAM. Verifier 1's refutation rests on PR-20, PR-22 and R37.3, the same static checks both verifiers weighed for finding 8 and did not treat as closing it. `rsk::mmio::write` is a safe function taking any address (`crates/rsk/src/lib.rs:597-600`). Verifier 1 is right that no SP-relocation chain was built and that the fault at 0xFFFFFFFE is untested. |
| E | R8.4's run-level theorems rest on a premise false on correct runs | **Not a proof gap; documentation correction.** | Verifier 2 on classification. In `L/history.rs:456-461` and `474-479`, `d` is a free index with no constraint on `h[d]`, so the lemmas instantiate at first eligibility + 1 on every run and cover the whole blocked interval. Verifier 1 is right that plan:175 and the comment at history.rs:445-455 call `d` "its dispatch" and equate the premise with A4, which is inaccurate. The window from first eligibility to dispatch rests on the hardware argument; plan:175 says so, and the verifiers report GAP-14 as open. |
| F | R18.1 is listed as complete, but whole Kernel entries for other Tasks are charged to the interrupted Job | **Not a new defect (recorded as Δ_entry); the recorded description is wrong and must be corrected.** | Verifier 2 on status: design:440 defines Δ_entry, and plan:291 records the observed false Overrun of `sampler` and its disposition. Verifier 1 on substance: `timer_core` charges only at entry (`K/entry.rs:1845`) and the intake arm has no charge point (`1791-1799`), so the charge is the handler's Rust half per interrupting entry. The verifiers read the JSON records as about 2,000 cycles mean and up to 4,824 for a timer entry. Plan:822 says "some hundreds of cycles per entry" and "per-preemption". The paper should state R18.1 "within Δ_entry per interrupting Kernel entry", and Δ_entry as a mean. |
| G | Releases whose deadline falls beyond the operating duration are suppressed | **Not a defect; informational.** | Verifier 2. The guard is deliberate (`L/kernel.rs:2591-2594`) and design:150-152 records it as the reading of R9.9. The residue is that the texts of R9.1 and R10.1 do not list the exception. For the evaluation: the last Release inside the final D window is not made (per verifier 1, `sensor` at 185 ms in the 200 ms demo), which matters if the paper reports Release counts. |
| H | The Endpoint copy path reaches the UJ-056 read without `copy_words`' checks; storage alignment is not guaranteed | **Refuted as a defect; the evidence gap is folded into finding 17.** | Verifier 2, consistent with the refuted alignment finding in section 5. `apps/p1-demo/gen-nrf52840/link.x:127-134` opens `.kernel.bss` with `ALIGN(4)`, and the verifiers report `ENDPOINT_STORAGE` as its only input in every ELF they inspected. Verifier 1's residue is accurate: `copy_in_words` and `copy_out_words` (`K/unsafe_module/hw.rs:500-511`) have no harness, and `Action::Copy` appears only in `step`, `apply` and the Kani case. |
| I | Two mutation kills are recorded only as solver timeouts | **Not a defect of the tag; already recorded, with one correction.** | Verifier 2. `docs/paper/evaluation_results.md` on branch `paper-eval` already says the history mutants are rejected by resource-limit exhaustion and that this is weaker evidence. That file counts three. The logs under `target/xtask-mutations/` show two: `lock-by-other-than-top` and `dispatch-below-ceiling` have only the rlimit error, while `mit-run-spacing-dropped` also has two "postcondition not satisfied" errors. |

## 4. What was checked and found clean, per area

### Stubs (`K/unsafe_module/stubs.rs`, shells in `K/mod.rs`, cores in `K/entry.rs`, `crates/rsk-entry/src/lib.rs`)

Preemption windows, as enumerated by the auditor:

| Window | Location | Protected by | Status |
|---|---|---|---|
| W1: boundary before `cpsid i` in `rsk_irq_entry` | stubs.rs:82-83 | Nothing masks. A higher dispatch vector defers itself (entry.rs:1704-1725) and is re-enabled and re-pended by the preempted stub (1656-1673); a level-0 entry returns unchanged or owes the unwind (1823-1827, consumed 1681-1686). | Consistent except finding 6 |
| W2: pushes, conditional vpush, Rust half | stubs.rs:83-109 | PRIMASK; only HardFault or NMI can enter, and they never return. MSP stays 8-byte aligned. | Clean |
| W3: dispatch return | stubs.rs:106-113 | View loaded while privileged; `cpsid f` before `cpsie i`; the return clears FAULTMASK before unstacking. | Clean (documents stale, finding 15) |
| W4: no-change return | stubs.rs:116-127 | vpop keyed on the callee-saved EXC_RETURN; fence. | Clean |
| W5: hand-off to `rsk_unwind` | stubs.rs:128-130 | PRIMASK held, own pushes popped. | Clean except finding 6 |
| W6: SVC frame selection | stubs.rs:155-160 | Level 0, so no configurable preemptor. | Which frame it is: findings 4 and 9, contested D |
| W7: SVC return or unwind hand-off | stubs.rs:161-166 | Level 0; `dsb` before `bx`. | Clean |
| W8: `rsk_unwind` entry | stubs.rs:192 | Entered from level-0 handlers, or with PRIMASK held. | Clean |
| W9: epilogue and pops | stubs.rs:193-209 | PRIMASK; the epilogue loads view and BASEPRI before any return. | Clean |
| W10: fenced unwind return | stubs.rs:210-217 | `cpsid f; cpsie i; bx`; one exception active, so no INVPC. | Clean |
| W11: killed-Job chain | stubs.rs:219-221 | PRIMASK; the pop decision uses each level's EXC_RETURN. | Clean (see lead 2.1) |
| Fault stubs | stubs.rs:256-321 | Level 0; `in_kernel` from EXC_RETURN bit 3 or PRIMASK. | Frame read at 263-267: contested A |

Also clean:
- Every conditional branch targets a local label in the same function.
- All immediates and IT blocks are encodable.
- Every `bx` is preceded by `dsb` (erratum 838869).
- FP handling (ASPEN=1, LSPEN=0) never corrupts a Job's S0-S31.
- Stack floors, frame alignment and xPSR are consistent.
- PRIMASK is clear on every return to Thread mode (A2).
- The vector table and reset path are correct.
- `rsk_timer_entry` is bound to no vector (dead at this tag).

Not examined: time-base arming, `mpu_load` internals, Endpoint logic, the HIL-only `dt.rs` hooks.

### Unsafe hardware layer

Clean:
- PPB addresses and NVIC index arithmetic.
- AIRCR, SHPR, SHCSR, CCR, FPCCR and CPACR writes.
- BASEPRI encodings.
- MPU RBAR and RASR field placement and the `mpu_load` sequence.
- TIMER0 offsets, the arm sequence and the 64-bit extension (no wrap race).
- WDT offsets, key and reload.
- Boot-check register fields.
- DWT charge arithmetic.
- `copy_words`, `message_permitted_exec` and `dma_permitted_exec` (u64 arithmetic, no overflow).
- `partition_reinit` against the Generator's link script.

The Kani double accepts any aligned peripheral address, so it cannot detect a wrong register offset; this is why findings 3 and 10 pass Kani.

Not established and not reported as findings: whether DWT_CYCCNT counts through WFI without a debugger (the recorded assumption ASM-06); registers are not scrubbed between Partitions (confidentiality is the non-goal NG-11).

### Endpoint architecture code (all 1,236 added lines of the diff from 2bc1eb8)

Clean by inspection:
- User pointer validation in `endpoint_args`.
- Identifier checks before indexing.
- Channel and slot index arithmetic.
- Fifo push, pop and wrap.
- Loop bounds.
- Budget accounting of the SVC.
- Stop and restart ordering.
- The result register on every path.
- A receive into the caller's own SVC frame (the outcome stays with the caller).
- Event-log flooding (covered by R14.7).

### Endpoint logic (rsk-logic diff, 210 lines)

Clean:
- No assumption construct in the new code.
- `Fifo::push`, `pop`, `clear` and `is_wf` are tight.
- `fault_of` is tight on which Partition and response.
- `message_permitted` covers the whole RAM interval, and the SRD-disabled subregions hold nothing.
- Storage disjointness is consistent between Generator and confcheck.
- Every new precondition is established at its call site.

`Kernel::inv` is not extended over Endpoint state (recorded in plan 5.10). The register has four Endpoint mutations, all in rsk-kernel. One minor item was dropped by the auditor.

### Specification adequacy

Judged to imply the English at the logic level: R7.1, R7.3, R8.3, R8.5, R9.3, R10.1 and R10.2 (spacing), R14.9 (other Partitions unchanged), R16.1 to R16.3 for unprivileged accesses, R18.2 (the level part and once-per-Job report), and R19.1 under A13.

Weaker than the English, with the item that covers each:
- R7.2: selection only; recorded.
- R8.4: contested E.
- R9.1: contested G and finding 5.
- R10.1 acceptance: finding 7.
- R16.5: findings 8 and 9.
- R18.1: contested F.

One minor item was dropped by the auditor.

### Trust base and fragility

- Eight `external_body` items, all registered.
- No `assume`, `admit`, `assume_specification` or `#[kani::stub]`.
- 109 `unsafe` occurrences, every UJ identifier resolving to a register entry.
- All three history mutants make their lemmas false (hand-checked counterexamples), so the kill verdicts are correct.
- Kani harness sizes equal the demo's generated configuration.
- All 21 `kani::assume` sites were reviewed.
- The `apply` harness covers all 33 `Action` variants.

One minor item was dropped by the auditor.

## 5. Refuted findings (do not re-raise)

- **"Nothing establishes that the Endpoint storage is word-aligned; a misaligned base makes the copy do nothing while the send returns OK"**: refuted by both verifiers. The generated `.kernel.bss` is `ALIGN(4)` with `ENDPOINT_STORAGE` as its only input, and payloads and offsets are multiples of 4, so the trigger cannot occur in the tagged tree.
- **"The Kani boot-check harness assumes away the clock path every cold boot takes"**: refuted by both verifiers. The bound is stated in the harness and recorded where R45.8 requires it, and no claim takes Kani credit for the excluded path.
