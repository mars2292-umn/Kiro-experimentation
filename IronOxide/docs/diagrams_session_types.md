# How global session types work in IronOxide

Tasks in different Partitions talk through Endpoints. An Endpoint can carry a **Protocol**: one description of the whole conversation, written once, from which each participant's allowed moves are derived. This page shows what a Protocol is, what is checked before the system runs, how the compiler holds each Task to its part, and what the Kernel does with the messages.

The examples are the demo Application's own (`apps/p1-demo/decl/src/lib.rs`).

Terms: a **role** is one participant of a Protocol; a **global type** is the Protocol seen from above, every message with its sender and receiver; a **local type** is one role's view of it; **projection** turns the first into the second.

## 1. A Protocol is a global type

```text
protocol PingPong {
    rec Loop {
        choice at ctl {
            Req(u32): ctl -> sink;
            Rep(u32): sink -> ctl;
            continue Loop;
        } or {
            Stop(): ctl -> sink;
            end;
        }
    }
}
```

Read as a conversation:

```mermaid
sequenceDiagram
    participant ctl
    participant sink
    loop Loop
        alt ctl chooses to ask
            ctl->>sink: Req(u32)
            sink->>ctl: Rep(u32)
        else ctl chooses to stop
            ctl->>sink: Stop()
            Note over ctl,sink: end
        end
    end
```

The four constructs:

| Construct | Meaning |
|---|---|
| `Label(Type): a -> b;` | Role `a` sends one message with this label and payload to role `b` |
| `choice at a { .. } or { .. }` | Role `a` decides which branch happens; each branch starts with a message from `a` |
| `rec X { .. }` and `continue X;` | Repetition |
| `end;` | The conversation is over |

An Endpoint binds the Protocol's roles to Tasks:

```text
endpoint pingpong: protocol PingPong {
    ctl => part0.control, sink => part1.log
} capacity = 1, payload = 8;
```

## 2. From one declaration to checked code

```mermaid
flowchart TB
    P["Protocol and Endpoint<br/>in the App_Declaration"]
    subgraph PC["Protocol_Checker (rsk-protocol)"]
        WF["1. Well-formedness"]
        PR["2. Projection onto each role"]
        EX["3. Exploration of every interleaving<br/>over bounded channels"]
        CERT["4. Certificate check<br/>(verified)"]
    end
    GEN["Generator: one typed API per role"]
    RUST["Compiler: a Task can only make<br/>the moves its role allows"]
    KRN["Kernel: copies messages between<br/>Partitions, one channel per ordered pair"]

    P --> WF --> PR --> EX --> CERT
    CERT -- "accepted, with the bound<br/>of every channel" --> GEN
    PR -- "local types" --> GEN
    GEN --> RUST --> KRN
    WF -. "rejected: names the interaction" .-> X["Build fails"]
    PR -. "rejected: names the role and the choice" .-> X
    EX -. "rejected: deadlock, orphan message,<br/>unexpected message, bound above capacity" .-> X
```

## 3. Well-formedness: what a Protocol may not say

Each rejection names the interaction at fault.

| Rule | Why |
|---|---|
| No message from a role to itself | A role has no channel to itself |
| Every `continue` names an enclosing `rec` and follows at least one message | Otherwise the loop does nothing forever |
| A choice has at least two branches, each opened by a message from the chooser | The chooser must tell someone what it chose |
| The branches' first messages go to the same receiver, with different labels | The receiver learns the choice from the label |
| Every role of the Endpoint appears in some message | A role with nothing to do is a declaration error |

## 4. Projection: each role's own view

Projection keeps the messages a role sends or receives and drops the rest. For `PingPong`:

```mermaid
stateDiagram-v2
    direction LR
    state "ctl" as C {
        [*] --> cS0
        cS0 --> cS1: send Req to sink
        cS1 --> cS0: receive Rep from sink
        cS0 --> cS2: send Stop to sink
        cS2 --> [*]
    }
```

```mermaid
stateDiagram-v2
    direction LR
    state "sink" as S {
        [*] --> sS0
        sS0 --> sS1: receive Req from ctl
        sS1 --> sS0: send Rep to ctl
        sS0 --> sS2: receive Stop from ctl
        sS2 --> [*]
    }
```

