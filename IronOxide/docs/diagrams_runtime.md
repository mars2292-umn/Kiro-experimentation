# IronOxide at run time: what the Kernel and the Tasks can do

At run time nothing is created or destroyed. The Kernel releases Jobs of the declared Tasks, starts the one with the highest Priority that the rules allow, and enforces the limits each Partition was given. This page shows who runs where, what a Task may ask of the Kernel, and what happens when something goes wrong.

Terms: a **Task** is declared once; a **Job** is one execution of it; a **Release** makes a Job ready; a **Partition** is a group of Tasks with its own memory and its own fault policy.

## 1. Who runs where

```mermaid
flowchart TB
    subgraph KRN["Kernel: privileged, highest interrupt level, never preempted"]
        SVCH["Supervisor calls from Tasks"]
        TMR["Time-base timer:<br/>releases, Budgets, deadlines"]
        INTK["Interrupt intake:<br/>sporadic releases"]
        FLT["Fault handlers"]
        HM["Health_Monitor:<br/>applies the declared response"]
    end
    subgraph DSP["Dispatch: one interrupt vector per Task, at the Task's level"]
        STUB["Dispatch stub: save the preempted Job,<br/>load the new Partition's memory view"]
    end
    subgraph THR["Thread mode"]
        JOB["Jobs: unprivileged,<br/>own Partition's memory only"]
        IDLE["Idle: Kernel waits for an interrupt"]
    end

    TMR -- "pend the Task's vector" --> STUB
    INTK -- "pend the Task's vector" --> STUB
    SVCH -- "pend the Task's vector" --> STUB
    STUB -- "start the Job" --> JOB
    JOB -- "supervisor call" --> SVCH
    JOB -. "illegal access" .-> FLT
    FLT --> HM
    TMR --> HM
```

## 2. From reset to the first Job

The Kernel refuses to start if the board is not the one the configuration was built for.

```mermaid
flowchart TB
    R["Reset"] --> Z["Zero each region's data"]
    Z --> C["Boot checks:<br/>core revision, MPU regions, priority bits,<br/>clock, FPU, table checksums, vector table"]
    C --> OK{"all pass?"}
    OK -- "no" --> SAFE["Safe state:<br/>nothing enabled, failed check recorded"]
    OK -- "yes" --> CFG["Configure: priorities, MPU on,<br/>FPU eager stacking, watchdog"]
    CFG --> ST["Start: arm the first releases,<br/>start the time base"]
    ST --> RUN["Instant 0: Jobs run"]
```

## 3. The life of a Job

```mermaid
stateDiagram-v2
    [*] --> Idle: declared at build time
    Idle --> Pending: Release (timer or event)
    Pending --> Running: highest Priority above the ceiling and the running Job
    Running --> Preempted: a higher-Priority Job starts
    Preempted --> Running: that Job ends
    Running --> Idle: Job returns
    Running --> Idle: ended by the Health_Monitor
    Pending --> Idle: Release dropped by a Partition stop or restart
```

Three things can release a Job:

| Kind | Trigger | Rule the Kernel enforces |
|---|---|---|
| Periodic | The time-base timer | Released at offset + k × period, computed so it never drifts |
| Sporadic, from an interrupt | A peripheral's interrupt bound to the Task | At most one Release per minimum inter-arrival time; early events are deferred or discarded as declared, and the source is masked meanwhile |
| Sporadic, from a Task | A signal raised, or a message sent to an Endpoint | Same spacing rule |

## 4. Which Job runs

A pending Job starts only when its Priority is above both the Job that is running and the system ceiling. The ceiling rises while any Job holds a shared Resource.

```mermaid
flowchart TB
    P["A Job is pending"] --> A{"Priority above the<br/>running Job?"}
    A -- "no" --> W["Waits"]
    A -- "yes" --> B{"Priority above the<br/>system ceiling?"}
    B -- "no" --> W
    B -- "yes" --> S["Starts now, preempting"]
    W --> E["Re-examined at every Kernel entry"]
```

