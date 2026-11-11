# IronOxide Tutorial

As of 2026-11-10. An earlier, shorter version is published at https://claude.ai/artifact/XczjQyadvaBaJeDg7KdgW1.

This tutorial is written for someone who knows how to program, has met threads, mutexes and interrupts at least once, and has never heard of Ravenscar, response-time analysis or the Stack Resource Policy. Part I explains what the project is for and the ideas it is built on. Part II builds six applications in order, from one periodic Task to the full demo with three memory-isolated Partitions, and runs each on an emulated Cortex-M4. Part III is a reference: the event log, the common errors, a glossary and where each claim in this document comes from.

If you only want to see something run, skip to "Before you start" in Part II. Come back to Part I when a term in a declaration or an error message does not make sense.

---

# Part I: What the project is, and the ideas behind it

## 1. The problem: software that must not be late

A microcontroller is a small computer on one chip: a processor core, some flash for the program, some RAM, and peripherals such as timers, serial ports, analogue converters and pin controllers. The Cortex-M4 that this project targets runs at tens of megahertz, has no cache and no operating system of its own, and typically has a few hundred kilobytes of memory. Such chips sit inside pacemakers, insulin pumps, car brake controllers, aircraft actuators, industrial robots and railway signalling.

In those systems a program can fail in two ways. It can compute the wrong value, which every programmer knows about. Or it can compute the right value too late. If the brake controller reads the wheel speed sensor every 10 milliseconds and decides whether to release pressure, an answer that arrives 30 milliseconds late is as bad as a wrong one, because the wheel has already locked. Software of this kind is called hard real-time: each piece of work has a deadline, and missing it is a failure, not a slowdown.

Two more things make this harder than it sounds.

**Nobody can test their way to confidence.** A timing failure can depend on the exact moment an interrupt arrives relative to the exact instruction another activity is executing. A test suite that runs for a week has seen a tiny fraction of those interleavings. Certification standards for this software, such as DO-178C for aircraft and IEC 62304 for medical devices, therefore ask for analysis: a written argument, with evidence, that the worst case still meets every deadline. The argument has to cover the worst case, not the typical one.

**One chip often carries work of different importance.** The same controller may run the brake logic and a diagnostic logger that writes to a serial port. The logger is allowed to have bugs that the brake logic is not. The standards express this with criticality levels: DO-178C's Design Assurance Levels run from A, where a failure is catastrophic, to E, where it has no safety effect. The cost of developing software grows steeply with its level, so the practical goal is to prove that the low-level code cannot disturb the high-level code, and then spend the expensive effort only on the high-level code. Showing that a less trusted piece of software cannot corrupt the memory of, or steal time from, a more trusted piece is called robust partitioning, with two halves: spatial partitioning (memory and peripherals) and temporal partitioning (processor time).

IronOxide is a kernel, a set of code generators and a verification toolchain whose purpose is to let you write this kind of software in Rust and produce the evidence that it meets its deadlines and keeps its Partitions apart. The crate prefix throughout the repository is `rsk`.

## 2. Why ordinary concurrency does not fit

A general-purpose operating system gives you threads you can create at any time, mutexes a thread can block on, condition variables, message queues of unbounded length, and timers you can set and cancel. This is convenient and it is unanalysable. The questions a safety argument needs answered have no answer in that model:

- How long can a high-priority thread wait for a mutex? As long as whoever holds it takes, plus however long any middle-priority thread chooses to run in between. This is priority inversion, and it famously stalled the Mars Pathfinder lander in 1997 until a watchdog reset it.
- Can two threads deadlock? If each can take two locks in either order, yes.
- How much memory will the queue need? Unknown, so it is allocated dynamically, so allocation can fail at run time.
- How many threads exist? Any number, so the scheduler's data structures and the worst-case schedule cannot be computed in advance.

The real-time community's answer, developed through the 1980s and 1990s, was not a cleverer scheduler. It was to restrict the programming model until the remaining programs have a mathematical model, and then to prove things about the model. Fixed-priority preemptive scheduling with ceiling-based locking is that model. Programs that fit it can be analysed with a few lines of arithmetic, and the arithmetic has been proven sound.

## 3. Ravenscar in one page

In 1997 a workshop in the Yorkshire village of Ravenscar defined a subset of the Ada language's concurrency features that fits the model above. The Ravenscar profile is a list of restrictions. Each one removes a feature that would break the analysis:

| Restriction | What it removes | What it buys |
| --- | --- | --- |
| All tasks are declared at the top level and never created or destroyed at run time | Dynamic task creation | A finite, known set of activities to analyse; no allocation |
| Each task has one fixed priority | Priority changes at run time | A fixed schedule to analyse |
| A task is a loop with exactly one point where it waits | Waiting in the middle of work, timeouts, waiting for several things at once | Each iteration runs to completion once started, which makes its execution time a single number |
| Shared data is accessed only through protected objects with ceiling locking | Mutexes a task can block on | Bounded blocking, no deadlock, no priority inversion |
| No task waits inside a protected operation | Blocking while holding a lock | The same |
| Delays are to absolute instants, never relative | "Sleep for 10 ms", which drifts | Periodic work that stays on its grid |
| Every queue has length one; no dynamic memory | Unbounded buffers | Memory known at build time |

The reward is that a Ravenscar program can be checked by response-time analysis (Section 4.3) and, with the SPARK toolset, proved free of run-time errors. Ravenscar is the standard profile for high-integrity Ada and is used in satellites, avionics and railway systems.

IronOxide carries this idea to Rust on the Cortex-M4 and adds three things Ravenscar leaves to others:

1. **Spatial partitioning.** Every Task body runs in the processor's unprivileged mode behind a Memory Protection Unit (MPU) configuration that confines it to its own Partition's memory and peripherals. A wild pointer in the logger faults the logger. Ravenscar programs are usually one privileged program.
2. **Temporal partitioning.** The Kernel measures how long each Job runs and stops or records a Job that exceeds its declared Budget, so a runaway low-level Task cannot consume the time the analysis promised to a high-level one. It also enforces a minimum spacing on interrupt-driven Tasks, so a chattering sensor line cannot flood the processor.
3. **Generated configuration and proof.** One declaration of the whole application feeds a generator that produces the scheduler tables, the memory map, the linker script and a model for the timing analyser. Independent checkers recompute the derived values from the model and compare them with the linked binary, so a generator bug cannot silently change the configuration. The Kernel's scheduling logic is proven in the Verus verifier; application Task bodies are proven panic-free in Creusot; the analyser's own arithmetic is proven to compute what the theory says.

The closest existing Rust project is RTIC, which also uses the Cortex-M interrupt controller as the scheduler and ceiling locking through priority masking. IronOxide keeps that mechanism and changes the privilege model, the Kernel's position above every ceiling, and the generated-and-checked configuration. The design document has a full comparison table of rsk, Ada Ravenscar and RTIC.

The project documents call the set of restrictions the Profile, numbered PR-01 to PR-36, and `docs/profile.md` lists each with the guarantee it supports, how it is enforced and its Ada analogue. You will meet those numbers in error messages.

## 4. The theory, in simple terms

This section is the part most tutorials skip. Every attribute you write in a declaration in Part II is a parameter of one of the models below, and the analyser's verdict is only as good as your understanding of what the numbers mean.

### 4.1 Tasks, Jobs and Releases

A **Task** is a recurring activity: "read the sensor", "update the control output", "write a log line". It has a name, a priority and a body, which is a function. A Task is not a thread. It has no stack of its own between runs, no loop you write, and no state the Kernel keeps for it beyond "when is it next due".

A **Job** is one execution of the body. When the Kernel decides a Task is due, it creates a Job, and the Job runs the body from its first statement to its `return`. The body cannot pause in the middle, wait for a message, or sleep. This is the Ravenscar "one wait point" rule: the single wait point is the return, after which the Task waits for its next Release.

A **Release** is the instant at which a Job becomes eligible to run. Where Releases come from is the Task's release kind:

- A **periodic** Task is released by the Kernel's clock at fixed instants: `offset`, `offset + period`, `offset + 2·period`, and so on. These instants are absolute. If a Job runs late, the next Release is still at its grid point, not "period after the late one finished", so a periodic Task never drifts.
- A **sporadic** Task is released by an event: a peripheral interrupt (a timer, a pin edge, a DMA completion) or a Signal that another Task raises in software. Events can arrive at any time, so for analysis the declaration promises a **minimum inter-arrival time**, the MIT: two Releases are never closer than that. The Kernel enforces the promise by deferring or discarding events that arrive too early, which is what makes it safe to analyse against.

Each Task carries three further numbers:

- The **deadline**, relative to the Release: the Job must have finished this long after its Release. It is at most the period or MIT, so at most one Job of a Task is ever in flight.
- The **Budget**: the longest the body is allowed to execute, as a time or as a cycle count. It must be at least the body's worst-case execution time (WCET), measured on the real hardware. The analyser uses the Budget as the execution time, and the Kernel enforces it, so the analysis and the run time agree.
- The **priority**, 1 to 7, 7 the most urgent, fixed at build time.

A picture of one periodic Task with period 10 ms, deadline 10 ms and a Job that runs for 3 ms:

```
time (ms)   0         10        20        30
            |---------|---------|---------|
Release     R         R         R         R
Job         ###       ###       ###       ###
Deadline              D         D         D
```

### 4.2 Fixed-priority preemptive scheduling

The rule is one sentence: at every instant the processor runs the eligible Job with the highest priority, and if a higher-priority Job is released while a lower one runs, the lower one is suspended at once and resumed when the higher one finishes. That is preemption.

Priorities are assigned by you. The usual rule of thumb, called rate-monotonic assignment, is that the Task with the shorter period gets the higher priority. It is provably optimal when deadlines equal periods and there is no sharing, and a good default otherwise. A deadline shorter than the period argues for a higher priority; a Task that holds a shared Resource for a long time argues for care, as Section 4.4 explains.

Two Jobs of equal priority never preempt each other. In IronOxide the one declared first runs first, which is a small deviation from Ada's first-in-first-out rule that the Profile document records.