The two are mirror images: where `ctl` chooses by sending, `sink` waits and learns the choice from the label that arrives.

## 5. Three roles: when a role is not told the choice

```text
protocol Triangle {
    rec L {
        choice at ctl {
            Req(u32): ctl -> mid;
            Fwd(u32): mid -> sink;
            Ack(u32): sink -> ctl;
            continue L;
        } or {
            Stop(): ctl -> mid;
            Done(): mid -> sink;
            end;
        }
    }
}
```

```mermaid
sequenceDiagram
    participant ctl
    participant mid
    participant sink
    alt ctl chooses to ask
        ctl->>mid: Req
        mid->>sink: Fwd
        sink->>ctl: Ack
    else ctl chooses to stop
        ctl->>mid: Stop
        mid->>sink: Done
    end
```

`ctl` tells only `mid`. `sink` is not informed directly, so its two possible futures have to be **merged**: it waits for either `Fwd` or `Done` from `mid`, and the label tells it which branch it is in.

```mermaid
stateDiagram-v2
    direction LR
    state "sink (merged)" as K {
        [*] --> kS0
        kS0 --> kS1: receive Fwd from mid
        kS1 --> kS0: send Ack to ctl
        kS0 --> kS2: receive Done from mid
        kS2 --> [*]
    }
```

The merge fails, and the Protocol is rejected, when the branches cannot be told apart: for example if `sink` would have to send in one branch and receive in the other without first receiving a label that distinguishes them.

## 6. Exploration: is the conversation safe on real channels?

Messages are not delivered instantly. Each ordered pair of roles has its own first-in-first-out channel of the declared capacity, so the roles can interleave in many ways. The checker explores all of them.

```mermaid
flowchart LR
    L["Local types of every role"] --> E["Explore every reachable configuration:<br/>each role's state + contents of every channel"]
    E --> B["Bound k per channel:<br/>the most messages it ever holds"]
    E --> D{"Any bad configuration?"}
    D -- "deadlock: roles waiting on each other" --> R["Reject"]
    D -- "orphan: a message nobody will take" --> R
    D -- "unexpected: a label the receiver<br/>has no branch for" --> R
    B --> C{"k within the declared capacity?"}
    C -- "no" --> R
    C -- "yes" --> A["Accept"]
```

The bound is the worst case over every interleaving, not the best. A sender is never made to wait in this system, so a channel must be large enough for the most unlucky ordering. For `PingPong`, every channel holds at most one message, which the declared capacity of 1 meets.

The explorer itself is not trusted. It hands its set of visited configurations to a small verified checker, which confirms the set is closed under every possible move and contains no deadlocked configuration. For `PingPong` that certificate has 6 configurations.

## 7. The typed API: illegal moves do not compile

For each role the Generator emits one type per state. A method exists only in the state where the Protocol allows that move, and it consumes the old state and returns the new one.

```rust
// Role `ctl` of Endpoint `pingpong`: generated.
impl Ctl<S0> {
    fn send_req(self, m: u32) -> Result<Ctl<S1>, NotSent<S0>>;   // Req, then wait for Rep
    fn send_stop(self)        -> Result<Ctl<S2>, NotSent<S0>>;   // Stop, then end
}
impl Ctl<S1> {
    fn try_recv(self) -> Result<RecvS1, NotReceived<S1>>;        // Rep arrives -> back to S0
}
impl Ctl<S2> {
    fn finish(self) -> State;                                    // end
}
```

```mermaid
flowchart LR
    S0["Ctl&lt;S0&gt;"] -- "send_req" --> S1["Ctl&lt;S1&gt;"]
    S1 -- "try_recv gives Rep" --> S0
    S0 -- "send_stop" --> S2["Ctl&lt;S2&gt;"]
    S2 -- "finish" --> E(("end"))
```

What this rules out at compile time:

- Sending `Req` twice in a row: after `send_req` the value is a `Ctl<S1>`, which has no `send_req`.
- Receiving when it is this role's turn to send: `Ctl<S0>` has no `try_recv`.
- Using a state twice: each method takes `self` by value.

## 8. Across Jobs: the state is kept, and a dropped role is reported

A conversation outlives one Job, and a Job may not block. So each Task keeps its role's current state in a per-Task slot between Jobs.

