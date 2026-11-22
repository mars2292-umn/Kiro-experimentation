# PACEMAKER on rsk: what happens at run time

This page follows the pulse generator (PG) through a beat, a Sense, a programming command and a fault. `diagrams_static.md` shows the parts these diagrams refer to.

Status: these diagrams draw `design.md` Draft 0.2. Nothing is built or measured. Every time shown is a proposal, and the nRF52840 register behaviour is still to be confirmed by the spike.

Terms: a **Pace** is a pulse the PG delivers; a **Sense** is a heartbeat the PG detects; to **arm** a Pace is to write the timer compares that will produce it; the **escape interval** E is the longest time the PG waits for a heartbeat before it paces.

The rule behind every diagram: **hardware makes each edge, a Job only arms it.** A Job has to finish before the instant it arms, and nothing more is asked of scheduling.

## 1. From reset to the first Pace

```mermaid
flowchart TB
    R["Reset"] --> B["rsk boot checks"]
    B --> P["Kernel writes the six PPI channels<br/>from the generated configuration"]
    P --> Z["Instant 0: the statics of therapy are zero,<br/>which is the Power-On Reset State"]
    Z --> C["First Job of control"]
    C --> C1["Configure every pin and the GPIOTE channels"]
    C1 --> C2["Start TIMER3, TIMER1, TIMER2"]
    C2 --> C3["Measure the offset between<br/>the Kernel time base and TIMER3"]
    C3 --> OK{"two readings close enough,<br/>within eight tries?"}
    OK -- "no" --> F["Fault of therapy:<br/>restart and try again"]
    OK -- "yes" --> A["Arm the first ventricular Pace:<br/>VVI at 65 ppm"]
    A --> RUN["TIMER3 delivers it, and pace is released"]
```

## 2. A paced beat

The Mode is VVI and no heartbeat arrives. The whole pulse is on the pin before any Job runs: the Job is released by the pulse's finish.

```mermaid
sequenceDiagram
    participant T as TIMER3
    participant X as V pace pin
    participant K as Kernel
    participant P as Job of pace
    participant S as step function

    Note over T: counter reaches compare 1
    T->>X: pin high, through PPI
    Note over T: start flag set, no interrupt
    Note over T: counter reaches compare 5
    T->>X: pin low, through PPI
    Note over X: pulse width exact to 1 µs
    T->>K: interrupt of compare 5
    K->>P: release, then dispatch
    Note over K: the line stays masked for 20 ms
    P->>P: lock timing
    P->>T: read and clear the start flags
    P->>S: Paced, ventricle, at v
    S-->>P: next Pace at v + E, Marker VP
    P->>T: arm from the state (section 4)
    P->>P: push the Marker to the ring, unlock
    P->>K: return
```

Because `pace` is released by the finish of the pulse, it cannot rewrite the finish compare while the pin is high. Any other Job that wants to arm this Chamber between the start and this Job finds the start flag set and writes nothing (section 4).

## 3. A sensed beat that withdraws the Pace

The heart beats in time, so the pending Pace must not be delivered.

```mermaid
sequenceDiagram
    participant H as Heart_Simulator
    participant G as GPIOTE input 1
    participant T as TIMER3
    participant C as TIMER2 counter
    participant K as Kernel
    participant J as Job of sense_v
    participant S as step function

    H->>G: edge on the V sense line
    G->>T: capture 3 takes the instant s
    G->>C: count one edge, by the fork
    C->>K: interrupt, count reached its compare
    K->>J: release, then dispatch
    Note over K: the line stays masked for 50 ms
    J->>J: lock timing
    J->>C: read the count, move the compare one edge on
    J->>T: read capture 3
    J->>S: Sense, ventricle, at s, 1 edge
    S-->>J: not refractory, so next Pace at s + E, Marker VS
    J->>T: disarm compare 1, check its flag: not fired
    J->>T: write compare 5, then compare 1, check the lead
    J->>J: push the Marker, unlock
    J->>K: return
```

The counter is never cleared, so no edge is lost between a read and a clear.

## 4. Writing a Chamber's compares, and the lead

Two things can go wrong with a compare. Written with a value the counter has already passed, it does not fire until the counter wraps, 71.6 minutes later. And a finish compare rewritten under a pulse leaves the pin high. One sequence, used to arm, move and withdraw, prevents both.

```mermaid
flowchart TB
    D["Disarm the start compare"] --> F{"start flag set:<br/>did a Pace of this Chamber fire?"}
    F -- "yes" --> TL["Write nothing more.<br/>The pulse keeps its own finish.<br/>pace arms from the state afterwards"]
    F -- "no" --> W{"withdrawing?"}
    W -- "yes" --> DONE["Done: nothing armed"]
    W -- "no" --> A["Write the finish compare,<br/>then the start compare"]
    A --> N["Read the present"]
    N --> Q{"start at least one LEAD<br/>ahead of the present?"}
    Q -- "yes" --> OK["Armed"]
    Q -- "no" --> R{"third attempt?"}
    R -- "no" --> M["Set the start to present plus LEAD"]
    M --> D
    R -- "yes" --> L["Write a fresh finish compare,<br/>raise the pin by a direct write,<br/>record a late-arm event"]
```