On a Cortex-M4 this scheduler is not a loop in software. The chip's interrupt controller, the NVIC, already implements "run the highest-priority pending interrupt, preempting lower ones". IronOxide gives each Task its own interrupt vector at the NVIC priority of the Task, and releasing a Task means setting that interrupt's pending bit. The NVIC then does the arbitration in hardware. The Kernel's own work, such as locks, timers and fault handling, runs at one reserved level above every Task, so no Task configuration can ever mask the Kernel.

### 4.3 Response-time analysis

This is the arithmetic that answers "will every deadline be met". For a Task i with execution time C_i, the worst case is when it is released at the same instant as every higher-priority Task, because then it suffers the most preemption. Its worst-case response time R_i is the smallest value that satisfies:

```
R_i = C_i + Σ over higher-priority Tasks j of  ⌈ R_i / T_j ⌉ · C_j
```

Read the sum as: during the R_i that Task i takes, each higher-priority Task j is released ⌈R_i / T_j⌉ times and each of those Jobs runs for C_j before i can continue. The equation has R_i on both sides, so it is solved by iteration: start with R = C_i, evaluate the right-hand side, repeat until the value stops changing. If it stops at or below the deadline the Task is schedulable; if it grows past the deadline it is not.

A worked example with three Tasks, times in milliseconds:

| Task | Priority | Period T | Execution C | Deadline |
| --- | --- | --- | --- | --- |
| A | 7 | 10 | 1 | 10 |
| B | 5 | 20 | 3 | 20 |
| C | 2 | 40 | 5 | 40 |

- A has nothing above it: R_A = 1.
- B: start with 3. One A Job fits in 3 ms, so R = 3 + 1 = 4. One A Job fits in 4 ms, so R = 4 again. R_B = 4.
- C: start with 5. In 5 ms there is one A and one B: R = 5 + 1 + 3 = 9. In 9 ms there is still one A and one B: R = 9. R_C = 9.

All three meet their deadlines, with slack 9, 16 and 31 ms. Change C's execution to 12 ms and the iteration for C goes 12, 12+2+3 = 17, 12+2+3 = 17: still fine. Give B an execution of 9 ms and B's own response is 9+1 = 10, then C's goes 5, 5+1+9 = 15, 5+2+9 = 16, 16: fine. The method is conservative and quick, and it is the same calculation whether the system has three Tasks or thirty.

IronOxide's Analyzer computes exactly this, with four additions the simple formula ignores: a blocking term B_i from shared Resources (next section), a release jitter J measured on the hardware (the interrupt controller does not start a Job at the instant of its Release), the Kernel's own overheads per Job and per interrupt, and the possibility that a lower-criticality Task overruns before the Kernel stops it. The requirement R29.1 states the full equation. The Analyzer is proven to return the least fixed point, and its verdict is one of the evidence items of the verification job.

### 4.4 Sharing data: priority inversion and the Stack Resource Policy

Two Tasks of one Partition that both need the same counter or the same buffer share it through a **Resource**. A Resource is a declared piece of data with a declared set of accessing Tasks. A Job uses it inside a **Critical Section**: a closure that gets exclusive access and must finish without waiting. The question is how exclusivity is enforced.

**The naive way, and why it fails.** Give the Resource a mutex. A high-priority Job H tries to lock it while a low-priority Job L holds it, so H blocks. A medium-priority Job M, which needs nothing, is released, preempts L, and runs for as long as it likes. H, the most urgent work in the system, waits for M, which has nothing to do with it. That is unbounded priority inversion. The response-time formula has no term for it, so the analysis is wrong.

**The Stack Resource Policy (SRP), which IronOxide uses.** Every Resource gets a **Ceiling** at build time: the highest priority of any Task that accesses it. The scheduler keeps a **System Ceiling**, the highest Ceiling of any Resource currently held. The one rule is: a Job may only start if its priority is strictly above the System Ceiling. While it runs it can lock any of its Resources without ever finding one held, because any Job that could hold one of them would have raised the System Ceiling to at least this Job's priority and prevented it from starting.

Replay the scenario. L locks the Resource it shares with H. The Resource's Ceiling is H's priority, 7, so the System Ceiling becomes 7. H is released: 7 is not above 7, so H does not start. M is released: 5 is not above 7 either, so M does not start. L finishes its Critical Section in the 2 ms it was declared to take, the System Ceiling drops, and H starts at once, then M. H was delayed by exactly one Critical Section of one lower Task. The consequences, all of them proven for the Kernel in Verus rather than assumed:

- **Bounded blocking.** A Job is delayed at most once, by at most the longest Critical Section of any lower-priority Task on a Resource whose Ceiling is at or above its priority. That one number is the B_i of the response-time equation.
- **No deadlock.** A Job never waits for a lock, so there is no cycle of waiters.
- **No lock queues, no waiting at all.** `lock` never blocks. The Kernel raises the ceiling and runs the closure.
- **Jobs nest like a stack.** The running Jobs, in order of start, have strictly increasing priority, which is what lets the Kernel keep their saved registers on one stack and gives the policy its name.

In hardware the System Ceiling is the Cortex-M BASEPRI register, which masks every interrupt at or below a level. A lock is a supervisor call that raises BASEPRI to the Ceiling; unlock restores it. Because Task bodies are unprivileged they cannot touch BASEPRI themselves, which is also why they cannot subvert the policy.

The cost of SRP is that a long Critical Section in a low-priority Task delays every higher Task that shares the Resource. The demo declaration in `apps/p1-demo/decl/src/lib.rs` records one such tuning: a 6 ms section in a priority-5 Task sat right at the priority-7 Task's 10 ms deadline, and the section was shortened. The Analyzer reports which Critical Section determines each Task's blocking term, so this is visible, not a surprise.

### 4.5 Budgets and temporal partitioning

The response-time analysis assumes every Job runs for at most its C_i. A Task in a less trusted Partition may not honour that: it may have a bug, an unbounded input, or a WCET measurement that missed a path. If nothing stops it, every assumption about the Tasks it can preempt is void.

So the Kernel charges every Job's execution to its Budget with a hardware timer and applies the Partition's declared **overrun response** when the Budget runs out. The responses form a closed set:

| Response | What happens | Allowed for |
| --- | --- | --- |
| `END_JOB` | The Job is ended on the spot; the Task's next Release follows the normal schedule. If the Job held a Resource, the response escalates to a Partition restart so that no other Job sees half-updated data | faults, overruns |
| `RESTART_PARTITION` | Every Job of the Partition is ended, its RAM is re-initialised, its message roles are reset and its peers told; Releases resume at the next grid points | faults, overruns, deadline misses |
| `STOP_PARTITION` | Like a restart, then no further Release of the Partition until reset | faults, overruns, deadline misses |
| `SAFE_STATE` | The whole system enters the declared safe state | faults, overruns, deadline misses; always used for Kernel faults |
| `RECORD_ONLY` | The event is logged; the Job continues | deadline misses only |
| `RECORD_AND_CONTINUE` | The overrun is logged once; the Job completes | overruns of the highest-criticality Partition only |

The last row shows the design's attitude to criticality: the most trusted Partition is allowed to overrun because stopping it would be worse than the overrun, and the Analyzer treats its interference as its full Budget regardless. For every other Partition the Analyzer can rely on the Kernel stopping an overrunning Job, which is what makes the analysis of the trusted Partition independent of the behaviour of the untrusted ones. That independence is temporal partitioning.

The Budget must cover the WCET, and the WCET must be measured. The repository's hardware-in-the-loop rig measures every Job on the nRF52840 development board and records the longest observed execution plus a margin; the Analyzer refuses a Budget below the record. On the QEMU emulator there is no cycle counter, so Budgets are not enforced there and a Job that runs long only misses its deadline. Keep that in mind when a tutorial example "works" on QEMU.

The same mechanism handles interrupt-driven Tasks. Every sporadic Task's events are taken first by a Kernel intake handler, which timestamps the event, masks the interrupt line until one MIT after the event and the Job has ended, and only then releases the Task. A source that fires faster than its MIT therefore costs the processor at most one intake per MIT window, which is the bound the Analyzer uses. Too many deferrals in a row, more than the Partition's `mit_threshold`, is reported as a fault of the Partition.

### 4.6 Partitions and spatial partitioning

A **Partition** is the unit of isolation and of criticality. It has a level A to E, its own Rust crate, its own code and RAM regions, up to five peripheral windows, and its own fault, overrun and deadline-miss responses. Tasks belong to exactly one Partition. Resources never cross a Partition boundary.

The Cortex-M4's MPU has eight regions, each a power-of-two-sized, aligned block of addresses with access permissions. When the Kernel starts a Job it loads the Job's Partition's regions: the shared read-only code (the `core` library and the `rsk` API), the Partition's code, the Partition's RAM, and its peripheral windows. The Job runs in unprivileged Thread mode, so it cannot change the MPU, cannot mask interrupts, cannot reach the Kernel's memory or the interrupt controller, and cannot touch any other Partition. An access outside the view is a MemManage fault that the Kernel attributes to the Partition and answers with its fault response.

Two details worth knowing because they shape the declarations:

- Each Partition's stack sits at the bottom of its RAM region, so a stack overflow runs off the region and faults before it corrupts anything. This is why a Partition declares `stack` as well as `ram`.
- Region sizes are powers of two and aligned, so `code = 32.KiB()` and `ram = 32.KiB()` are the kinds of numbers you declare, and the generator places Partitions largest first. A layout that does not fit the chip's memory is rejected at generation time, not discovered at link time.

Peripherals with direct memory access (DMA) are a known hole in MPU-based isolation, because the peripheral writes memory without going through the MPU. IronOxide maps such peripherals read-only to their Partition and routes the pointer and length registers through a Kernel service that checks the buffer lies in the Partition's own RAM. The demo's `sampler` Task uses this.

