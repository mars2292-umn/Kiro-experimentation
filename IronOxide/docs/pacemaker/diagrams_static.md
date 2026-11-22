# PACEMAKER on rsk: what the system is made of

The pulse generator (PG) is an rsk Application of three Partitions and six Tasks. This page shows the boards around it, what each Partition owns, how the hardware is wired inside the chip, and which crate holds what. `diagrams_dynamic.md` shows the same system in motion.

Status: these diagrams draw `design.md` Draft 0.2. Nothing is built or measured. Every period, MIT, deadline and Budget is a proposal, and the nRF52840 register behaviour is still to be confirmed by the spike.

Terms: a **Pace** is a pulse the PG delivers; a **Sense** is a heartbeat the PG detects; a **Chamber** is the atrium (A) or the ventricle (V); the **step function** is the pure function that decides the therapy.

## 1. The system and what surrounds it

Nothing here is a real heart or a real programmer. The Heart_Simulator produces sense edges and receives pace pulses; the DCM_Stub stands for the clinician's programmer.

```mermaid
flowchart LR
    subgraph HOST["Host computer"]
        DCM["DCM_Stub<br/>commands, parameters,<br/>simulated battery and activity"]
        JUDGE["Rig judgement<br/>every interval and width<br/>against its tolerance"]
    end
    subgraph BC["Board C"]
        BR["Bridge<br/>host to SPI"]
    end
    subgraph BA["Board A: the pulse generator"]
        PG["PACEMAKER Application<br/>on rsk"]
    end
    subgraph BB["Board B"]
        HS["Heart_Simulator<br/>plays a scenario table"]
    end
    LA["Logic analyzer"]

    DCM -- "frames" --> BR
    BR -- "SPI, polled by the PG" --> PG
    HS -- "A sense line, V sense line" --> PG
    PG -- "A pace line, V pace line" --> HS
    PG -. "pace pins" .-> LA
    HS -. "sense mirrors" .-> LA
    LA -- "captured edges" --> JUDGE
    MAG["Magnet pin"] --> PG
```

| Part | Where | Role |
|---|---|---|
| PG | Board A | The system under test |
| Heart_Simulator | Board B | Sense edges in, pace pulses back; restarts a Chamber's intrinsic timer on a Pace in the responsive scenarios |
| Bridge | Board C | Carries link frames between the host and the PG's SPI |
| DCM_Stub | Host | Programs the PG, reads status and records, sets the simulated battery level and activity value |
| Logic analyzer | Rig | The only source of timing verdicts |

The wiring is a proposal until it is measured and recorded in `hil-rig.toml` (PD-O4).

## 2. Partitions, Tasks and what each owns

A higher box can lose nothing to a lower one: the lower Partitions reach `therapy` only through Endpoints.

```mermaid
flowchart TB
    subgraph TH["Partition therapy: level A"]
        direction TB
        T1["pace<br/>sporadic, Priority 7"]
        T2["sense_v<br/>sporadic, Priority 6"]
        T3["sense_a<br/>sporadic, Priority 5"]
        T4["control<br/>periodic 20 ms, Priority 4"]
        RES[("Resource timing<br/>step state, parameters in force,<br/>State, event ring")]
        HW["Peripherals: GPIOTE, P0,<br/>TIMER1, TIMER2, TIMER3"]
        T1 --- RES
        T2 --- RES
        T3 --- RES
        T4 --- RES
    end
    subgraph CO["Partition comms: level C"]
        T5["link<br/>periodic 50 ms, Priority 2"]
        HW2["Peripheral: SPIM1"]
    end
    subgraph DI["Partition diag: level D"]
        T6["history<br/>periodic 100 ms, Priority 1"]
        D2["Histograms, trends,<br/>implant data, Marker buffer"]
    end
```

| Task | Partition | Released by | Period or MIT | Deadline | Priority |
|---|---|---|---|---|---|
| `pace` | therapy | TIMER3 compare 4 or 5, the finish of a pulse | MIT 20 ms | 2 ms | 7 |
| `sense_v` | therapy | TIMER2 compare | MIT 50 ms | 2 ms | 6 |
| `sense_a` | therapy | TIMER1 compare | MIT 50 ms | 2 ms | 5 |
| `control` | therapy | The time base | 20 ms | 20 ms | 4 |
| `link` | comms | The time base | 50 ms | 50 ms | 2 |
| `history` | diag | The time base | 100 ms | 100 ms | 1 |

Six Tasks of the seven the Target allows. All four `therapy` Tasks access `timing`, so its Ceiling is 7: no Task preempts a Critical_Section on it.

## 3. Hardware inside the chip: who makes each edge

Every edge on a pace pin is made by a timer compare, and every Sense instant is taken by a timer capture. The dashed arrows are the only places where software is involved.

