# Proof coverage: Verus, Lean, both, neither

Written 2026-11-25. Example applications are left out.

This is a read-only survey; no prover was run for it. It is taken from the project's own evidence register (`docs/requirements_evidence.toml`), the verification plan, and the header and theorem list of every hand-written Lean file. Where a Lean counterpart could not be confirmed from those, the item is under Verus only.

The two provers look at different things. Verus proves the Rust source against a model. Lean proves the same functions as extracted from the compiled crate (Aeneas, ghost code erased). The hand-written Lean files listed contain no `sorry` and no `axiom`.

The columns "can it be proven on the other platform" and "would it be useful" are judgments made for this survey from the same sources, not results: nothing in them was attempted. "Useful" weighs what the second proof would add against what already stands in for it.

## Proven in both Verus and Lean

| Component | Property |
|---|---|
| Scheduler (`rsk-logic`, `Sched`) | Start condition: the Task dispatched is the one `select` returns and is eligible |
| | LIFO resumption, the scheduler's half |
| | Mutual exclusion and the ceiling protocol (lock, unlock, forced end of a Job) |
| | Single-section blocking: at most one lower Critical_Section per Release |
| Time base (`time.rs`) | Periodic release at offset plus k periods, no drift; the timer entry serves only armed, due slots |
| Sporadic intake | Accept only with no Release held and one MIT since the last; arrivals at least one MIT apart over a run |
| Budget accounting | Overrun detected whatever the ceiling; time is charged only to the executing Job; an armed Budget slot is the top Job's |
| Slots, Endpoint FIFO, event log | Each refines its abstract model |
| Memory layout and MPU views (`mpu.rs`) | `Layout::validate` true implies well-formed; Partition views share nothing writable; no view grants Kernel memory or a Kernel peripheral; a read-only window is not writable; a stack overflow faults |
| Buffer checks | `dma_permitted` and `message_permitted` answer exactly their definitions; an EasyDMA request is granted only for the owner's permitted buffer |
| Boot validation | An accepted `Config` is well-formed |
| Kernel logic, the `Bound` wrapper, the entry cores | No overflow, no index out of bounds, no panic (Lean: every entry returns and keeps the invariant) |
| Entry cores (`arch/cm4/entry.rs`) | The per-level records agree with the Kernel's preemption stack, through dispatch, unwind, supervisor call, interrupt, fault and boot |
| Endpoint entry cores | An accepted argument names a real Endpoint, role and permitted buffer |
| Assembly stubs, as a machine model | A resumed Job gets back R4–R11, S16–S31, its stack pointer, PSP and CONTROL |
| Analyzer (`rta.rs`, `terms.rs`) | The response time is the least fixed point of the recurrence; the blocking, overhead and interference terms are as specified; the verdict follows |
| Generator arithmetic (`rsk-gen` `derive_v`) | Priority byte, ceilings, region size, time scaling, timer-slot plan |
| Config_Checker core (`rsk-confcheck` `derive_v`) | Level encoding, ceilings, checked sums, timer slots, alignment, disjointness |
| Protocol_Checker (`rsk-protocol`) | A multiparty Protocol the checker accepts has no reachable deadlocked configuration |

## Proven in Lean only

| Component | Property | Can Verus prove it? | Would it be useful? |
|---|---|---|---|
| Kernel and Analyzer together | Property 18: a Task the Analyzer reports schedulable with bound R has every Job finished within R, over the run of the Kernel's extracted functions, also cut into Kernel entries | In principle, over the ghost history. In practice hard: the argument is an induction over a run with a busy-window count, and it joins two crates that Verus verifies apart | No. The Lean proof is already on the compiled code of both crates; a second proof on the source adds little |
| Run of the Kernel | Every entry of the linked Kernel leaves it "quieted"; a pending Kernel-level event waits at most one handler with its unwind | Yes for "quieted", as a postcondition of each entry core. The hold is a statement about a run and has the difficulty of the row above | Little. Both serve Property 18 only |
| Deadline slots | An armed deadline slot belongs to a Job pending or in progress, as an invariant over the run (Verus has it only as two postconditions) | Yes: a clause of `Kernel::inv`, as its Budget-slot twin `budget_slots_owned` already is | Yes, and small. A defect was found here on 2026-11-17; as a clause of the invariant it would be checked at every edit of the source, where the Lean proof is checked only when the translation is rebuilt |
| Register drivers (`arch/cm4/drivers.rs`) | What each driver does to a register file (86 theorems), and that each returns | Yes, with a specification of the register trait inside Verus; the drivers are already generic over it | Moderately. It would join the drivers to the Verus hardware model: today the step from an `Action`'s specification to the driver that performs it is by reading |
| Generated table accessors | Each returns for every argument, in each of the 14 configurations | Only if the generated code were put under Verus, which it is not | No |
| `Bound::init`, ghost helpers | They return once validation passed | Not applicable: the ghost helpers are Verus's own constructs, and Verus already checks `init`'s arithmetic | No |