Criticality levels decide one more thing: when a message crosses from a lower-level Partition to a higher-level one, the receiver must declare a validation function, and a message that fails validation is a fault of the sender. The more trusted side never has to trust the less trusted side's data.

### 4.7 Messages between Partitions: Endpoints and Protocols

Since Resources stay inside a Partition, Tasks in different Partitions talk through **Endpoints**: bounded message channels whose storage belongs to the Kernel, so neither Partition can corrupt it. A send copies the message in and never waits; if the channel is full the send returns an error and the Job decides what to do. A receive copies the oldest message out and returns `None` when there is nothing, again without waiting. The channel's `capacity` is declared, the message size `payload` is declared, and the Analyzer charges each operation's Kernel cost to the Job that performs it, up to the declared `ops` per Job.

A plain Endpoint carries any `Plain` type, meaning a `Copy` type with no pointers whose every bit pattern is valid, such as `u32`, arrays, and `#[repr(C)]` structs of such fields with `#[derive(rsk::Plain)]`.

A **Protocol** adds a contract about order. It is written as a global session type, which is a small language for saying who sends what to whom in what sequence, with choices and loops. The smallest useful one, a request-reply loop with a stop branch:

```
protocol Ping {
    rec L {
        choice at a {
            Req(u32): a -> b;
            Rep(u32): b -> a;
            continue L;
        } or {
            Stop(): a -> b;
            end;
        }
    }
}
```

Read it as: repeatedly, role `a` chooses either to send `Req` to `b`, after which `b` sends `Rep` back and the loop continues, or to send `Stop` to `b`, after which the conversation ends. The Protocol Checker in the generator projects this global description onto each role, giving `a` the local view "send Req then receive Rep, or send Stop", and `b` the view "receive Req then send Rep, or receive Stop". It also checks that the protocol is deadlock-free, computes the buffer each channel needs, and refuses a declared capacity below it.

The generator turns each role's projection into a **typestate API**: one Rust type per protocol state, where each operation consumes the current state value and returns the next one. In state `S0` the `a` role's value has a `send_req` method and a `send_stop` method and nothing else; `send_req` returns an `S1` value that has only `try_recv`. Calling `try_recv` on an `S0` is a compile error. The state value lives in a slot in the Task's context between Jobs; a Job takes it out, performs its operations, and stores the result back. Forgetting to store it, or receiving a message the state does not expect, is a run-time protocol violation that the Kernel logs. The effect is that a Task cannot send the wrong message at the wrong time, and the checker has already shown that the right messages at the right times never deadlock.

A `chain` adds an end-to-end deadline across several Tasks and Endpoints, and `after = <endpoint>` lets one role's operations wait, within a Job, on another role's receive. Both are the same machinery with more roles; the demo shows each.

### 4.8 What checks what

The project's view is that each property should be established by the cheapest tool that can establish it, and that tests are evidence, not proof. For a reader new to the project the most useful map is which tool catches which mistake:

| Property | Established by | When you notice |
| --- | --- | --- |
| A Task body touches only its declared state and Resources | The generated context type: the body receives only what the declaration gave it | Compile error |
| A lock's `&mut` cannot escape, locks nest LIFO, no re-entry | Closure-scoped `lock` taking `&mut self` | Compile error (borrow checker) |
| Bodies are synchronous and run to completion | The Job wrapper requires a plain `fn` returning `()` | Compile error |
| No `static mut`, no unbounded loop, no relative delay, no `unsafe` | The Profile Lint and `#![forbid(unsafe_code)]` in every application crate | Build error naming a PR rule |
| No recursion, no unresolved indirect call, no allocator, no FP instruction in an FPU-free Task | The Link Checker on the ELF | Build error |
| Ceilings, capacities, the memory map match the declaration | The Config Checker, an independent implementation that recomputes them from the Task Model and compares with the binary | Build error |
| The chip is what the configuration assumes (core revision, MPU regions, priority bits, clocks) | Kernel boot checks | Safe state at boot |
| Mutual exclusion, bounded blocking, deadlock freedom, drift-free release, MIT spacing, overrun detection whatever the ceiling | Kernel proofs in Verus over a model of the Cortex-M4 hardware | `cargo xtask verify` |
| The Kernel's unsafe hardware layer | Kani bounded model checking | `cargo xtask verify` |
| Every deadline met, with blocking and overheads | The Analyzer, proven to compute the least fixed point | `cargo xtask verify` (the report is an evidence item) |
| Task bodies are panic-free and preserve Resource invariants (levels A and B) | The App Prover, Creusot | `cargo xtask verify` |
| Release latency, Kernel entry costs, WCET records | Measured on the hardware rig with a logic analyzer | `cargo xtask hil` |

The type system gives the first three rows. It does not give deadlock freedom, timing, panic freedom or stack bounds, and the project does not pretend otherwise; each of those is assigned to a proof or an analysis.

## 5. From one declaration to a running binary

Everything above is configured from one place: the `rsk::app!` declaration. The build pipeline is:

1. The **declaration crate** contains the `rsk::app!` block and nothing else. The macro validates it against every Profile rule, with errors at the offending span, and exports two kinds of manifest macro: `rsk_system!` and one `rsk_partition_<name>!` per Partition.
2. Each **Partition crate** invokes its manifest, which expands to the generated context types, state cells, Resource proxies, role types and the Job wrappers the Kernel's vectors call. Then it defines one function per Task.
3. The **system crate** invokes `rsk_system!` and defines the safe-state function.
4. The **binary crate** has no code. It links the Kernel's `rsk-entry` crate, which owns the vector table and the reset handler, with the generated linker script.
5. `cargo xtask gen` runs the generator as a command-line tool on the declaration and writes `gen/`: the Task Model in JSON for the Analyzer, `link.x` and `memory.x` for the linker, the manifests, and a human-readable `summary.txt`.
6. `cargo xtask verify` runs every checker, proof and analysis in order and writes one evidence item per step.

The vocabulary you will meet in declarations, generated files and error messages:

| Term | Meaning |
| --- | --- |
| App_Declaration | The `rsk::app!` block: target, Partitions, Resources, Tasks, Endpoints |
| Kernel | The privileged code: dispatch, locks, timers, intake, fault handling, the event log. The only `unsafe` code in a build |
| Health_Monitor | The part of the Kernel that detects overruns, deadline misses, MIT violations and faults and applies responses |
| Generator | The proc macro and CLI (`rsk-gen`) that turn the declaration into tables, types and the linker script |
| Task_Model | The JSON description of the application the Analyzer reads |
| Analyzer | The response-time analysis tool |
| App_Prover | Creusot, applied to Partition crates |
| Profile_Lint, Link_Checker, Config_Checker | The three static checkers of Section 4.8 |
| Flight_Build | A build intended for the device, with debugging output removed |
| HIL rig | Hardware-in-the-loop: the nRF52840 boards and logic analyzer that measure timing |
| Semihosting | A debugger convention that lets the emulated program print to the host console and exit with a status |
| Safe state | The function the Kernel calls when the operating duration ends or a fault escalates; on QEMU it prints a report and exits |
| Operating duration | How long the application runs before the Kernel enters the safe state; every example here uses it to end a run cleanly |

---

# Part II: Building the examples

## Before you start

**Toolchain.** The workspace pins Rust 1.95.0 through `rust-toolchain.toml` and vendors every dependency, so builds run with `--frozen` and need no network. For the examples you need rustup with the `thumbv7em-none-eabihf` target and `qemu-system-arm` on your PATH; the xPack QEMU 9.2 build and Homebrew's both work. Verus, Kani and Creusot are needed only for the verification job in Step 6.

```
rustup target add thumbv7em-none-eabihf
cargo xtask check          # the static checks, about a minute
cargo xtask qemu           # builds the P1 demo and runs it on QEMU
```

`cargo xtask qemu` builds the demo binary in release mode for the target, then launches `qemu-system-arm -M mps2-an386 -cpu cortex-m4` with semihosting enabled and the binary as the kernel image. The mps2-an386 is QEMU's model of an Arm evaluation board with a Cortex-M4; it has no cycle counter, which is why Budgets are not enforced on it.

A good first run ends with the demo's safe-state report and `RSK-OK`. The letters before it are each Job's one-character debug print, so you can see the schedule interleaving with your eyes:

```
AFCGABDAEABFDACAEBFDAABDFAECABDAFGAEBDACABDFAEDABFADCAEBFAA
RSK-SAFE-STATE
control=20
sensor=9
log=5
...
RSK-OK
```

`A` is `control` every 10 ms, `C` is `sensor` every 20 ms, `F` is `filter` every 25 ms, and so on. Notice that `A` appears far more often than anything else and is never split: it has the highest priority, so nothing preempts it.

**How one application is laid out.** Every application is four crates under `apps/<name>/`, and the generator writes a fifth directory, `gen/`, that you never edit by hand:

| Directory | Crate role | What you write there |
| --- | --- | --- |
| `decl/` | The declaration | The `rsk::app!` block only |
| `system/` | The system crate | `rsk_system!()` plus the safe-state function |
| `part<N>/` (or `part/`) | One crate per Partition | `rsk_partition_<name>!()` plus the Task bodies |
| `bin/` | The Flight_Build binary | `use rsk_entry as _;` and a 30-line `build.rs` |
| `gen/` | Generated | Nothing: `cargo xtask gen` writes task_model.json, link.x, memory.x, the manifests and a summary |

Why four crates and not one? The memory layout of Section 4.6 is arranged at link time by placing each crate's compiled sections into its Partition's region, which needs one crate per Partition. The declaration is separate because both the system crate and every Partition crate depend on it. The binary is separate because exactly one application may be bound to the Kernel's entry crate at a time.

The smallest real application in the tree, `apps/hil-stimulus/`, is this shape with one Partition and one Task, and the tutorial's first examples follow it.

**Two habits that save time.** After any change to a declaration, run `cargo xtask gen` and read `gen/summary.txt`: it shows the memory map, every Task's converted numbers, every Ceiling and every Endpoint's projection, and most mistakes are visible there before anything runs. And before reaching for the forty-minute verification job, run the cheap checks: `cargo xtask check`, `cargo xtask gen --check`, the tests, and a QEMU run.

