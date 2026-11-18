# IronOxide before run time: components and the data between them

Everything about an Application is fixed before it runs: the Tasks, their Priorities, the Resources they share, the memory each Partition gets. This page shows which component produces what, and what data passes between them, from the declaration to a binary that is allowed to fly.

The diagrams use the names of the repository: crates under `crates/`, the demo Application under `apps/p1-demo/`.

## 1. The whole pipeline

One declaration is the source of every artefact. Nothing downstream is written by hand twice.

```mermaid
flowchart LR
    DECL["App_Declaration<br/>rsk::app! in the decl crate"]
    PARTS["Partition crates<br/>Task bodies"]
    GEN["Generator<br/>rsk-gen"]
    BUILD["Compiler and linker"]
    ELF["Linked binary<br/>ELF"]
    CHECK["Build checks<br/>Profile_Lint, Link_Checker,<br/>Config_Checker"]
    RIG["HIL rig<br/>rsk-hil"]
    AN["Analyzer<br/>rsk-analyzer"]
    EV[("Evidence store")]

    DECL --> GEN
    GEN -- "generated code,<br/>linker script" --> BUILD
    PARTS --> BUILD
    BUILD --> ELF
    GEN -- "Task_Model" --> CHECK
    ELF --> CHECK
    ELF --> RIG
    RIG -- "measured timing" --> AN
    GEN -- "Task_Model" --> AN
    CHECK --> EV
    AN --> EV
    RIG --> EV
```

## 2. What the Generator reads and writes

The Generator validates the declaration, derives every value that must not be maintained by hand, and writes one directory per configuration (for example `apps/p1-demo/gen-nrf52840/`).

```mermaid
flowchart TB
    DECL["App_Declaration"]
    TGT["Target table<br/>memory sizes, interrupt lines,<br/>peripherals"]

    subgraph GEN["Generator"]
        VAL["Validate<br/>every item in one Partition,<br/>names, Priorities, bounds"]
        DER["Derive<br/>Ceilings, lock depth, capacities,<br/>priority map, dispatch vectors"]
        LAY["Lay out memory<br/>power-of-two regions,<br/>largest first"]
    end

    DECL --> VAL
    TGT --> VAL
    VAL --> DER --> LAY

    LAY --> PG["partition_NAME.rs<br/>typed handles for each Partition:<br/>Resources, Signals, Endpoints, layout"]
    LAY --> SM["system_manifest.rs<br/>the Kernel's constant tables"]
    LAY --> LX["link.x, memory.x<br/>where every section goes"]
    LAY --> TM["task_model.json<br/>declared and derived values"]
    LAY --> SUM["summary.txt<br/>the configuration, for review"]
```

| Output | Who uses it | What it carries |
|---|---|---|
| `partition_NAME.rs` | The Partition's crate | The only handles a Task can use: its own Resources, Signals, Endpoint roles, EasyDMA channels |
| `system_manifest.rs` | The Kernel | Task table, Ceilings, Budgets, timer slots, MPU view per Partition, fault responses, a checksum |
| `link.x`, `memory.x` | The linker | One code and one RAM region per Partition, the Kernel's regions, the shared code |
| `task_model.json` | Config_Checker, Analyzer, Link_Checker, rig | Everything declared and derived, marked which is which |

## 3. From source to binary

Each Partition is its own crate, so the linker can place its code and data in that Partition's regions by archive name.

```mermaid
flowchart LR
    subgraph SRC["Source"]
        P0["Partition crates"]
        SYS["System crate<br/>includes system_manifest.rs"]
        K["Kernel<br/>rsk-kernel, rsk-logic, rsk-entry"]
        API["Task API<br/>rsk"]
    end
    LINT["Profile_Lint<br/>rsk-lint"]
    CC["Compiler"]
    LD["Linker<br/>with link.x"]
    ELF["ELF"]

    P0 -. "source rules:<br/>no heap, bounded loops,<br/>no function pointers" .-> LINT
    P0 --> CC
    SYS --> CC
    K --> CC
    API --> CC
    CC --> LD --> ELF
```

What ends up where in the binary:

```mermaid
flowchart TB
    subgraph FLASH["Flash"]
        KF["Kernel code and tables"]
        SH["Shared code<br/>core, rsk API"]
        PC0["Partition 0 code"]
        PC1["Partition 1 code"]
        PC2["Partition 2 code"]
    end
    subgraph RAM["RAM"]
        KR["Kernel state and stack"]
        PR0["Partition 0: stack, then data"]
        PR1["Partition 1: stack, then data"]
        PR2["Partition 2: stack, then data"]
    end
```

## 4. Checks on the linked binary

Two independent tools read the binary. Neither trusts the Generator: each recomputes what it checks.

```mermaid
flowchart LR
    ELF["ELF"]
    TM["task_model.json"]

    subgraph LK["Link_Checker (rsk-link)"]
        CG["Call graph from the<br/>machine code"]
        ST["Worst-case stack<br/>per Task and per region"]
        SC["Scans: forbidden instructions,<br/>recursion, unresolved calls"]
    end

    subgraph CF["Config_Checker (rsk-confcheck)"]
        RC["Recompute Ceilings, capacities,<br/>priority map, MPU layout"]
        CMP["Compare with the tables<br/>found in the binary"]
    end

    ELF --> CG --> ST
    CG --> SC
    TM --> ST
    ST -- "stack bounds" --> CMP
    ELF --> CMP
    TM --> RC --> CMP

    ST --> OK1{"every function<br/>bounded?"}
    CMP --> OK2{"every value<br/>matches?"}
    OK1 -- "no" --> FAIL["Build fails"]
    OK2 -- "no" --> FAIL
```

## 5. Timing: from measurement to a schedulability verdict

The Analyzer never trusts a number it cannot tie to a specific binary: every record carries the hash of the build it was measured on.

```mermaid
flowchart LR
    ELF["HIL build of the binary"]
    RIG["Rig run on the board"]
    WCET["WCET records<br/>per Task and per Critical_Section"]
    KT["Kernel timing parameters<br/>cost of each Kernel entry,<br/>release jitter, longest masked span"]
    TM["task_model.json"]
    AN["Analyzer"]
    REP["Report<br/>response time per Task<br/>against its deadline"]
    UPP["UPPAAL model<br/>for a cross-check"]

    ELF --> RIG
    RIG --> WCET
    RIG --> KT
    WCET --> AN
    KT --> AN
    TM --> AN
    AN --> REP
    AN --> UPP
```

## 6. Proof tools and what each one reads

```mermaid
flowchart TB
    subgraph CODE["Code"]
        KL["Kernel logic and entry code"]
        UM["Kernel hardware module"]
        DRV["Register drivers"]
        APP["Partition Task bodies"]
        TOOLS["Analyzer, Generator arithmetic,<br/>Config_Checker core, Protocol checker"]
    end

    VERUS["Verus<br/>contracts and invariants"]
    AEN["Aeneas, then Lean<br/>theorems over the extracted code"]
    KANI["Kani<br/>bounded model checking"]
    CRE["Creusot<br/>Task bodies against the API's contracts"]

    KL --> VERUS
    TOOLS --> VERUS
    KL --> AEN
    DRV --> AEN
    TOOLS --> AEN
    UM --> KANI
    DRV --> KANI
    APP --> CRE

    VERUS --> EV[("Evidence store")]
    AEN --> EV
    KANI --> EV
    CRE --> EV
```

## 7. The verification job

`cargo xtask verify` runs every step above on one revision and writes one item per result. Two documents are generated from the store, so they cannot drift from it.

```mermaid
flowchart LR
    JOB["cargo xtask verify"]
    ST[("Evidence store<br/>one item per check, proof,<br/>test and measurement")]
    MAN["Build manifest<br/>hash of every binary and tool"]
    TR["traceability_matrix.md<br/>every acceptance criterion<br/>to its evidence"]
    RQ["requirements_evidence.md<br/>every requirement in prose<br/>with its cited contracts"]

    JOB --> ST
    JOB --> MAN
    ST --> TR
    ST --> RQ
    MAN --> ST
```

## Reading the data flow in one sentence each

- The **declaration** is the only thing an engineer writes about the system's structure.
- The **Generator** turns it into code for the compiler, a script for the linker, and a model for the tools.
- The **Link_Checker** and the **Config_Checker** confirm the binary is what the model says.
- The **rig** measures that binary, and the **Analyzer** decides from the model and the measurements whether every deadline is met.
- The **provers** establish that the Kernel does what its contracts say, whatever the Application.