## Proven in Verus only

| Component | Property | Can Lean prove it? | Would it be useful? |
|---|---|---|---|
| Scheduler, over the ghost history | Deadlock freedom; ceiling restored across preemptions | Yes: the run over the Kernel's functions exists, with exclusion (`Sched/Mutex.lean`) and the scheduler's half of resumption (`Sched/Lifo.lean`) already on it | Moderately, and cheap: two of the design's numbered properties would then hold of the compiled code as well |
| Interrupt sources | At most one handler run per MIT per source (the intake count bound); every outcome masks the source | Yes: the sporadic timing fields and their spacing are already in the Lean run (`Sched/Sporadic.lean`); the mask is one more field | Yes. It is what bounds an interrupt source's interference, which Property 18 takes as a parameter of the run |
| Health_Monitor | Each response has its declared effect on the faulting Partition and leaves the others alone; a fault is logged first, against the right Task and Partition | Yes: the responses are extracted and Lean already proves that they return | Yes, the most useful of this table. It is the fault-containment claim, and on the compiled code Lean so far shows only that a response returns, not what it does |
| Hardware model (`hw_model`) | The interrupt controller takes a dispatch vector exactly when the Task is eligible; a Kernel-level handler is never preempted | Yes: the model is pure specification and would be restated in Lean | Little by itself. Useful only together with the row below |
| Entry cores over the hardware model | BASEPRI matches the state after every entry; vector priorities set at boot; PRIMASK untouched by a supervisor call; only the time base feeds the watchdog; floating-point access goes to the right Job | With work. The ghost hardware model is erased by the extraction, so the statements have to be restated over the register model the drivers are proven against (`Hw/Model.lean`) | Yes, high value and large. BASEPRI would become a statement about register writes on the compiled code, not about a ghost field |
| Kernel RAM | No word changes between two entries; the Endpoint storage persists | Yes, as a restatement of a fact of the model | Little |
| Endpoints | Whole-message send and receive under the storage-placement premise; re-initialisation and the peer-state report | In part. The index state and the arguments are already in Lean; the copy is raw memory, which the extraction does not translate | Moderately, for the re-initialisation and the peer-state report; the copy is better left to Kani |
| Dispatch planning (`plan.rs`) | A new Job's frame is aligned and inside its stack; the timer compare value | Yes: pure arithmetic | A little: the frame's placement is part of the stack-overflow argument |
| Time base | Wrap extension of the counter | Yes: the reading is already in `Sched/Time.lean` | Moderately: Property 18's instants are the extended ones |
| Checksums | The Kernel's and the Config_Checker's CRC-32 compute the stated function | Yes: Lean already proves the Kernel's checksum returns | Moderately. One Lean definition for both would replace "the two restated definitions are the same by reading" |
| Config_Checker (`image_v`) | The image decoders; the storage-placement check means what it says | Yes, if those functions are added to the translation (only `derive_v` is in it) | Little |
| Generator (`accept_v`) | No configuration is produced outside the numeric acceptance conditions | Yes, if added to the translation | Little: the Kernel's validators and the Config_Checker re-check the output |
| Analyzer | `check_inputs` rejects exactly inadmissible inputs; Endpoint sites and their cost | Yes: the function is small and pure; it is not in the translation today | Yes, and small. Property 18's hypotheses about the inputs would be discharged by the code instead of matched to the Verus predicate by reading |
| Link_Checker (`cert.rs`) | The stack-walk result is checked by a proven certificate on every run | Yes | Little |
| Protocol_Checker | Projection and role operations; the two-party progress certificate; the protocol-text lexer | Yes: projection is already defined in `Proto/Spec.lean` | Little to moderate: the multiparty theorem already covers the two-party case on the compiled checker |

## Proven by neither