## Step 1: the declaration language and the smallest application

### The grammar

An `rsk::app!` block has a header, then any number of `partition`, `protocol`, `endpoint` and `chain` items. Every quantity carries a unit: times are `N.cycles()`, `N.ticks()`, `N.ns()`, `N.us()`, `N.ms()`, `N.s()`, `N.min()`, `N.h()`; sizes are `N.B()`, `N.KiB()`, `N.MiB()`. A bare number where a unit is required is a compile error at that number, because a bare 10 that meant milliseconds to the author and ticks to the generator is exactly the kind of mistake the project exists to prevent.

```
rsk::app! {
    target = <board>, profile = Ravenscar, operating_duration = <time>,
    safe_state = <path>, log_capacity = <n>, fp_flush_to_zero = <bool>;   // last two optional

    partition <name> (level = A|B|C|D|E, crate = <crate ident>,
                      code = <size>, ram = <size>, stack = <size>,
                      fault = <response>, overrun = <response>, deadline_miss = <response>,
                      mit_threshold = <n>) {
        peripherals = [<board peripheral names>];
        resource <name>: <type> = <init> [invariant |v: <type>| <pearlite>];
        state <task>.<name>: <type> = <init>;
        signal <name>;
        task <name> (periodic, period = <time>, [offset = <time>,] deadline = <time>,
                     budget = <time>, priority = <1..7>, [resources = [..],] [raises = [..],] [fpu]);
        task <name> (sporadic, signal = <signal> | binds = <peripheral>, mit = <time>,
                     mit_policy = defer, deadline = <time>, budget = <time>, priority = <n>);
    }

    protocol <Name> { <global type> }
    endpoint <name>: <type> | protocol <Name> { <role> => <part>.<task> [validate = ..] [ops = n] [after = <endpoint>], .. }
        capacity = <n>, payload = <bytes>;
    chain <name> { <part>.<task> -> <part>.<task> } deadline = <time>;
}
```

What each header field means:

| Field | Meaning | Notes |
| --- | --- | --- |
| `target` | The board: `qemu_mps2_an386` or `nrf52840` | Decides the clocks, the peripheral names, the memory map and the interrupt lines |
| `profile` | `Ravenscar` | The Profile variant; `Jorvik` exists as a reserved alternative |
| `operating_duration` | How long the application runs before the Kernel enters the safe state | Every tutorial example ends this way; a real device would declare a long duration or re-enter from the safe state |
| `safe_state` | A path to a `fn() -> !` in the system crate | Called after the duration ends and on any escalated fault |
| `log_capacity` | How many events the Kernel keeps | The safe state reads them |
| `fp_flush_to_zero` | Floating-point denormal handling for `fpu` Tasks | Optional |

What each Partition attribute means:

| Attribute | Meaning | Section |
| --- | --- | --- |
| `level` | Criticality A (most) to E (least) | 4.6 |
| `crate` | The Rust crate holding this Partition's bodies | 5 |
| `code`, `ram` | Sizes of the MPU regions; powers of two | 4.6 |
| `stack` | The stack at the bottom of the RAM region | 4.6 |
| `fault`, `overrun`, `deadline_miss` | The responses | 4.5 |
| `mit_threshold` | How many consecutive deferred events count as a fault | 4.5 |
| `peripherals` | The peripherals this Partition owns, which become its MPU windows | 4.6 |

The responses are `END_JOB`, `RESTART_PARTITION`, `STOP_PARTITION`, `SAFE_STATE`, `RECORD_ONLY` (deadline misses only) and `RECORD_AND_CONTINUE` (overruns of the highest-criticality Partition only). Every validation error names the Profile rule it breaks, for example `[PR-03] a Task declares exactly one Priority`, at the span of the offending item.

### Example 1: one Partition, one Task

Create `apps/tutorial/` with the four crates. The declaration crate holds nothing but the block:

```rust
// apps/tutorial/decl/src/lib.rs
#![no_std]
#![forbid(unsafe_code)]

rsk::app! {
    target = qemu_mps2_an386, profile = Ravenscar, operating_duration = 100.ms(),
    safe_state = crate::halt, log_capacity = 16;

    partition main (level = C, crate = tutorial_part, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                    fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                    mit_threshold = 10) {
        peripherals = [];
        state tick.count: u32 = 0;
        task tick (periodic, period = 10.ms(), deadline = 10.ms(), budget = 1.ms(), priority = 1);
    }
}
```

Line by line:

- `#![no_std]` says this crate does not use the Rust standard library, which needs an operating system. Embedded Rust uses `core` instead, which has no allocation, no threads and no I/O. `#![forbid(unsafe_code)]` is mandatory in every application crate; the Profile Lint checks it.
- The application runs for 100 ms, then calls `crate::halt` in the system crate.
- One Partition, `main`, at level C, whose bodies live in the crate `tutorial_part`. A fault restarts it, a Budget overrun ends the Job, a deadline miss is only recorded. It owns no peripherals.
- `state tick.count` is a `u32` that belongs to the Task `tick` and survives from one Job to the next. It is per-Task state, not a Resource, because only `tick` touches it, so no lock is needed. The Profile forbids a `static` of your own for this purpose, so that every piece of mutable state is visible in the declaration.
- `tick` is released every 10 ms, must finish within 10 ms of each Release, may execute for 1 ms per Job, and has the lowest priority. With one Task the priority does not matter yet.

The Partition crate pulls in its generated manifest and defines one function per Task, named as the Task is, taking the generated context type:

```rust
// apps/tutorial/part/src/lib.rs
#![no_std]
#![forbid(unsafe_code)]

tutorial_decl::rsk_partition_main!();

pub fn tick(mut cx: rsk_ctx::Tick) {
    cx.count.with(|c| *c = c.saturating_add(1));
    rsk::debug(&[b'T']);
}
```

The manifest macro expands to the generated modules (Step 2 lists them). `rsk_ctx::Tick` is the context: a struct with one field per thing the declaration gave this Task, here just `count`. The body is a plain function. It is called once per Job by a generated wrapper that the Kernel's interrupt vector for this Task invokes; the body cannot be `async`, cannot loop forever and cannot sleep, so it returns and the Job is over. `saturating_add` rather than `+` is a habit worth forming now: the App Prover will later ask you to show that nothing overflows, and a saturating counter needs no argument.

The system crate pulls in the other manifest and supplies the safe state the header named. On QEMU the safe state reports over semihosting and exits with 0 when the run went as expected, which is what `cargo xtask qemu` turns into its exit status:

```rust
// apps/tutorial/system/src/lib.rs
#![no_std]
#![forbid(unsafe_code)]

use rsk_kernel::arch::cm4::unsafe_module::hw;

tutorial_decl::rsk_system!();

/// No Health_Monitor event is expected in this run (used by HIL builds only).
pub const TRACE_EXPECT: (u32, u8, u32) = (0, 0xFF, 0);

pub fn halt() -> ! {
    let n = tutorial_part::rsk_state::TICK_COUNT.peek();
    if !hw::debugger_attached() {
        loop { hw::safe_state_idle() }
    }
    hw::semihosting_write(b"\nRSK-SAFE-STATE\n");
    // 100 ms at a 10 ms period: ten Jobs.
    hw::semihosting_exit(if n == 10 { 0 } else { 1 });
}
```

The safe state runs privileged, in the Kernel, after every Partition has been stopped, so it may read any Partition's state cells through `peek`. On a real device with no debugger it would idle forever; here it prints and exits. This "count the Jobs and compare with what the schedule predicts" pattern is how every example in this tutorial checks itself.

The binary crate has no code of its own. Its `main.rs` is three attributes and `use rsk_entry as _;`, and its `build.rs` adds `../gen` to the linker search path and passes `-Tlink.x` and `--nmagic`, exactly as `apps/hil-stimulus/bin/build.rs` does. Copy that file.

### Wiring the application into the workspace

Five edits, all of which the existing applications show:

1. Add the four crates to `members` in the root `Cargo.toml`.
2. Add them to `[workspace.metadata.rsk.crates]`: the decl crate with `declaration = true`, the bin with `hil-stimulus = true` so the verification job's Target build leaves it alone (an `rsk-entry` build binds one application at a time).
3. Give the system crate a `default = ["qemu-mps2"]` feature that enables `rsk-kernel/board-qemu-mps2`, and the bin a `default = ["qemu-mps2"]` forwarding to `rsk-entry/qemu-mps2`.
4. In `crates/rsk-entry/Cargo.toml` add an optional dependency on the system crate and a feature `app-tutorial = ["dep:tutorial-system"]`; in its `lib.rs` add the `use tutorial_system::Sys;` and `trace_expect` lines under `#[cfg(feature = "app-tutorial")]`, beside the P1 demo's. This is the one edit outside `apps/`.
5. The bin depends on `rsk-entry` with `default-features = false, features = ["app-tutorial"]`.

Note that this wiring is described from the patterns of `apps/hil-stimulus/` and `apps/p1-demo/`; the `apps/tutorial/` crates are not in the repository, so expect to adjust a path or a feature name on first build.

### Generate and run

```
cargo xtask gen                 # writes apps/tutorial/gen/
cat apps/tutorial/gen/summary.txt
cargo xtask qemu --bin tutorial
```