```mermaid
flowchart LR
    subgraph IN["Sense inputs"]
        SA["A sense line"]
        SV["V sense line"]
    end
    subgraph GI["GPIOTE inputs"]
        I0["IN 0"]
        I1["IN 1"]
    end
    subgraph T3["TIMER3: 1 MHz, 32 bits, free-running"]
        C2["capture 2<br/>instant of the A Sense"]
        C3["capture 3<br/>instant of the V Sense"]
        C0["compare 0<br/>A Pace starts"]
        C1["compare 1<br/>V Pace starts"]
        C4["compare 4<br/>A pulse finishes"]
        C5["compare 5<br/>V pulse finishes"]
    end
    T1["TIMER1 as a counter<br/>counts A edges"]
    T2["TIMER2 as a counter<br/>counts V edges"]
    subgraph GO["GPIOTE outputs"]
        S2["set, clear: A pace pin"]
        S3["set, clear: V pace pin"]
    end

    SA --> I0
    SV --> I1
    I0 -- "PPI" --> C2
    I0 -- "fork" --> T1
    I1 -- "PPI" --> C3
    I1 -- "fork" --> T2
    C0 -- "PPI: set" --> S2
    C4 -- "PPI: clear" --> S2
    C1 -- "PPI: set" --> S3
    C5 -- "PPI: clear" --> S3

    T1 -. "interrupt" .-> JA["Task sense_a"]
    T2 -. "interrupt" .-> JV["Task sense_v"]
    C4 -. "interrupt" .-> JP["Task pace"]
    C5 -. "interrupt" .-> JP
```

Six PPI channels, two with a fork. The Kernel writes them once at Init from the generated configuration.

| What must be exact | What makes it so |
|---|---|
| Pulse width | Compare 4 minus compare 0 (or 5 minus 1), written together by one Job while the start is disarmed: exact to 1 µs |
| Interval between two Paces | A difference of two compare values |
| Interval from a Sense to a Pace | A capture and a compare of the same timer |
| The time of a Marker | The capture or compare value, whenever its Job runs |

Compares 0 and 1 and the captures raise no interrupt: `pace` is released when a pulse has finished, so it never runs while the pin is high. Each sense line has its own counter because GPIOTE has one interrupt for all its channels, and rsk masks an interrupt line for one MIT after each event: on a shared line a Sense in one Chamber would hold back the other.

## 4. Who owns which peripheral

```mermaid
flowchart TB
    subgraph K["Kernel: reserved, no Partition can be given them"]
        K1["TIMER0: the time base"]
        K2["CLOCK, watchdog"]
        K3["PPI: the six fixed channels,<br/>written at Init only"]
        K4["Interrupt controller,<br/>memory protection unit"]
    end
    subgraph T["therapy"]
        P1["TIMER3: therapy timer"]
        P2["TIMER1, TIMER2: edge counters"]
        P3["GPIOTE: pin events and tasks"]
        P4["P0: every pin's configuration"]
    end
    subgraph C["comms"]
        P5["SPIM1: the link<br/>transfers go through the Kernel"]
    end
    subgraph D["diag"]
        P6["none"]
    end
    P4 -. "configures the SPI pins for" .-> P5
```

`therapy` uses five peripheral windows, the most a Partition can have. P0 has one owner, so `comms` depends on `therapy` to configure its SPI pins: a finding about rsk, recorded in the design.

## 5. Endpoints: the only paths between Partitions

Two of the five are validated, the two that go upward in criticality.

```mermaid
flowchart LR
    subgraph TH["therapy: level A"]
        CTL["control"]
    end
    subgraph CO["comms: level C"]
        LNK["link"]
    end
    subgraph DI["diag: level D"]
        HIS["history"]
    end

    LNK == "program: command and parameter set<br/>VALIDATED by therapy" ==> CTL
    CTL -- "status: State, Mode, battery,<br/>answer to the last command" --> LNK
    CTL -- "events: up to 8 records" --> HIS
    HIS == "records: one histogram, trend or Marker frame<br/>VALIDATED by comms" ==> LNK
    LNK -- "request: a record request or a reset" --> HIS
```

| Endpoint | From | To | Validated | Releases a Task |
|---|---|---|---|---|
| `program` | `comms.link` | `therapy.control` | Yes: exactly the admitted parameter values, limits and command codes | No |
| `status` | `therapy.control` | `comms.link` | Not needed | No |
| `events` | `therapy.control` | `diag.history` | Not needed | No |
| `records` | `diag.history` | `comms.link` | Yes: frame kind and length | No |
| `request` | `comms.link` | `diag.history` | Not needed | No |

No Endpoint releases a Task: every receiver is periodic and polls. A flood from a lower Partition therefore costs the receiver nothing beyond its own Budget. `pace`, `sense_v` and `sense_a` never touch an Endpoint.