| Component | What is not proven | What stands in | Provable by | Would it be useful? |
|---|---|---|---|---|
| Trusted hardware functions (`Cm4Hw::apply` and the volatile accesses) | That they write what the model says | Kani on encoders, review, board tests | Neither, for the volatile access itself: it is an axiom about memory in any prover. The code between `apply` and the drivers could be proven in either, given the register model | For the part that is provable, moderately. It shrinks the trusted list from nine functions toward the single volatile read and write |
| Assembly stubs | The assembly itself | The model above; text compared with the binary's disassembly | Lean, with a formal semantics of the Armv7-M instructions used (116 instructions in seven stubs, by the plan's count). Not Verus | Yes, high value and the largest item here: the stubs are on the path of every isolation and scheduling claim |
| `rsk-entry`, the shells in `arch/cm4/mod.rs` | Everything | Inspection | Verus for the order of the boot (validate, init, then start) and the shells' few lines. Lean only if statics and function pointers are kept out of what is translated | Moderately: the claims are short and the code is small |
| Entry code facts | Only the selected vector is pended; the loaded MPU view is the right one; no reentry of the Kernel; PRIMASK and FAULTMASK use | Inspection and rig tests | Verus for the first two: the pending set and the loaded view are in the model and appear in no postcondition. Reentry and the mask flags inside the stubs need the stubs' semantics (row above) | Yes for the first two, and modest work. The plan records that a core which omitted the view's restore would still verify |
| Hardware model | That the chip behaves as the model | Citations, one differential test per rule | Neither, by nature | Not applicable |
| MPU register encoding | — | Kani (bit-precise) | Both (bit-vector reasoning) | No: the Kani harness covers every input |
| `rsk` (the Partition API) | Its contracts are trusted by the App_Prover | Kani on EasyDMA lending and result codes | Verus for the wrappers against the Kernel's own contracts; the supervisor call between them stays trusted. Lean for the safe parts | Yes. Every application proof rests on these contracts and nothing ties them to what the Kernel is proven to do |
| Generator front end | Parsing, validation rules beyond the numeric ones, emission, macros | Tests; read-back of emitted tables | Verus or Lean for the remaining validation rules (ownership of peripherals and interrupts). Emission of text is not practical in either | Little to moderate: a wrong output is caught downstream; a wrongly accepted declaration is what a proof would add |
| Config_Checker front end, `rsk-elf` | Task_Model reader, table builder, ELF lookup | Tests | Both: parsers over bytes with bounds | Moderately. This is the independent check, and a wrong symbol lookup would make it accept a wrong binary |
| Analyzer front end | Input parsing and validation, role resolution, the report | Tests, golden files | Both for validation and role resolution; not the report's text | Little to moderate; role resolution is the part that feeds a proven term |
| Link_Checker | Instruction decoding and call-graph construction | Tests, stack painting on the rig | Lean or Verus, with the same instruction semantics the stubs need, here for stack-pointer effects only | Moderately to highly, but large. The stack bound is a safety argument and the tool has no second tool to compare with |
| Profile lint (`rsk-lint`) | Completeness of each rule | Conformance suite | Neither in practice: it needs a semantics of Rust source | No |
| Protocol | Label naming, the emitted role module's text, four liveness and tool assumptions | Tests, stated assumptions | The liveness of a Task body is the App_Prover's, per application. The emitted text: neither | Little |
| Timing | Every Kernel overhead, execution time and jitter | Measured only; static bounds deferred by decision | Neither: it needs a static execution-time analyser with a timing model of the processor | Yes in principle (it is why several evidence rows read "weaker"), but outside what these provers do |
| Lean run vs. the real Kernel | That every execution is a run of the Lean model; that the Verus input predicate equals the Lean hypotheses | Read function by function | Lean for both: compose each entry core's writes into the run's transitions; translate `check_inputs` | Yes, high. The first is the remaining distance between Property 18 and the code; the second is small |
| `rsk-hil`, `rsk-wcet-agent`, `rsk-model`, `rsk-loom`, `rsk-digest`, `xtask` | Everything | Test tooling | In parts, by either | No: none of it runs in the product, and its results are labelled as testing |
| Below the source | The compiler, the provers' soundness | Assumed | Neither here | Not applicable |
| Jorvik variant, multi-core, qualified compiler | Not attempted | By decision | Not applicable | Not applicable |