The finish compare is written only while the start is disarmed and no Pace of the Chamber is waiting for its `pace` Job. So a start that fires always has its own finish in force.

`LEAD` is proposed at 500 µs. It has to cover what can interrupt the Job between the check and the compare taking effect. The last box should never be reached; it exists because the alternative is a missed Pace.

## 5. The life of one armed Pace

```mermaid
stateDiagram-v2
    [*] --> NotArmed
    NotArmed --> Armed: a Job writes start and finish
    Armed --> Armed: a Job moves it to a new instant
    Armed --> NotArmed: a Sense withdraws it in time
    Armed --> PinHigh: counter reaches the start
    PinHigh --> Delivered: counter reaches the finish
    Delivered --> Armed: pace is released, runs the step and arms the next
    Armed --> Fused: a Sense came too late to withdraw it
    Fused --> Delivered: both events recorded, Sense first
```

`PinHigh` to `Delivered` needs no software, so a pin cannot stay high across a restart of the Partition, a stop, or the safe state.

## 6. The inhibition window

A Sense withdraws a Pace only when its Job runs. A Sense that comes less than the sense Task's response time before the Pace is too late.

```mermaid
sequenceDiagram
    participant H as Heart_Simulator
    participant T as TIMER3
    participant X as V pace pin
    participant J as Job of sense_v
    participant P as Job of pace
    participant S as step function

    H->>T: Sense captured at s
    Note over T: the Pace is due at s plus 1 ms
    T->>X: pin high: the Pace is delivered
    J->>T: disarm the start, check its flag
    T-->>J: too late: the flag is set
    J->>S: Sense at s
    Note over X: pin low: pace is released
    P->>S: Paced at s plus 1 ms
    S-->>P: fusion noted in the Marker
    Note over S: next interval starts from the Pace
```

The window is a stated parameter, proposed at 2 ms, and the rig measures it by sweeping a Sense toward a Pace. It is a bounded deviation from the source, which has a Sense always cancel the pending Pace.

## 7. A dual-chamber cycle

DDD, with an atrial heartbeat the PG tracks and no ventricular heartbeat. Timing is ventricular-based: every escape interval starts at a ventricular event.

```mermaid
sequenceDiagram
    participant V as Ventricle
    participant A as Atrium
    participant S as step function
    participant T as TIMER3 compares

    V->>S: ventricular event at v
    S->>T: arm V Pace at v + E
    S->>T: arm A Pace at v + E - AV
    Note over S: VRP and PVARP start
    A->>S: atrial Sense at a, after PVARP
    S->>T: withdraw the A Pace
    S->>T: move the V Pace to the later of a + sensed AV delay and v + URL interval
    Note over T: no ventricular Sense arrives
    T->>V: V Pace delivered
    V->>S: ventricular event: the next cycle starts
```

What a non-refractory event does, by the Mode's response letter:

| Event | Inhibited (I) | Tracked (D) | Triggered (T) |
|---|---|---|---|
| Atrial Sense | The atrial Pace is withdrawn | The atrial Pace is withdrawn and the ventricular Pace follows after the sensed AV delay, never above the URL | An atrial Pace one `LEAD` from now |
| Ventricular Sense | The ventricular Pace is withdrawn and a new cycle starts | The same | A ventricular Pace one `LEAD` from now |
| Any Sense in a refractory or blanking period | Nothing is armed or withdrawn | The same | The same |

## 8. A Sense, by where it falls in the cycle

```mermaid
flowchart TB
    E["An edge on a sense line"] --> B{"in the Chamber's<br/>blanking period?"}
    B -- "yes" --> X["Ignored: no Marker"]
    B -- "no" --> R{"in its refractory period?"}
    R -- "yes" --> RS["Refractory-sense Marker,<br/>pending Paces unchanged"]
    R -- "no" --> N{"cycle marked noisy?"}
    N -- "yes" --> AS["Pending Pace left armed:<br/>asynchronous pacing"]
    N -- "no" --> M["Acted on by the Mode's response:<br/>inhibit, track or trigger"]
```

## 9. A sense storm

After each delivered event rsk masks that line for one MIT. Edges that arrive meanwhile are counted by the hardware and cost no Kernel entry.

```mermaid
sequenceDiagram
    participant H as Noisy lead
    participant C as TIMER2 counter
    participant K as Kernel
    participant J as Job of sense_v
    participant S as step function

    H->>C: edge
    C->>K: interrupt
    K->>J: release
    Note over K: line masked for 50 ms
    J->>S: Sense, 1 edge
    H->>C: edge
    H->>C: edge
    H->>C: edge
    Note over C: counted and latched, nothing delivered
    Note over K: 50 ms passed: line unmasked
    C->>K: one interrupt
    K->>J: release
    J->>S: Sense, 3 edges
    Note over S: cycle marked noisy, the pending Pace stays armed
```

Masked edges are not MIT violations, so a storm does not approach the Partition's violation threshold and cannot restart it. A late Sense within the 50 ms changes no Pace either, because every Mode's refractory period after an event is at least 150 ms. The rig's storm scenario is the demonstration.