Consequences that hold for every Application:

- A Job that has started is never blocked: every Resource it will need is free.
- A Job waits for a lower-Priority Job at most once, for at most one Critical_Section.
- There is no deadlock.

## 5. A preemption, step by step

```mermaid
sequenceDiagram
    participant A as Job A (low)
    participant K as Kernel
    participant S as Dispatch stub
    participant B as Job B (high)

    A->>A: executing
    Note over K: timer: B's release is due
    K->>K: mark B pending, charge A's Budget so far
    K->>S: pend B's vector
    S->>S: save A's registers, load B's memory view
    S->>B: start B
    B->>B: executes
    B->>K: return (Job end)
    K->>S: unwind B's level
    S->>S: restore A's registers and memory view
    S->>A: resume A
```

## 6. What a Task can ask the Kernel

A Task has no way to reach the Kernel except these calls. Each is a supervisor call that the Kernel checks before acting.

```mermaid
flowchart LR
    T["Task"]
    T -- "lock, unlock" --> R["A Resource it is declared to access:<br/>raises and restores the ceiling"]
    T -- "now" --> N["The current instant"]
    T -- "raise" --> SG["A signal it is declared to raise:<br/>releases another Task"]
    T -- "send, receive" --> EP["An Endpoint role it owns:<br/>the Kernel copies the message"]
    T -- "DMA request, DMA register write" --> D["An EasyDMA channel of its Partition:<br/>the Kernel validates the buffer first"]
    T -- "return" --> JE["Ends the Job"]
    T -- "panic" --> PN["Reported as a fault of its Partition"]
```

| A Task can | A Task cannot |
|---|---|
| Read and write its own Partition's RAM and the peripherals its Partition owns | Touch another Partition's memory, the Kernel's memory, or a peripheral it does not own |
| Lock a Resource it is declared to access, as often as it likes, one section at a time | Lock a Resource it already holds, or one it is not declared for |
| Run until it returns or its Budget is spent | Wait, sleep or block inside a Critical_Section |
| Raise declared signals and use its declared Endpoint roles | Create Tasks, change Priorities, or mask interrupts |
| Use floating point if declared as an FPU Task | Program a DMA pointer directly: those registers are read-only to it |

## 7. Memory as a running Job sees it

When the Kernel starts a Job it loads that Partition's view into the memory protection unit. Everything outside the view faults.

```mermaid
flowchart LR
    subgraph VIEW["What a Job of Partition 1 can reach"]
        SH["Shared code: read, execute"]
        C1["Partition 1 code: read, execute"]
        R1["Partition 1 RAM: read, write"]
        W1["Partition 1 peripherals"]
    end
    subgraph NO["What faults"]
        KM["Kernel code, data, stack"]
        OP["Other Partitions' code and RAM"]
        KP["Kernel peripherals: timer, clock,<br/>watchdog, interrupt controller"]
        BL["Below its own stack"]
    end
    J["Job of Partition 1"] --> VIEW
    J -. "MemManage or BusFault" .-> NO
```

## 8. What the Kernel enforces on every Job

```mermaid
flowchart TB
    subgraph SPACE["Space"]
        M1["Memory view per Partition"]
        M2["Stack pointer checked at each Kernel entry"]
        M3["DMA buffers validated before a transfer"]
    end
    subgraph TIME["Time"]
        T1["Budget: execution time per Job"]
        T2["Deadline: completion checked at its instant"]
        T3["Minimum inter-arrival time of sporadic sources"]
        T4["Operating duration: safe state at its end"]
    end
    subgraph ORDER["Order"]
        O1["Start rule: Priority above the running Job and the ceiling"]
        O2["Locks released in reverse order"]
        O3["Endpoint protocol: each role's next step"]
    end
```

## 9. When something goes wrong

Every violation becomes an event in the Kernel's log, and the Partition's declared response is applied. The set of responses is closed: there are no others.