```mermaid
sequenceDiagram
    participant Slot as RoleSlot (per Task)
    participant Job
    participant K as Kernel

    Job->>Slot: take (leaves it empty)
    Slot-->>Job: State::S0(Ctl<S0>)
    Job->>K: send_req(42)
    K-->>Job: Ok(Ctl<S1>)
    Job->>Slot: store State::S1(..)
    Note over Job: Job returns
    Note over Slot,K: next Job of the same Task
    Job->>Slot: take
    Slot-->>Job: State::S1(Ctl<S1>)
    Job->>K: try_recv()
    K-->>Job: Rep(7), Ctl<S0>
    Job->>Slot: store State::S0(..)
```

Rust's types cannot force a value to be used, only prevent it being used twice. Two run-time checks cover that gap:

- A Job that returns with its slot empty is a Protocol violation.
- A role value dropped in a state where the Protocol expects it to continue is a Protocol violation.

Both are reported to the Health_Monitor against the Task's Partition.

## 9. At run time: what the Kernel does with a message

```mermaid
flowchart LR
    subgraph PA["Partition A"]
        TA["Task playing ctl"]
    end
    subgraph KR["Kernel"]
        C1[("channel ctl to sink")]
        C2[("channel sink to ctl")]
    end
    subgraph PB["Partition B"]
        TB["Task playing sink"]
    end
    TA -- "send: supervisor call,<br/>payload copied in" --> C1
    C1 -- "receive: supervisor call,<br/>payload copied out" --> TB
    TB -- "send" --> C2
    C2 -- "receive" --> TA
```

- **One channel per ordered pair of roles**, in Kernel memory. Neither Partition can reach the other's memory; the Kernel copies.
- **Word 0 of every message is the label's index** in the Protocol. The payload follows.
- **The Kernel checks each call**: the caller is the Task bound to that role, the length matches, and the buffer lies in the caller's own RAM. Anything else is API misuse and the Partition's fault response applies.
- **A send never waits.** A full channel is reported as a violation of the bound; the checker has already shown it cannot happen if the roles follow the Protocol.
- **A receive never waits.** An empty channel returns "not yet"; the Task tries again in a later Job. An arriving message can also release a sporadic Task that names the Endpoint as its release source.

Three things can interrupt a conversation, and each has a defined outcome:

| Event | What happens |
|---|---|
| A message crosses from a lower Criticality_Level to a higher one | The receiving role must declare a validation function. A rejected message is consumed, and the fault is charged to the sending Partition |
| A Partition is stopped or restarted | Every role of its Endpoints is re-initialized. The peer's next operation returns "peer changed" and its role goes back to the initial state |
| A Task holds roles on several Endpoints | The declaration can order them (`after = other_endpoint`), so that the deadlock argument covers the Endpoints together and not only one at a time |

## 10. What is proved, and by what

```mermaid
flowchart TB
    subgraph BT["Before run time"]
        A1["Accepted Protocols never reach a set of roles<br/>all waiting on each other, nor exceed a channel's capacity"]
        A2["The lexer and the certificate check are correct"]
        A3["A Task body follows its role's local type"]
    end
    subgraph RT["Kernel"]
        B1["A channel is a first-in-first-out queue<br/>and send and receive preserve it"]
        B2["A refused send changes nothing;<br/>a full channel is reported"]
    end
    V["Verus"] --> A1
    V --> A2
    V --> B1
    V --> B2
    L["Lean, on the extracted checker"] --> A1
    T["Rust's type checker + the generated API"] --> A3
    C["Creusot"] --> A3
```

The progress theorem rests on assumptions that are discharged elsewhere or reported when broken: the roles follow their projections (the typed API, with violations reported at run time), sends never wait, a Job never suspends, and re-initialization returns every role of an Endpoint to its initial state.

## In one paragraph

A Protocol is written once as a global type. The Protocol_Checker rejects it unless it is well formed, projects onto every role, and is free of deadlock, orphan messages and unexpected messages over channels of the declared capacity. The Generator turns each role's local type into Rust types in which only the allowed moves exist. At run time the Kernel is a careful courier between Partitions: it copies, checks who is calling, and reports anything that departs from the bound the checker computed.