## 6. Inside the therapy Partition

The Tasks are thin. Each reads the hardware, calls the step function, and writes the hardware.

```mermaid
flowchart TB
    subgraph TASKS["Tasks"]
        PA["pace"]
        SE["sense_a, sense_v"]
        CT["control"]
    end
    subgraph CORE["pm-core: no rsk, no hardware"]
        STEP["step<br/>parameters, state, one event<br/>gives new state and arming"]
        VAL["validate<br/>words to a parameter set, or nothing"]
    end
    subgraph HWL["Hw layer: one trait, two implementations"]
        NRF["nRF52840<br/>TIMER3, counters, GPIOTE"]
        QEMU["QEMU<br/>one timer, scripted Senses"]
    end
    ST[("timing<br/>state, parameters, State, ring")]

    PA -- "fired" --> HWL
    SE -- "sensed" --> HWL
    PA -- "Paced event" --> STEP
    SE -- "Sense event" --> STEP
    CT -- "Command, Tick events" --> STEP
    STEP -- "arm, disarm" --> HWL
    STEP -- "Markers" --> ST
    CT -- "validation function<br/>of the program Endpoint" --> VAL
    TASKS --- ST
```

The four events the step function accepts:

| Event | Given by | Carries |
|---|---|---|
| `Sense` | `sense_a`, `sense_v` | Chamber, instant, number of edges |
| `Paced` | `pace` | Chamber, instant |
| `Tick` | `control`, every 20 ms | Present instant, activity, magnet, battery level, link state |
| `Command` | `control` | A validated command |

## 7. Crates and who depends on whom

`pm-core` is the only crate that knows the therapy. It is the same code on the board, in QEMU, in the DCM_Stub and in host tests.

```mermaid
flowchart TB
    CORE["crates/pm-core<br/>step, parameters, validation"]
    subgraph APP["apps/pacemaker"]
        DECL["decl<br/>App_Declaration, one variant per Target"]
        TH["therapy<br/>four Tasks, Hw layer"]
        CO["comms<br/>Task link"]
        DI["diag<br/>Task history"]
        SYS["system, bin<br/>generated system crate, binary"]
        GEN["gen<br/>generated configuration,<br/>Task_Model, linker script"]
    end
    HS["apps/heart-sim<br/>rsk Application for board B"]
    STUB["tools/dcm-stub<br/>host program"]
    TESTS["Host tests<br/>and App_Prover"]
    RSK["rsk<br/>Task API"]

    TH -- "step, validate" --> CORE
    CO -- "frame encoding" --> CORE
    STUB -- "same types and validation" --> CORE
    TESTS --> CORE
    DECL --> GEN
    GEN --> TH
    GEN --> CO
    GEN --> DI
    GEN --> SYS
    TH --> RSK
    CO --> RSK
    DI --> RSK
    HS --> RSK
```

## 8. What changes in rsk

Two additions, and nothing else: no Kernel one-shot event, no new EasyDMA class.

```mermaid
flowchart LR
    subgraph NEW["Added for this Application"]
        N1["Fixed PPI channels in the declaration:<br/>event, task, optional fork"]
        N2["TIMER3 in the Target table<br/>as a Partition-ownable peripheral"]
    end
    N1 --> G["Generator:<br/>channel table in the generated configuration<br/>and in the Task_Model"]
    G --> K["Kernel: writes the channels at Init,<br/>never again"]
    G --> CC["Config_Checker:<br/>recomputes the channels<br/>from the Task_Model"]
    N2 --> G2["Generator: accepts TIMER3<br/>as a Partition's peripheral"]
```

A fixed channel may connect only peripherals of one Partition.

## 9. What each kind of evidence covers

```mermaid
flowchart TB
    subgraph WHAT["Claims"]
        A["The step function's properties:<br/>URL respected, a Pace always pending,<br/>refractory Senses change nothing"]
        B["validate accepts exactly<br/>the admitted sets"]
        C["Arming: the pulse finish is start plus width,<br/>the start is ahead of the counter"]
        D["No panic, overflow or<br/>out-of-bounds index in therapy"]
        E["therapy meets its deadlines<br/>whatever comms and diag do"]
        F["Intervals and widths on the pins<br/>are within tolerance"]
    end
    HT["Host tests<br/>property-based"]
    AP["App_Prover<br/>contracts"]
    AN["Analyzer"]
    RIG["Rig and logic analyzer"]

    A --> HT
    A --> AP
    B --> HT
    B --> AP
    C --> AP
    C --> RIG
    D --> AP
    E --> AN
    E --> RIG
    F --> RIG
```

The provers and host tests cover the decision; only the rig covers the pins.