```mermaid
flowchart TB
    subgraph EVENTS["Detected by the Kernel"]
        E1["Illegal memory or peripheral access"]
        E2["Budget overrun"]
        E3["Deadline miss"]
        E4["Sporadic source too fast"]
        E5["API misuse, rejected DMA request,<br/>protocol violation, stack violation"]
        E6["Panic in a Task"]
    end
    HM["Health_Monitor:<br/>log the event, look up the<br/>Partition's response"]
    subgraph RESP["Responses"]
        R1["Record only"]
        R2["Record and continue"]
        R3["End the Job"]
        R4["Stop the Partition"]
        R5["Restart the Partition"]
        R6["Safe state for the whole system"]
    end
    E1 --> HM
    E2 --> HM
    E3 --> HM
    E4 --> HM
    E5 --> HM
    E6 --> HM
    HM --> R1
    HM --> R2
    HM --> R3
    HM --> R4
    HM --> R5
    HM --> R6
```

What each response does to the Partition:

| Response | The offending Job | Other Jobs of the Partition | Its Resources and Endpoints | Other Partitions |
|---|---|---|---|---|
| Record only, record and continue | Continues | Unaffected | Unaffected | Unaffected |
| End the Job | Ended; Resources it held are released | Unaffected | Released ones are free | Unaffected |
| Stop the Partition | Ended | Ended; no further releases | Re-initialized; DMA transfers stopped first | Unaffected |
| Restart the Partition | Ended | Ended; releases continue | Re-initialized, RAM zeroed; DMA transfers stopped first | Unaffected |
| Safe state | Ended | Every Job in the system ended | Not used again | Stopped too |

A fault inside the Kernel itself always leads to the safe state.

## 10. What the Kernel can do that a Task cannot

| The Kernel | Why it is acceptable |
|---|---|
| Reads and writes all memory; the memory protection unit does not confine it | Its code is proven, its stack depth is bounded from the binary |
| Masks interrupts while it runs | The longest masked span is measured and charged to every Task's response time |
| Writes the registers of the interrupt controller, the timer, the watchdog and the memory protection unit | These are reserved to it; a Partition cannot be given them |
| Ends Jobs, stops and restarts Partitions | Only as the declared response to a recorded event |
| Copies messages between Partitions | It is the only writer of Endpoint storage |

The Kernel cannot allocate memory, cannot run floating-point arithmetic, and has no call through a function pointer: all three are excluded by build-time checks.

## 11. How Kernel entries queue back to back, and what stays masked

Each Kernel entry is short: 25 to 66 µs in the measured build. The longest time the Kernel runs without returning to a Job is much longer, 330 to 380 µs in three-minute stress runs, because entries run back to back. Sections 11.1 and 11.2 show how the chain forms. Section 11.3 shows what it does and does not block, and what was changed.

### 11.1 Why one entry is followed by another

When an entry returns, the processor does not go back to the Job if something is waiting. It goes straight into the highest pending handler.

```mermaid
flowchart TB
    E["A Kernel entry finishes<br/>(timer, intake, supervisor call, dispatch)"] --> U{"Did it end a Job<br/>that was preempted?"}
    U -- "yes" --> EP["Unwind that Job's level<br/>(inside the same handler)"]
    EP --> P
    U -- "no" --> P{"Is a Kernel-level interrupt<br/>already pending?"}
    P -- "yes: the time base<br/>or an intake" --> NEXT["Run that entry next<br/>(no return to a Job)"]
    NEXT --> E
    P -- "no" --> D{"Is a dispatch pending<br/>that the resumed Job<br/>does not mask?"}
    D -- "yes" --> DSP["Run the dispatch stub next"]
    DSP --> E
    D -- "no" --> END["Return to the Job or to idle:<br/>the masked stretch ends"]
```

The time base is pending again so often because of how long a timer entry takes. The timer is armed for the next due slot when the entry finishes. If that slot comes due within the 45 to 65 µs the entry itself needs, the timer has already fired by the time the entry returns.