## 10. The therapy States

Exactly one State holds at a time.

```mermaid
stateDiagram-v2
    [*] --> POR: reset or restart of therapy
    POR --> Permanent: a parameter set is accepted
    Permanent --> Temporary: temporary pacing starts, link up
    Temporary --> Permanent: temporary pacing over, or link lost
    Permanent --> Magnet: magnet on, response enabled
    Magnet --> Permanent: magnet off, back to the State before
    POR --> PaceNow: Pace-Now command
    Permanent --> PaceNow: Pace-Now command
    Temporary --> PaceNow: Pace-Now command
    PaceNow --> Permanent: a Mode is programmed
    Permanent --> POR: restart of therapy
```

A restart of the Partition enters POR from any State; the diagram draws it from Permanent only. Every transition except into Pace-Now takes effect at the next ventricular event (atrial in the atrial-only Modes). The Magnet State returns to whichever State it interrupted.

## 11. Programming a parameter set

```mermaid
sequenceDiagram
    participant D as DCM_Stub
    participant L as link (comms)
    participant K as Kernel
    participant C as control (therapy)
    participant S as step function

    D->>L: parameter set in numbered parts
    Note over L: polls SPIM1 every 50 ms
    L->>L: assemble until complete
    L->>K: send on program
    K->>K: run therapy's validation function
    alt every value admitted
        K-->>L: accepted
        Note over C: polls program every 20 ms
        C->>K: receive
        C->>S: Command
        Note over S: in force at the next ventricular event
        C->>K: send on status
    else a value outside the tables
        K-->>L: refused, reported against comms
        Note over C: parameters in force unchanged
    end
    L->>K: receive on status
    L->>D: status frame
```

The DCM_Stub holds the same validation function, so a test can assert that the PG answered as predicted.

## 12. Where the time goes for a Pace-Now command

```mermaid
flowchart LR
    F["Frame sent<br/>by the DCM_Stub"] -- "at most one link period:<br/>50 ms" --> L["link forwards it<br/>on program"]
    L -- "at most one control period:<br/>20 ms" --> C["control gives the<br/>Command to the step"]
    C -- "at most one interval at 65 ppm:<br/>923 ms" --> P["Ventricular Pace<br/>on the pin"]
```

The other slow paths have the same shape:

| Change | Path | Bound with the proposals |
|---|---|---|
| Link loss | Three unanswered polls, then one `control` period | 170 ms |
| Magnet, battery level, a State change | One `control` period, then the next ventricular event | 20 ms plus one cycle |

## 13. Events on their way to the programmer

No therapy Task waits for a lower Partition. The ring absorbs what `history` has not yet taken.

```mermaid
flowchart LR
    J["pace, sense_a, sense_v:<br/>push a record inside<br/>the Critical_Section"] --> RG[("Ring in timing<br/>32 records")]
    RG -- "control, every 20 ms:<br/>up to 8 records" --> EV["Endpoint events"]
    EV -- "history, every 100 ms" --> H["Histograms, trends,<br/>Marker buffer"]
    H -- "one frame on request" --> RC["Endpoint records<br/>validated by comms"]
    RC -- "link, every 50 ms" --> D["DCM_Stub"]
    RG -. "ring full: new records<br/>dropped and counted" .-> DR["Drop count,<br/>reported, never silent"]
```

## 14. When something goes wrong

```mermaid
flowchart TB
    subgraph EV["What happens"]
        E1["Fault, overrun or deadline miss<br/>in therapy"]
        E2["Fault, overrun or flood<br/>in comms or diag"]
        E3["A parameter set outside the tables"]
        E4["Three unanswered polls"]
        E5["A late arm, third attempt"]
        E6["Kernel safe state"]
    end
    R1["Restart of therapy:<br/>the Power-On Reset State"]
    R2["Their own responses:<br/>therapy is unaffected"]
    R3["Not delivered:<br/>parameters in force unchanged"]
    R4["Temporary State and any test are over:<br/>pacing as before"]
    R5["The Job raises the pin itself:<br/>counted"]
    R6["No pacing:<br/>pins low after any armed finish"]
    E1 --> R1
    E2 --> R2
    E3 --> R3
    E4 --> R4
    E5 --> R5
    E6 --> R6
```

## 15. A restart of the therapy Partition

```mermaid
sequenceDiagram
    participant J as A Job of therapy
    participant K as Kernel
    participant T as TIMER3
    participant X as Pace pin
    participant C as control

    J->>K: fault, overrun or deadline miss
    K->>K: Health_Monitor logs it
    K->>K: stop the Jobs of therapy, zero its RAM
    Note over T: an armed finish compare still fires
    T->>X: pin low
    Note over K: releases continue
    K->>C: next periodic release
    C->>C: statics are zero: this is the first Job
    C->>T: configure, start, measure the offset
    C->>T: arm a ventricular Pace: VVI at 65 ppm
    T->>X: Pace
    Note over X: the pause in pacing is measured on the rig
```

Zeroed statics are the Power-On Reset State, so a restart of the Partition is a power-on reset of the therapy and needs no code of its own.