The summary is the first thing to read after any declaration change. For this example it shows one Partition with code at `0x00030000` and RAM at `0x20008000`, each a 32 KiB region, the 4 KiB stack, one Task on vector 36 (the first free interrupt line after the Kernel's), the period as 250,000 ticks of the 25 MHz time base and the Budget as 25,000 cycles. Everything you wrote in milliseconds has become the integer the Kernel and the Analyzer actually use, and you can check the conversion by hand.

The QEMU run prints ten `T` characters, then `RSK-SAFE-STATE`, and exits 0.

**Try it.** Change the period to 7 ms and the expected count to 15 (releases at 0, 7, ..., 98). Change `operating_duration` to 95 ms and watch the count drop to 10 again. Remove `budget` and read the `[PR-27]` error, which tells you every attribute a Task must declare.

## Step 2: what a Task body can do

A Task body is a synchronous function that returns `()`. It sees only its generated context, so the declaration decides what it can touch: `#![forbid(unsafe_code)]` is mandatory in every application crate and the only unsafe code in a Flight_Build is the Kernel's. Open `apps/tutorial/gen/partition_main.rs` after Step 1 to see what the manifest expands to. For the one-Task example it is about seventy lines, and reading it once removes most of the mystery. The modules you use from a body:

| Module | Contents | Example |
| --- | --- | --- |
| `rsk_ctx` | One context struct per Task, named in CamelCase | `rsk_ctx::Tick { count: PerTask<u32> }` |
| `rsk_state` | One static cell per `state` line, readable by the safe state | `rsk_state::TICK_COUNT.peek()` |
| `rsk_resources` | One cell per `resource` line (Step 4) | |
| `rsk_roles` | One typestate module per Endpoint role (Step 5) | `rsk_roles::ping_a::State` |
| `rsk_target` | The board's clocks and memory map | `rsk_target::ms(3)`, `rsk_target::TICK_HZ` |
| `rsk_jobs` | The Job wrapper the Kernel's vector calls; not for you | |

### The API in the `rsk` crate

Per-Task state is a `PerTask<T>` with `get`, `set` and `with(|v| ..)`. It needs no lock because only this Task's Jobs touch it and at most one Job of a Task runs at a time, which is one of the proven Kernel properties.

`rsk::now()` is the monotone time base in ticks. The tick rate depends on the board: the QEMU board's time base runs at 25 MHz and the nRF52840's at 16 MHz, so always convert through `rsk_target::ms(n)` or `rsk_target::TICK_HZ` rather than writing a tick count. `ms` saturates on overflow and carries a contract the App Prover uses.

`rsk::debug(&[u8])` writes to the semihosting console and is silent in `quiet` builds. `rsk::mmio::read` and `rsk::mmio::write` reach the peripheral windows the Partition declared; any other address is a MemManage fault attributed to the Partition. `rsk::spin::<N>()` burns a fixed number of iterations.

There is no sleep, no wait, no timer you can set, and no way to read a wall clock. Those are the Profile's rules PR-10, PR-24 and PR-26: periodic behaviour is expressed by declaring a periodic Task, and nothing else.

### Example 2: a Task that works for a measured time

Every loop must have a bound the Profile Lint can see (rule PR-16), because an unbounded loop has no WCET and a Task with no WCET cannot be analysed. A wait on the time base therefore always carries an iteration limit as well:

```rust
pub fn tick(mut cx: rsk_ctx::Tick) {
    // Busy for 3 ms by the time base, and at most 10,000,000 iterations.
    let start = rsk::now();
    let mut i = 0u32;
    'busy: while rsk::now().wrapping_sub(start) < rsk_target::ms(3) && i < 10_000_000 {
        i += 1;
    }
    cx.count.with(|c| *c = c.saturating_add(1));
    rsk::debug(&[b'T']);
}
```

The label `'busy` is optional; it names the loop in the lint's export and the measurement build's loop report. Where the bound is not visible in the guard, annotate the function with `#[rsk::bounds('scan = 8)]` as `xtask/fixtures/api-misuse/ok/src/lib.rs` does.

The declared Budget of 1 ms is now smaller than the 3 ms the body takes. On QEMU nothing happens, because Budgets are not enforced there. On the nRF52840 the Kernel would end the Job after 1 ms of execution and log an overrun, and `count` would never increment. Raise the Budget to `4.ms()`; the Analyzer would otherwise refuse the configuration once the rig had measured the body.

### What you cannot write, and the error you get

The fixtures under `xtask/fixtures/api-misuse/` each break one rule and are checked to fail. The ones you will hit first:

| Attempt | Rule | Result |
| --- | --- | --- |
| `async fn tick(..)` | PR-09 | The Job wrapper requires `()`; the body does not compile |
| Keeping a `&mut` from inside `lock` after the closure | R8.6 | Borrow error: the reference cannot leave the closure |
| Locking a Resource again inside its own closure | R8.10 | E0499: `lock` takes `&mut self` |
| A `static mut` or a `static` cell of your own in the Partition crate | PR-05 | The Profile Lint rejects it; declare `state` or `resource` instead |
| Declaring a Task twice, or without `priority` | PR-01, PR-03 | Compile error at the span, naming the rule |
| A `loop {}` or a `while` with no visible bound | PR-16 | The Profile Lint rejects it; add a bound or a `#[rsk::bounds]` |
| Calling a function that calls itself | PR-17 | The Link Checker rejects the binary |

Re-run the three commands of Step 1 after each change. `cargo xtask gen --check` tells you when the generated files have drifted from the declaration, which the verification job also enforces.

## Step 3: timing attributes, responses and the event log

Every Task carries three times the Kernel enforces at run time, and one it treats as a start condition. The Analyzer later uses the same numbers for the response-time analysis of Section 4.3, so they are the contract between the application and the proof.

| Attribute | Meaning | What happens when it is broken |
| --- | --- | --- |
| `period` / `mit` | Release interval of a periodic Task, or the minimum inter-arrival time of a sporadic one | A release inside the MIT is deferred (`mit_policy = defer`); more than `mit_threshold` such violations is a fault of the Partition |
| `offset` | Delay of the first release after boot, periodic Tasks only (default 0) | Never broken; used to spread releases |
| `deadline` | Relative deadline of each Job | The Partition's `deadline_miss` response: `RECORD_ONLY` logs it |
| `budget` | The execution time one Job may consume, in time or `cycles` | The Partition's `overrun` response: `END_JOB` ends the Job, `RECORD_AND_CONTINUE` lets it finish (level A only) |
| `priority` | 1 to 7, 7 the most urgent | Two Tasks of one Partition may share a Priority; the Kernel still dispatches the earlier release first |

QEMU does not count cycles, so Budgets are not enforced there and a Job that runs long only misses its deadline. On the nRF52840 the Kernel charges every segment of a Job to its Budget, including the supervisor calls it makes, which is why the demo's Budgets were raised after the first hardware measurements.

### Example 3: two periodic Tasks that collide

Add a second Task to the Partition of Step 1 with a period that is not a multiple of the first, and an offset that places one release inside the other's work:

```rust
state tick.count: u32 = 0;
state slow.count: u32 = 0;
task tick (periodic, period = 10.ms(), deadline = 10.ms(), budget = 1.ms(), priority = 2);
task slow (periodic, period = 25.ms(), offset = 5.ms(), deadline = 25.ms(), budget = 6.ms(), priority = 1);
```

Give `slow` the 3 ms loop of Step 2 and `tick` a one-line count, with `slow` printing `s` at the start of its loop and `S` at the end. On the time line, `slow` is released at 5, 30, 55 and 80 ms and works for 3 ms each time, so its first Job spans 5 to 8 ms; `tick` is released at 10, 20, ... and never lands inside it. Change `slow`'s offset to `8.ms()`: now its Job spans 8 to 11 ms and `tick`'s release at 10 ms falls inside it. In the QEMU output the `T` now appears between `s` and `S`: preemption, by priority, exactly as Section 4.2 described. Swap the two priorities and the `T` moves after the `S`; `tick`'s Job is late by up to 3 ms, still inside its 10 ms deadline.

Run the response-time arithmetic by hand for the second configuration. `tick` has priority 2 and nothing above it: R = 1 ms. `slow` has priority 1 and `tick` above it: R = 3, then 3 + ⌈3/10⌉·1 = 4, then 4 again. Both well inside their deadlines, which is what the Analyzer reports for the generated model when the verification job runs it.

Change `slow`'s budget to `2.ms()` and leave its loop at 3 ms. The run is still clean on QEMU; on hardware it would log an overrun and `END_JOB` would cut the Job after 2 ms, so `S` would never print.

### Reading the event log in the safe state

The Kernel keeps `log_capacity` events, each with a kind, a Task and a Partition. The safe state walks them through the kernel handle and decides the exit code. The P1 demo's `report_and_exit` is the full version; the core of it is:

```rust
use rsk_kernel::arch::{KernelOps, System};
use rsk_kernel::logic::log::{EV_DEADLINE_MISS, EV_OPERATING_END, EV_OVERRUN, EV_SAFE_STATE};

pub fn halt() -> ! {
    let k = Sys::kernel().get();
    let len = k.log_len();
    let mut unexpected = 0u32;
    let mut i = 0;
    'events: while i < len {
        let kind = k.log_kind(i);
        let task = k.log_task(i);
        if kind != EV_OPERATING_END && kind != EV_SAFE_STATE {
            unexpected += 1;
        }
        i += 1;
    }
    hw::semihosting_exit(if unexpected == 0 { 0 } else { 1 });
}
```

The loop bound is the log capacity; the demo writes it as `#[rsk::bounds('events = NE)]` on the function. A clean run ends with exactly two events, kinds 10 (`EV_OPERATING_END`) and 18 (`EV_SAFE_STATE`). Part III lists every kind; the ones you will meet while experimenting are 6 overrun, 7 deadline miss, 9 MIT violation, 16 Partition restart, 17 Partition stop, 19 Job ended, 2 MemManage fault, 1 panic.

**Try it.** Give `slow` a deadline of `2.ms()` with the 3 ms loop. The log now holds a kind-7 event per `slow` Job, and your safe state can count them and compare with the four Releases. Then change `deadline_miss` to `STOP_PARTITION`: one kind-7, one kind-17, and `tick`'s count stops at the Job before the stop, because the whole Partition stopped.

### Where the Analyzer fits

`cargo xtask verify` runs the Analyzer on the generated Task Model with the measured WCET records of the HIL rig, or with placeholders when none exist, and fails the job if any Task's worst-case response time exceeds its deadline. Its blocking term comes from the Critical Sections of Step 4, so a Resource held too long by a low-priority Task is reported as a deadline problem of the high-priority one. The demo's declaration comments record one such case: `sensor`'s 8 ms section put `control` exactly at its 10 ms deadline, and the section was shortened to 6 ms.

## Step 4: sharing a Resource, and releasing a Task by Signal

Tasks of one Partition share state only through a declared `resource`, and only the Tasks listed in a Task's `resources = [..]` get a proxy for it in their context. The generator computes each Resource's Ceiling as the highest priority among its accessors, and the Kernel runs every lock under the Stack Resource Policy of Section 4.4: a Job that holds a Resource cannot be preempted by a Job whose priority is at or below the Ceiling, so a higher-priority release lands after the unlock. Deadlock and priority inversion are ruled out by construction, and a Job can never block waiting for a lock.

### Example 4: the fixture declaration

`xtask/fixtures/api-misuse/decl/src/lib.rs` is the smallest complete example of all three constructs. Two Resources, a Signal, two periodic Tasks and one sporadic Task released by the Signal:

```rust
partition part0 (level = A, crate = api_ok, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                 fault = RESTART_PARTITION, overrun = RECORD_AND_CONTINUE, deadline_miss = RECORD_ONLY,
                 mit_threshold = 10) {
    peripherals = [];
    resource state: u32 = 0;
    resource other: u32 = 0;
    signal tick;
    task alpha (periodic, period = 10.ms(), deadline = 10.ms(), budget = 1.ms(), priority = 7,
                resources = [state, other], raises = [tick]);
    task beta (periodic, period = 20.ms(), deadline = 20.ms(), budget = 1.ms(), priority = 5,
               resources = [state]);
    task gamma (sporadic, signal = tick, mit = 5.ms(), mit_policy = defer, deadline = 5.ms(),
                budget = 1.ms(), priority = 3);
}
```

`state` has Ceiling 7 (alpha and beta access it), `other` has Ceiling 7 (alpha alone). The summary lists both. The bodies, from the `ok` member of the same fixture:

```rust
use rsk::Mutex;

api_decl::rsk_partition_part0!();

pub fn alpha(mut cx: rsk_ctx::Alpha) {
    Mutex::lock(&mut cx.state, |s| *s += 1);      // through the trait
    let n = cx.other.lock(|o| {                      // or the inherent method
        *o += 1;
        *o
    });
    if n > 3 {
        let _ = cx.tick.raise();                     // releases gamma; false if the MIT deferred it
    }
}

pub fn beta(mut cx: rsk_ctx::Beta) {
    cx.state.lock(|s| *s = 0);
}

pub fn gamma(_cx: rsk_ctx::Gamma) {}
```

The closure of `lock` is the Critical Section. It receives `&mut T`, returns a value, and cannot call anything that waits, because nothing in the API waits. The lock itself is a supervisor call: the Kernel checks that this Task declared the Resource, raises the System Ceiling to the Resource's Ceiling, runs the closure, and restores the ceiling. Because `lock` takes `&mut self`, locking the same Resource inside its own closure is a borrow error rather than a run-time deadlock, which is what the `reentry` member of the fixture demonstrates. Nested locks on different Resources are fine and release in last-in-first-out order because closures nest that way.

### Seeing the ceiling work

Port this into the tutorial application and make `beta` hold `state` for 3 ms with the loop of Step 2 while `alpha` runs every 10 ms. The declared period of `beta` is 20 ms with offset 0, so `beta`'s section spans 0 to 3 ms, 20 to 23 ms, and so on, and `alpha`'s releases at 0, 20, 40 ms fall inside it.

Count in `alpha` how often it found `beta` mid-section by adding a second Resource `in_cs: u32 = 0 invariant |v: u32| v@ <= 1;` that `beta` sets to 1 before and 0 after its section, exactly as the demo's `sensor` and `control` do. Every `alpha` release that falls inside the section runs after the unlock, and `alpha` reads `in_cs == 1` at that moment. Section 4.4 predicts this: `alpha`'s priority 7 is not above the ceiling 7 that `beta` raised, so `alpha` cannot start until `beta` unlocks. The demo's safe state reports this count as `blocked_releases`, nine in its 200 ms run.

Now do the experiment the naive mutex would fail. Add a third Task `mid` at priority 6 with no Resources, period 20 ms and offset 1 ms, with a 5 ms loop. Under a mutex, `alpha` released at 20 ms would block on `state`, `mid` released at 21 ms would preempt `beta` and run for 5 ms, and `alpha` would wait until about 28 ms. Under SRP, `mid` at priority 6 is also not above the ceiling 7, so it does not start either; `beta` unlocks at 23 ms, `alpha` runs, then `mid`. The order of the debug letters in the QEMU output shows it. `alpha`'s worst-case blocking is 3 ms whatever `mid` does.

The `invariant` clause is optional and only matters to the App Prover (Step 6): it is a pearlite predicate the prover assumes on entry to each lock and must prove on exit. Here it says the flag is 0 or 1. Pearlite is Creusot's specification language; `v@` is the mathematical integer value of `v`.

### Signals and sporadic Tasks

A Signal is the software release source: `raise` is a Kernel call that records a release of the bound sporadic Task. The release takes effect when the raising Job's priority allows, which usually means at once if the sporadic Task is more urgent, or after the raising Job ends if it is less. A raise inside the Task's MIT is deferred with `mit_policy = defer`, and `raise` returns `false` so the raising Job knows; more than `mit_threshold` deferrals in a row is a fault of the Partition. In the demo, `control` raises `tick` every third Job, every 30 ms, against `echo`'s 20 ms MIT, so every raise is accepted.

**Try it.** In the fixture, `alpha` raises `tick` on every Job after the fourth, every 10 ms, against `gamma`'s 5 ms MIT: every raise is accepted. Change the MIT to `15.ms()`: now every second raise is deferred, `raise` returns `false`, and the log shows a kind-9 event for each. Set `mit_threshold = 2` and the third consecutive deferral restarts the Partition.

## Step 5: an interrupt-released Task, a second Partition, and Endpoints

### Peripherals and interrupt releases

A Partition lists the peripherals it owns in `peripherals = [..]`; the generator turns each into an MPU window the Partition can read and write through `rsk::mmio`, and nothing else can. On QEMU the names are `CMSDK_TIMER0`, `CMSDK_TIMER1`, `CMSDK_DUALTIMER`, `CMSDK_UART0`, `CMSDK_UART1` and `CMSDK_GPIO0`; on the nRF52840 they include `GPIOTE`, `P0`, `SPIM0`, `TIMER1`, `RTC1`, `PWM0`, `SAADC` and `UARTE0`. The Kernel owns the rest, in particular its time-base timer, and the summary lists which.

A sporadic Task bound to a peripheral with `binds = <name>` is released by that peripheral's interrupt. The sequence, from Section 4.5: the interrupt is taken at the Kernel's level by the intake handler, which timestamps it, masks the line, checks the MIT, and pends the Task's dispatch vector; the Job runs, acknowledges the event in the peripheral, and returns; the line is unmasked at the later of one MIT after the event and the Job's end. The Job must acknowledge the event itself, because the Kernel knows nothing about the peripheral's registers.

### Example 5a: a timer-driven sporadic Task

The demo's `part1` does this with the dual timer. The declaration:

```rust
partition part1 (level = C, crate = p1_demo_part1, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                 fault = STOP_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                 mit_threshold = 10) {
    peripherals = [CMSDK_DUALTIMER];
    state log.count: u32 = 0;
    state pulse.count: u32 = 0;
    task log (periodic, period = 40.ms(), deadline = 40.ms(), budget = 6.ms(), priority = 2);
    task pulse (sporadic, binds = CMSDK_DUALTIMER, mit = 15.ms(), mit_policy = defer, deadline = 15.ms(),
                budget = 500.us(), priority = 4);
}
```

The periodic `log` starts the timer in its first Job, and every `pulse` Job clears the interrupt. From `apps/p1-demo/part1/src/lib.rs`:

```rust
mod pulse_source {
    const DUALTIMER: u32 = 0x4000_2000;
    const TIMER1_LOAD: u32 = DUALTIMER + 0x00;
    const TIMER1_CONTROL: u32 = DUALTIMER + 0x08;
    const TIMER1_INTCLR: u32 = DUALTIMER + 0x0C;
    const PERIOD_TICKS: u32 = 450_000;             // 18 ms at 25 MHz, longer than the 15 ms MIT

    pub fn start() {
        rsk::mmio::write(TIMER1_LOAD, PERIOD_TICKS);
        rsk::mmio::write(TIMER1_CONTROL, (1 << 7) | (1 << 6) | (1 << 5) | (1 << 1));
    }
    pub fn acknowledge() {
        rsk::mmio::write(TIMER1_INTCLR, 1);
    }
}

pub fn log(mut cx: rsk_ctx::Log) {
    if cx.count.get() == 0 {
        pulse_source::start();
    }
    cx.count.with(|c| *c = c.saturating_add(1));
}

pub fn pulse(mut cx: rsk_ctx::Pulse) {
    pulse_source::acknowledge();
    cx.count.with(|c| *c = c.saturating_add(1));
}
```

The addresses are the dual timer's registers on the mps2-an386, and the window the summary shows for `part1` is exactly `0x40002000` of 4 KiB. The 18 ms timer period is longer than the 15 ms MIT, so every event is accepted. Make the timer period 10 ms and every second event arrives inside the MIT: it is deferred, the source stays masked until the window ends, and the count halves. This is the Kernel protecting the rest of the system from a peripheral that fires too often.

Why is `pulse`'s deadline 15 ms when its Job takes well under a millisecond? Because `sensor` in `part0` has a higher priority and a 6 ms Critical Section, and `control` runs twice in any 15 ms window; the declaration's comments walk through the Analyzer's result of 11.5 ms. A deadline of 2 ms made the Kernel log a miss at every Release that coincided with `sensor`'s section, and 10 ms made the Analyzer reject the set. This is the response-time analysis of Section 4.3 doing its job on a real set.

The same Task on the nRF52840 is released by a GPIOTE edge on pin P1.01, which is how the HIL rig measures release latency; the `pulse_source` module is selected by a cargo feature and the declaration does not change.

### A second Partition

A second Partition is just another `partition` block with its own crate, a second `rsk_partition_<name>!()` manifest, and its own `[workspace.metadata.rsk.crates]` entry. Partitions share nothing: no Resource crosses a Partition boundary, a fault in one is handled by that Partition's `fault` response while the others keep their deadlines, and the summary shows each with its own disjoint code, RAM and window addresses. The only way across is an Endpoint.

**Try it.** Give the tutorial application a second Partition at level B with one periodic Task that does `rsk::mmio::write(0x4000_2000, 0)`, the dual timer that the first Partition owns. The write is outside the second Partition's view, so the Kernel logs a kind-2 MemManage fault against it and applies its fault response. The first Partition's counts are unaffected.

### Example 5b: a plain Endpoint

An Endpoint is a bounded channel in Kernel-owned storage between two Tasks of any Partitions, over a `Plain` message type: `u32`, arrays and `#[repr(C)]` structs of `Plain` fields with `#[derive(rsk::Plain)]`. From the demo:

```rust
endpoint telemetry: u32 { ctl => part0.control, sink => part1.log ops = 4 } capacity = 4, payload = 4;
```

`ops` is how many operations a Job may perform per release (the Analyzer charges each), `capacity` the queue length in each direction, `payload` the bytes per message. The sender and the receiver never wait:

```rust
// in control
match cx.telemetry.send(&cx.count.get()) {
    Ok(()) => {}
    Err(rsk::SendError::PeerChanged) => { /* the peer Partition restarted or stopped */ }
    Err(_) => cx.telemetry_full.with(|f| *f = f.saturating_add(1)),
}

// in log: drain at most the capacity per Job
let mut i = 0u32;
'drain: while i < 4 {
    match cx.telemetry.recv() {
        Ok(Some(v)) => { cx.telemetry_last.set(v); i += 1; }
        Ok(None) | Err(_) => break 'drain,
    }
}
```

`control` sends every 10 ms and `log` drains every 40 ms, so four messages accumulate between drains and the capacity of 4 is exactly enough. Set the capacity to 2 and `telemetry_full` counts two refused sends per `log` period; the sender is never delayed, it just learns that the message was not taken. `PeerChanged` is the error a Job sees after the other Partition restarted or stopped, so that stale state on one side is never mistaken for fresh messages.

When a message crosses from a lower criticality level to a higher one, the receiving role must name a validation function with `validate = crate::f`, a total `fn(&T) -> bool`; a rejected message is a fault of the sender's Partition. In the demo, `telemetry`'s `ctl` role declares one although `control` never reads that direction, because the channel from level C to level A exists.

### Example 5c: a Protocol-governed Endpoint

A `protocol` block gives the Endpoint a global session type (Section 4.7), and the generator emits a typestate API per role so that a body can only send and receive what the Protocol allows in its current state. The fixture's `Ping` is the smallest useful one:

```rust
protocol Ping {
    rec L {
        choice at a {
            Req(u32): a -> b;
            Rep(u32): b -> a;
            continue L;
        } or {
            Stop(): a -> b;
            end;
        }
    }
}
endpoint ping: protocol Ping { a => part0.alpha, b => part0.beta } capacity = 1, payload = 8;
```

The summary prints the projection onto each role: for `a`, `rec L.b!{Req(u32).b?{Rep(u32).L}, Stop().end}`, read as "send to b either Req, then receive Rep from b and loop, or Stop, then end". The payload of 8 bytes is the label word plus the `u32`. The checker bounds each direction at one message in flight, which the capacity of 1 meets.

Each role's state lives in a slot in the Task's context. A Job takes it out, performs one step, and stores the result back; every method consumes the state and returns the next one, so a wrong sequence is a type error. The `a` role, from the fixture's `ok` member:

```rust
pub fn alpha(mut cx: rsk_ctx::Alpha) {
    use rsk_roles::ping_a::{NotReceived, NotSent, RecvS1, State};
    let next = match cx.ping.take(State::Empty) {
        State::S0(a) => match a.send_req(1) {
            Ok(a1) => State::S1(a1),
            Err(NotSent::Full(a0)) | Err(NotSent::Refused(a0)) | Err(NotSent::PeerChanged(a0)) => State::S0(a0),
        },
        State::S1(a) => match a.try_recv() {
            Ok(RecvS1::Rep(_, a0)) => State::S0(a0),
            Err(NotReceived::Empty(a1)) | Err(NotReceived::Unexpected(a1)) | Err(NotReceived::Rejected(a1)) => State::S1(a1),
            Err(NotReceived::PeerChanged(a0)) => State::S0(a0),
        },
        other => other,
    };
    cx.ping.store(next);
}
```

Follow the types. `State::S0(a)` holds a value of the generated type for state 0, which has `send_req` and `send_stop`. On success `send_req` returns the state-1 value, which has only `try_recv`. On failure it hands the state-0 value back inside the error so that nothing is lost and the Job can try again next time. `try_recv` on state 1 either delivers `Rep` with the state-0 value, or an error that returns the state-1 value, or `PeerChanged`, after which the conversation restarts from state 0. Every arm ends in a `State` that is stored back. Try writing `a.try_recv()` in the `S0` arm: the method does not exist on that type.

The `b` role mirrors it: `try_recv` in S0 yields `RecvS0::Req(v, b1)` or `RecvS0::Stop(e)`, and `b1.send_rep(v)` returns to S0. A Job that forgets to `store` the state is a Protocol violation the Kernel logs as event 12. The `protocol_state` and `protocol_reuse` fixture members show the two misuses that fail to compile.

Three-party Protocols (`triangle` in the demo, where `ctl` chooses, `mid` learns the choice from the label it receives and `sink` sees the merge of both branches), end-to-end `chain` deadlines, and `after = <endpoint>` gating of one role on another's receive are the same machinery with more roles; the demo declaration carries a worked and commented example of each.

## Step 6: the full demo, hardware, and the verification job

`apps/p1-demo/` is every construct of Steps 1 to 5 in one application: three Partitions at three criticality levels, seven Tasks, two Resources with invariants, a Signal, a timer-bound sporadic Task, a floating-point Task, an EasyDMA request, one plain and four Protocol-governed Endpoints, and a chain deadline. Read `apps/p1-demo/decl/src/lib.rs` top to bottom; its comments explain why each number is what it is, and most of them were changed at least once after a measurement or an analysis.

| Task | Partition (level) | Priority | Release | Uses |
| --- | --- | --- | --- | --- |
| `control` | part0 (A) | 7 | every 10 ms | `state`, `in_cs`; raises `tick`; ctl of `telemetry`, `pingpong`, `triangle`; sink of `report` |
| `echo` | part0 (A) | 6 | sporadic, signal `tick`, MIT 20 ms | counts |
| `sensor` | part0 (A) | 5 | every 20 ms, offset 5 ms | holds `state` for 6 ms |
| `pulse` | part1 (C) | 4 | sporadic, `CMSDK_DUALTIMER`, MIT 15 ms | acknowledges the timer |
| `filter` | part2 (B) | 3 | every 25 ms | `fpu`; `stats`; sink of `relay`, ctl of `report` |
| `log` | part1 (C) | 2 | every 40 ms | starts the timer; sink of `telemetry` and `pingpong`, ctl of `relay`, mid of `triangle` |
| `sampler` | part2 (B) | 1 | every 100 ms | `rsk::dma_request` on `CMSDK_GPIO0` |

Look at the priorities against the periods: they are rate-monotonic (10, 20, 25, 40, 100 ms in descending priority) with the two sporadic Tasks slotted by their MITs, except that `pulse` at MIT 15 ms sits below `sensor` at 20 ms, and the declaration explains why. Look at the levels: the most trusted Partition has `RECORD_AND_CONTINUE` and the others `END_JOB`, as Section 4.5 described. Look at the Endpoints: every message from level C to B or A names a validation function.

### Run it on QEMU

`cargo xtask qemu` builds the default configuration and runs it for its 200 ms operating duration. The safe state prints every counter and checks each against the expectation in `apps/p1-demo/system/src/lib.rs`: twenty `control` Jobs, nine blocked releases, a floating-point state between 60 and 100 after eight `filter` Jobs, one injected rejected reply with its Partition stop, two injected Protocol violations. `cargo xtask qemu --flood` runs the variant in which every reply is invalid, to show that `control` keeps its deadlines under a flood of rejections, which is the temporal-partitioning claim made concrete.

The Analyzer's report for this model is written by the verification job under `target/xtask-verify/evidence/` (the `analysis-rta-*` items): each Task's C, B with the Critical Section that determines it, the interference terms, R and the slack. The tool itself is `cargo run -p rsk-analyzer`, which needs the Task Model, the build manifest the job writes, and either measured records or `--placeholders`. Compare the numbers with Section 4.3 and with the declaration's comments.

### Run it on the nRF52840-DK

The same declaration has a board variant, `apps/p1-demo/decl/src/nrf52840.rs`, selected by the cargo feature `nrf52840`; `cargo xtask gen` writes its outputs to `gen-nrf52840/`, and `bin/build.rs` picks that directory from the enabled feature. The HIL rig, two or three development kits plus a logic analyzer wired as `hil-rig.toml` records, is driven by `cargo xtask hil`:

```
cargo xtask hil --spike-only          # flash, run, read the safe-state report
cargo xtask hil --release-timing --short   # release latency against the stimulus board
cargo xtask hil --response            # fault injection: overruns, faults, restarts
cargo xtask hil --isolation           # the MPU access matrix and the DMA cases
cargo xtask hil --cycles              # Kernel entry costs, written as timing records
```

Each of these is a further declaration variant (`nrf52840_response`, `nrf52840_isolation`, and so on) with its own `gen-<variant>/` directory, and the Partition crates select the matching injections by feature. The rig writes the measured WCET and Kernel timing records the Analyzer consumes, under `docs/hil/`. The isolation suite is worth reading about even without the hardware: it has each Partition attempt every forbidden access to every other Partition and to the Kernel, and checks that each one faults.

### The verification job

`cargo xtask verify` is the CI entry point and takes about forty minutes on a laptop with every tool installed. In order, it runs the static checks, the network sandbox probe, the Target and host builds, the generated-file drift check, the tests and fixtures, the Kernel proofs under Verus, the Kani harnesses, the App Prover (Creusot) on every Partition body in every configuration, the Analyzer, and the QEMU run, and writes one evidence item per step under `target/xtask-verify/evidence/`. Before running it, do the cheap checks that catch almost everything:

```
cargo xtask check
cargo xtask gen --check
cargo test -p rsk-gen -p xtask
cargo xtask qemu
```

Then one `cargo xtask verify` as confirmation. Never run two at once; they share `target/xtask-mutations/tree` and the why3find session.

### What the App Prover asks of your bodies

Creusot compiles each Partition crate with `--cfg creusot` and proves every Task body panic-free under its declared Resource invariants. "Panic-free" means every arithmetic operation is shown not to overflow, every index is shown in bounds, every `unwrap` is shown to succeed, and every loop is shown to terminate. The idioms that keep a body provable, all visible in `apps/p1-demo/part0/src/lib.rs`:

- Count with `saturating_add`, never `+`, on anything the prover cannot bound.
- Give every time-base loop an `invariant(i@ <= N)` and `variant(N - i@)` under `#[cfg_attr(creusot, ..)]`, matching the iteration limit in the guard.
- Give a lock closure that must preserve a Resource invariant an `ensures` under `#[cfg_attr(creusot, ..)]`, with a plain closure under `#[cfg(not(creusot))]`.
- Call generated helpers rather than re-deriving them: `rsk_target::ms` carries a contract the prover uses; a `const fn` of your own without one returns an unconstrained value.

A proof member is one entry in `proofs/creusot/Cargo.toml` per Partition and configuration, and is proven alone with `cargo creusot -p <member> -- --offline`.

---

# Part III: Reference

## Event log kinds

The Kernel's log entries carry one of these kinds, defined in `crates/rsk-logic/src/logic/log.rs`:

| Kind | Name | Raised when |
| --- | --- | --- |
| 1 | `EV_PANIC` | A Task body panicked; the Partition's fault response applies |
| 2 | `EV_MEMMANAGE` | A memory access outside the Partition's view |
| 6 | `EV_OVERRUN` | A Job exhausted its Budget |
| 7 | `EV_DEADLINE_MISS` | A Job was still running at its deadline |
| 9 | `EV_MIT_VIOLATION` | A sporadic event arrived inside the MIT |
| 10 | `EV_OPERATING_END` | The operating duration elapsed |
| 12 | `EV_PROTOCOL_VIOLATION` | A role slot was left empty or an unexpected label arrived |
| 16 | `EV_PARTITION_RESTART` | A Partition was restarted |
| 17 | `EV_PARTITION_STOP` | A Partition was stopped |
| 18 | `EV_SAFE_STATE` | The safe state was entered |
| 19 | `EV_END_JOB` | A Job was ended by a response |
| 25 | `EV_ENDPOINT_FULL` | A send found the channel full |
| 26 | `EV_MESSAGE_REJECTED` | A validation function rejected a message |

The accessors are `k.log_len()`, `k.log_kind(i)`, `k.log_task(i)` and `k.log_partition(i)` on the kernel handle.

## Common errors and what they mean

| Symptom | Cause | Fix |
| --- | --- | --- |
| `error: [PR-xx] ...` at a line of the declaration | A Profile rule, named in the message | Read the rule in `docs/profile.md`; the fix is usually one attribute |
| `requires \`priority = ...\`` | A required Task or Partition attribute is missing | Every required attribute is listed in Step 1's grammar |
| `a unit takes no arguments: write \`10.ms()\`` | A time or size without its unit | Add the unit |
| `generated files are stale` from `cargo xtask gen --check` | The declaration changed after the last `gen` | `cargo xtask gen`, then commit `gen/` with the declaration |
| E0499 on a nested `lock` | Re-entering a Resource inside its own Critical Section | Read the value out first, then lock the other Resource |
| The Profile Lint rejects a `static` | State outside the declaration | Declare `state <task>.<name>` or a `resource` |
| A QEMU run exits 2 with no report | The safe state found no debugger; on QEMU this means semihosting is off | Run through `cargo xtask qemu`, which enables it |
| Event kind 2 at a `mmio::write` | The address is outside the Partition's peripheral windows | Add the peripheral to `peripherals = [..]`, or check the offset |
| `rsk-entry binds exactly one Application` | Two `app-*` features enabled, or none | One system crate per binary; check the bin's `rsk-entry` features |
| A Creusot goal fails on `+` or `/` | Unbounded arithmetic | `saturating_add`; divide only by a value with a contract |
| The Analyzer reports FAIL for a Task with a short body | Blocking from a lower Task's Critical Section, or interference from higher Tasks | Read the report's B term and the section it names; shorten the section or raise the priority |
| A count on hardware is lower than on QEMU | Budget overruns ending Jobs, which QEMU does not enforce | Compare the Budget with the rig's WCET record |

## Glossary

| Term | Meaning |
| --- | --- |
| Blocking | The delay a Job suffers because a lower-priority Job holds a Resource with a Ceiling at or above its priority; at most one Critical Section under SRP |
| Budget | The execution time a Job may consume; at least its WCET; enforced by the Kernel |
| Ceiling | A Resource's attribute: the highest priority among its accessing Tasks |
| Critical Section | The closure passed to `lock`; runs with the System Ceiling raised |
| Criticality level | A to E, the importance of a Partition; decides responses, proof obligations and validation |
| Deadline | The time after a Release by which the Job must finish |
| DO-178C | The avionics software standard whose Design Assurance Levels the Partition levels follow |
| Fixed point | The value where the response-time iteration stops changing |
| Interference | The time higher-priority Jobs take away from a Job |
| Job | One execution of a Task body, from Release to return |
| MIT | Minimum inter-arrival time of a sporadic Task's events, enforced by the Kernel |
| MPU | Memory Protection Unit, the Cortex-M4 hardware that confines each Partition |
| NVIC | Nested Vectored Interrupt Controller, the hardware that dispatches Tasks |
| Partition | The unit of isolation: a crate, a memory map, a criticality level, a set of responses |
| Preemption | Suspending a running Job because a more urgent one was released |
| Priority inversion | A high-priority Job waiting on a lower one through an unrelated middle one; impossible under SRP |
| Projection | A Protocol as seen from one role |
| Rate-monotonic | The priority assignment "shorter period, higher priority" |
| Release | The instant a Job becomes eligible to run |
| Resource | Shared state inside a Partition, accessed under SRP |
| Response time | How long after its Release a Job finishes; the Analyzer bounds the worst case |
| Session type | A description of a conversation: who sends what to whom, in what order, with what choices |
| Signal | A software release source one Task raises to release a sporadic Task |
| SRP | Stack Resource Policy, the ceiling-based locking discipline of Section 4.4 |
| Sporadic | Released by an event, subject to an MIT |
| System Ceiling | The highest Ceiling of any held Resource; in hardware, BASEPRI |
| Typestate | A programming pattern where each state is its own type and operations move between types |
| WCET | Worst-case execution time, measured on the hardware rig |
| WCRT | Worst-case response time, computed by the Analyzer |

## Where to go next

**Documents in the repository**

| Document | Read it for |
| --- | --- |
| `.kiro/specs/rsk-ravenscar-profile/design.md` | The design decisions DD-01 to DD-13: unprivileged Jobs, SRP in hardware, the memory map, Endpoints, the comparison with Ada Ravenscar and RTIC |
| `.kiro/specs/rsk-ravenscar-profile/requirements.md` | Every rule cited in an error message (`R8.10`, `PR-16`) with its rationale, and the full response-time equation (R29.1) |
| `docs/profile.md` | The Profile's restriction table: what is forbidden, how it is enforced, the Ada Ravenscar analogue |
| `docs/verification_plan.md` | What the verification job checks, the proof strategy per component, the assumptions |
| `docs/traceability_matrix.md` | Requirement to code to evidence, generated by `cargo xtask trace` |
| `apps/p1-demo/decl/src/lib.rs` | The commented full declaration; its numbers were all tuned against measurements |

**Background reading outside the repository.** For the scheduling theory, Burns and Wellings, *Real-Time Systems and Programming Languages*, covers fixed-priority scheduling, response-time analysis and Ravenscar in one book; the original response-time analysis is Joseph and Pandya (1986) and Audsley et al. (1993); the Stack Resource Policy is Baker (1991). For session types, Honda, Yoshida and Carbone, *Multiparty Asynchronous Session Types* (2008). The Ada Ravenscar profile is section D.13 of the Ada Reference Manual, and the ISO technical report *Guide for the use of the Ada Ravenscar Profile in high integrity systems* explains each restriction's rationale.

**Ideas for your own next example.** Add a `chain` across the `telemetry` Endpoint of Step 5 and watch the Analyzer bound it; move the Step 4 Partition to level B with `overrun = END_JOB` and inject a long Job to see the Job ended and logged; or add a third Partition that talks to the first two with the `Triangle` Protocol from the demo.

**Sources.** Everything in Part II and Part III comes from this workspace as of 2026-11-10: `crates/rsk-gen/src/syntax.rs` (the grammar), `crates/rsk/src/lib.rs` (the API), `crates/rsk-logic/src/logic/log.rs` (event kinds), `xtask/src/main.rs` (commands), `apps/hil-stimulus/` and `apps/p1-demo/` (the applications and their generated summaries), `xtask/fixtures/api-misuse/` (the small declaration and the compile-fail rules), and `target/qemu/logs/p1-demo.log` (the run shown). Part I's description of the project's design is from `.kiro/specs/rsk-ravenscar-profile/design.md` and `docs/profile.md`; the scheduling theory is standard and the references above are its sources.