### 11.2 The longest stretch measured

One stretch of 382 µs from a stress run (measured build, 64 MHz). Nine entries ran without a return to a Job. The timer was pending again after each of the first four timer-related entries.

```mermaid
sequenceDiagram
    participant J as Jobs (Thread mode)
    participant K as Kernel level
    participant D as Dispatch stubs

    J->>K: supervisor call "now" (7 µs)
    Note over K: time base pending
    K->>K: timer entry (45 µs)
    Note over K: time base pending again
    K->>K: timer entry: Budget overrun, Job ended (57 µs)
    K->>K: unwind the ended Job's level (8 µs)
    Note over K: time base pending again
    K->>K: timer entry (42 µs)
    Note over K: an intake interrupt pending
    K->>K: intake entry (38 µs)
    K->>D: dispatch pending
    D->>D: dispatch: start a Job (47 µs)
    Note over K: time base pending again
    D->>K: timer entry (64 µs)
    K->>D: dispatch pending
    D->>D: dispatch: start a Job (51 µs)
    D->>J: the new Job runs: the stretch ends
    Note over J,D: 358 µs in the entries, 24 µs between them
```

Where the 382 µs went:

| Part of the stretch | Entries | Time |
|---|---|---|
| Timer entries | 4 | 208 µs |
| Dispatches | 2 | 97 µs |
| Intake | 1 | 38 µs |
| Unwind | 1 | 8 µs |
| Supervisor call | 1 | 7 µs |
| Between entries (stubs, exception entry) | | 24 µs |

### 11.3 What the chain does and does not block (measured 2026-11-21)

The chain is how long the Kernel can run back to back. It is not how long an interrupt can be kept waiting. At every arrow below the processor picks the highest pending exception, so a Kernel-level interrupt that arrived during one entry is served at that entry's return, ahead of any queued dispatch. What cannot be interrupted is one handler together with the unwind it runs before it returns: the **hold**.

```mermaid
flowchart LR
    subgraph CH["The chain: about 330 to 380 µs of Kernel entries back to back"]
        direction LR
        H1["timer"] --> H2["timer + unwind"] --> H3["timer"] --> H4["intake"] --> H5["dispatch"] --> H6["timer"] --> H7["dispatch"]
    end
    subgraph HO["One hold: at most about 100 µs"]
        direction LR
        G1["timer entry that ends a Job<br/>and restarts its Partition"] --> G2["unwind of the ended Job"]
    end
    P(["a pending Kernel-level interrupt<br/>is taken at each arrow of the chain"]) -.-> CH
    Q(["nothing is taken<br/>inside a hold"]) -.-> HO
```

The Kernel now records both. Three-minute stress runs on the board:

| | Chain (entries back to back) | Hold (cannot be interrupted) |
|---|---|---|
| Measured build | 332 µs | 99 µs |
| Instrumented build | 327 to 328 µs | 107 to 109 µs |

The longest hold in every run was a timer entry that ended an overrunning Job and restarted its Partition, followed by that Job's unwind.

The analysis charges every Kernel entry to the event that caused it: each Task's releases, deadlines, Budget events and interrupts, including those of lower-Priority Tasks. Its blocking term, L_kernel, only has to cover the one section already in progress when an event arrives. L_kernel was the chain from 2026-11-07; it is the hold since 2026-11-21, because the chain's entries were being counted twice.

Two changes were tried on a branch:

| Change | Result | Kept |
|---|---|---|
| Let one timer entry also serve the slots that come due while it runs | The timer entry grew from 76 to 110 µs at most and the hold from 108 to 131 µs; the chain did not get shorter | No |
| Record the hold and use it as L_kernel | L_kernel's measured basis fell from about 330 µs to about 100 µs | Yes |

Running the unwind at the ended Job's own level was also considered. It would take the unwind (8 to 34 µs) out of the hold.
