# HAMR and IronOxide study (Phase P1 input)

Status: study record of 2026-11-09, written before requirements.md of spec `hamr-ironoxide-backend`. Every statement below is marked by its source: a file path in a cloned repository, a command run on this machine, or a web page (cited inline). Anything not confirmed from source or documentation is marked **unconfirmed** and carried into requirements.md as an open research question (ORQ-nn). Content from web pages was rephrased for compliance with licensing restrictions.

## 1. Versions and commit hashes

| Item | Version or commit | Source |
|---|---|---|
| IronOxide (target runtime, read-only sibling `../IronOxide`) | `423a366881d744034282bd99cb2d6d057e405f17` (2026-11-09) | `git log -1` |
| IronOxide Profile | version 0.6, Profile_Variant Ravenscar | `IronOxide/docs/profile.md` |
| IronOxide Task_Model schema | `rsk-task-model/1` | `IronOxide/crates/rsk-model/src/task_model.rs` |
| IronOxide toolchains | rustc 1.95.0 (edition 2021, `thumbv7em-none-eabihf`); Verus 0.2026.10.04.426d8b0 with rustc 1.98.1 and Z3 4.16.0; Creusot 0.15.0-dev at 18f2d371e with nightly-2026-08-03 and Why3 1.8.2; Kani 0.68.0 with CBMC 6.11.0 | `IronOxide/rust-toolchain.toml`, `IronOxide/toolchain/decision.toml`, `IronOxide/docs/verification_plan.md` section 1 |
| Sireum kekinian (HAMR host) | `e459583680c261d772f6ae87987073b9fd0c7c0b` (2026-11-04); built jar reports `Sireum v4.20261104.e4595836` | `git log -1`; `bin/sireum -v` |
| `hamr/codegen` submodule | `954bf4c0d50838badf4dda16f94bf60785aa492b` (2026-11-04) | `git submodule status` |
| `hamr/air` (AADL IR, GUMBO AST) | `d20624e4b54677702b6debda912fcd4f6a033c95` | same |
| `hamr/sysml` | `fc5768bd307d8b76985fc9c61bad901fa36c017a` | same |
| `hamr/phantom` (OSATE/FMIDE launcher) | `a77fbbbcf390d0bcfe594043ec2ff3be698c859b` | same |
| `runtime` | `526762338b0ad02f12383d48f890aa0b64eecd4a` | same |
| `slang` | `52f9d41540ff1e2e8daca176af3d7bf3351a23b7` | same |
| `logika` | `e51e4429d27b5a243dd509695cd25e38b7bedc5b` | same |
| Other kekinian submodules | alir 1f5e550, anvil 3e52b93, bin/install 4ad1c1f, hamr/vision 86ec957, infoflow 8476878, parser 2c7bd61, protocol a5fa5df, proyek d44fee0, resources 83dd8f8, server b9384a9, tools b56f05b, transpilers 6bbf9bc | same |
| JDK bootstrapped by the Sireum build | OpenJDK 25.0.4.1+1-LTS (Liberica), installed at `external/kekinian/bin/mac/java` | `java -version`; `kekinian/versions.properties` (`org.sireum.version.java=25.0.4.1+1`) |
| Scala library | 2.13.18 | `kekinian/versions.properties` |
| SysML v2 parser for HAMR | `org.sireum%hamr-sysml-parser%=4.20260911.6320233` | `kekinian/versions.properties` |
| Z3 for Sireum (not installed by this build) | 5.1.0 listed | `kekinian/versions.properties` |
| INSPECTA-models | `549d4a70c1c0368309032d84cb8ed0f2efd5af51` (2026-11-04) | `git log -1` |
| sysml-aadl-libraries (needed by the SysML path) | `6e26f80e68d16addceb21c614a1ecd6cb4b85fe1` (2026-07-29) | `git log -1` |
| Verus version the HAMR Rust output asks for | release 0.2026.10.26.d17fbd0, `vstd =0.0.0-2026-10-25-0158`, toolchain channel 1.99.1 | generated `Cargo.toml` and `rust-toolchain.toml` of each Microkit crate |

## 2. Licenses

| Repository or file | SPDX (by text) | File |
|---|---|---|
| Sireum kekinian | BSD-2-Clause (Robby, Kansas State University) | `external/kekinian/license.txt` |
| HAMR codegen | BSD-2-Clause (Kansas State University) | `external/kekinian/hamr/codegen/license.txt` (and `license.md`) |
| HAMR AIR | BSD-2-Clause | `external/kekinian/hamr/air/license.txt` |
| HAMR SysML front end | BSD-2-Clause | `external/kekinian/hamr/sysml/license.md` |
| HAMR phantom | BSD-2-Clause | `external/kekinian/hamr/phantom/license.txt` |
| Sireum runtime | BSD-2-Clause | `external/kekinian/runtime/license.txt` |
| `hamr-sysml-parser` (a Maven dependency, not in the tree) | LGPL-3.0 (derived from the SysML v2 Pilot Implementation) | license table in `external/kekinian/readme.md` |
| INSPECTA-models | BSD-3-Clause (loonwerks, 2024) | `external/INSPECTA-models/LICENSE` |
| sysml-aadl-libraries, `aadl.library` | BSD-style license of Carnegie Mellon University's SEI (2024; no SPDX identifier given) | `external/sysml-aadl-libraries/aadl.library/license.txt` |
| sysml-aadl-libraries, `hamr.aadl.library` and repository root | **no license file found** (unconfirmed; ORQ-21) | the root has only `README.md` and the two library folders |
| IronOxide | not checked for reuse: it is a dependency used unmodified | — |

Reuse rule recorded for this project: HAMR code may be adapted with the BSD-2-Clause notice retained; `hamr-sysml-parser` is used only as the library Sireum loads (no source copied); INSPECTA models may be adapted with the BSD-3-Clause notice; sysml-aadl-libraries is referenced from its clone, not copied, until the license of `hamr.aadl.library` is known.

## 3. Environment and where things were installed

- Nothing was installed system-wide. The Sireum build bootstrapped its JDK, Scala tooling and `sireum.jar` under `external/kekinian/bin/`. Downloads were redirected into the workspace with `SIREUM_CACHE=external/.cache/sireum` and `COURSIER_CACHE=external/.cache/coursier` (525 MB after the build). After the build no `~/Downloads/sireum`, `~/.m2`, `~/.ivy2`, `~/.sireum` or Coursier cache existed in the home directory.
- Cargo runs used `CARGO_HOME=external/.cache/cargo` and target directories under `external/.cache/`. They used the already-installed rustup toolchain 1.98.1 through `cargo +1.98.1` so that rustup would not install the 1.99.1 channel the generated `rust-toolchain.toml` files request.
- Present on the machine before this study and used read-only: cargo 1.98.1, rustup toolchains 1.95.0, 1.97, 1.98.1, nightly-2026-08-03/-08-21/-09-17, `qemu-system-arm` (`~/.local/bin`), `probe-rs`, `cargo-creusot`. Absent: `verus`, `cargo-verus`, `z3`, `qemu-system-aarch64`, an aarch64 or arm-none-eabi GCC, docker, the Microkit SDK, seL4, OSATE/FMIDE. `/usr/bin/java` was not used.
- `external/` is listed in `.gitignore`.

## 4. Commands run and outcomes

| # | Command (abridged; cwd under `external/`) | Outcome |
|---|---|---|
| 1 | `git clone --recursive https://github.com/sireum/kekinian.git` | done; 20 submodules checked out |
| 2 | `git clone https://github.com/loonwerks/INSPECTA-models.git` | done |
| 3 | `kekinian/bin/build.cmd` (with the cache variables above) | exit 0; `bin/sireum.jar` (136 MB) assembled; `sireum -v` prints `Sireum v4.20261104.e4595836` |
| 4 | `sireum hamr codegen --platform Microkit --package-name base --output-dir … --sel4-output-dir …/hamr/microkit --runtime-monitoring --workspace-root-dir …/aadl …/aadl/.slang/Gumbo_Structs_Arrays_Sys_i_Instance.json` on a copy of `micro-examples/microkit/gumbo-verus/structs_arrays` (`runs/structs_arrays`) | exit 0 in 39.5 s; Rust crates, C glue, `microkit.system`, `microkit.schedule.xml`, monitors and reports written (section 6). Compared with the committed output, 16 files differ: user-code regions, the runtime-monitor variant directory, and timestamps (the regenerated tree had no user code to preserve) |
| 5 | `RUSTC_BOOTSTRAP=1 cargo +1.98.1 test` in `crates/consumer_consumer` and `crates/producer_producer` of runs 4 and of the committed output | producer: 2 tests pass. consumer: 3 pass, 1 fails: `GUMBOX_tests::prop_testComputeCB_macro` panics with `attempt to multiply with overflow` at `consumer_consumer_GUMBOX.rs:168` (`square(a: i64) -> i64 { a * a }`), identically on the committed output (finding F-1) |
| 6 | `sireum hamr codegen --platform JVM …` on the same model, then `sireum proyek test` on the Slang project | codegen exit 0 (104 files); compilation fails with 5 errors: the generated component contains the placeholder text `Need to handle spec methods` and calls the missing `testSpec` (finding F-2) |
| 7 | `sireum hamr codegen --platform JVM --package-name isolette …` on a copy of `isolette` (AIR JSON `Isolette_isolette_single_sensor_Instance.json`, committed user code kept), then `sireum proyek test hamr/slang` | codegen exit 0; 4,823 tests run in 17.6 s, 4,822 pass, 1 fails: the `Example system test` halts on `None.get` in `Temperature_Sensor_i_…timeTriggered` under ART's static scheduler (`art.scheduling.static.Explorer`) (finding F-3) |
| 8 | `RUSTC_BOOTSTRAP=1 cargo +1.98.1 test` in each of the 9 committed isolette Microkit Rust crates that have tests | all pass: 83 tests (4, 4, 5, 5, 16, 4, 15, 7, 23) |
| 9 | `git clone https://github.com/santoslab/sysml-aadl-libraries.git`; `sireum hamr sysml codegen --platform Microkit --sourcepath sysml:sysml-aadl-libraries --system-name Gubmo_Structs_Arrays::Sys_i …/Gumbo_Structs_Arrays.sysml` | exit 0; the SysML v2 front end runs headless (no OSATE) and also writes the AIR instance `sysml/.slang/Sys_i_Instance.json` |

Logs: `external/build.log`, `external/runs/*.log`, `external/runs/isolette/isolette_microkit_tests.log`.

## 5. Where the end-to-end run stopped, and why

The requested end-to-end run is codegen, Verus verification of the GUMBO-derived contracts, a Microkit image build, and a boot. It stopped after codegen and host-side tests:

1. **Verus verification (`make verus`, `cargo-verus build`)**: not run. `verus` and `cargo-verus` are not installed, and the generated crates pin Verus 0.2026.10.26.d17fbd0 with rustc 1.99.1, which this machine does not have. Installing them means downloading a Verus release and a rustup toolchain; that needs the user's approval (it writes to `~/.rustup` unless a workspace-local `RUSTUP_HOME` is set up).
2. **Microkit image build (`make` in `hamr/microkit`)**: not possible. It needs the seL4 Microkit SDK (`MICROKIT_SDK`, `MICROKIT_BOARD`), an aarch64 bare-metal toolchain and `cargo -Z build-std` for `aarch64-unknown-none`; none is present and docker is not installed.
3. **QEMU boot (`make qemu`, `.github/workflows/hamr/simulate.cmd`)**: not possible without the image and `qemu-system-aarch64`.
4. **AADL text to AIR**: not run. Instantiating AADL text needs OSATE or FMIDE (`run-hamr.cmd` launches `org.sireum.aadl.osate.cli`); neither is installed. The committed AIR JSON files were used instead. The SysML v2 path does not need OSATE (row 9).

## 6. What HAMR generates for Rust (seL4 Microkit back end)

Paths are under `external/runs/structs_arrays/hamr/microkit/` (run 4) unless noted.

| Output | Path | Role |
|---|---|---|
| Component crate per thread | `crates/<proc>_<thread>/` (`Cargo.toml`, `rust-toolchain.toml`, `Makefile`) | `staticlib` linked into the Microkit protection domain |
| Entry points | `crates/consumer_consumer/src/lib.rs` | `#[no_mangle] extern "C"` functions `<thread>_initialize`, `<thread>_timeTriggered`, `<thread>_notify`; the application object lives in `static mut app: Option<…>`; a `#[panic_handler]` logs and faults |
| Application skeleton (developer code) | `src/component/<thread>_app.rs` | inside `verus! { }`: `initialize` and `timeTriggered` (periodic) or per-port handlers (sporadic) with `requires`/`ensures` between `BEGIN MARKER … END MARKER` comments; GUMBO functions as `open spec fn`; user code outside markers is preserved on regeneration |
| Platform API | `src/bridge/<thread>_api.rs` | traits `…_Put_Api`, `…_Get_Api`, `…_Full_Api`; struct `…_Application_Api<API>` with one `ghost` field per port; `get_<port>` ensures the result equals the ghost port value; `unverified_get_…` are `#[verifier::external_body]`; `…_num_invalid` counts dropped messages with invalid bit patterns (bad enum, bool not 0 or 1, string without NUL) |
| C bridge | `src/bridge/extern_c_api.rs`, `components/<thread>/src/*.c` | `unsafe` calls into the C port queues (`types/src/sb_queue_*.c`) |
| Executable contracts (GUMBOX) | `src/bridge/<thread>_GUMBOX.rs` | the GUMBO clauses as plain Rust `fn … -> bool`, used by tests and runtime monitors |
| Property-based tests | `src/test/tests.rs`, `src/test/util/{generators,test_apis,cb_apis}.rs` | proptest; `numValidComputeTestCases = 100`, reject ratio 5; inputs filtered by the GUMBOX assumes, outputs checked against the GUMBOX guarantees |
| Runtime monitors | `components/<thread>/src/<thread>_MON.c`, `crates/domain_monitor/`, `monitor.microkit.system`, `monitor.microkit.schedule.xml` | with `--runtime-monitoring`: per-thread monitor protection domains and a domain monitor |
| Types | `crates/data/src/<pkg>/*.rs`, `types/include/sb_aadl_types.h` | AADL Data Model types as Rust and C types |
| System description and schedule | `microkit.system`, `microkit.schedule.xml`, `system.mk`, `Makefile` | protection domains, channels, memory regions; the domain schedule (section 7) |

How GUMBO maps to Verus in this back end (source: `hamr/codegen/shared/.../microkit/plugins/gumbo/GumboRustPlugin.scala`, lines 634–705, and the generated files):

- compute `assume` clauses become `requires` of `timeTriggered` over `old(api).<port>`; compute `guarantee` clauses and `compute_cases` become `ensures` over `old(api)` and `final(api)` (cases as implications);
- an integration `assume` on an in port becomes a postcondition of the port's getter, whose body is trusted (`external_body`); an integration `guarantee` on an out port becomes a precondition of the putter, which the sender's code must establish;
- initialize `guarantee` clauses become `ensures` of `initialize`;
- GUMBO functions become `open spec fn`; spec functions with developer-supplied bodies call `…__developer_verus` and `…__developer_gumbox` functions.

Profile conflicts of this output, which is why the IronOxide back end cannot reuse the Microkit crate layout as is:

| HAMR Microkit Rust construct | IronOxide rule it breaks |
|---|---|
| `#![feature(proc_macro_hygiene)]`, `#![feature(stmt_expr_attributes)]`, `RUSTC_BOOTSTRAP=1` in every cargo call | PR-36 |
| `static mut app`, `static mut init_api`, `#![allow(static_mut_refs)]` | PR-05 |
| `#[no_mangle] extern "C"`, `unsafe { … }` in `lib.rs` and `extern_c_api.rs` | PR-20 (`#![forbid(unsafe_code)]` in Application crates; DD-11) |
| crate-level `#[panic_handler]` | PR-19 (the Kernel's panic handler is the only one, IronOxide R14.1) |
| `log::info!("{0}", …)`, `{info:#?}` formatting | PR-21 (`core::fmt` is `unreachable` in the Core_Subset) |
| `vstd` and `verus_builtin` dependencies of the flight crate, toolchain 1.99.1 | IronOxide R55 (one pinned toolchain, 1.95.0 for Flight_Builds), PR-36 |
| one crate per thread | IronOxide DD-11 (one crate per Partition) |

## 7. How HAMR derives and enforces the seL4 schedule

Source: `hamr/codegen/shared/.../microkit/plugins/c/components/CComponentPlugin_DomainScheduler.scala` (lines 140–620), `microkit/util/MicrokitUtil.scala`, `microkit/plugins/linters/MicrokitLinterPlugin.scala`, and the generated `microkit.schedule.xml`.

- Each process holds exactly one thread and must carry `CASE_Scheduling::Domain`; domains 0 and 1 are reserved (0 for padding, 1 for the pacer); two processes cannot share a domain.
- The bound processor must have `Timing_Properties::Clock_Period` and `Timing_Properties::Frame_Period`.
- Each thread's slot length is the upper bound of its `Compute_Execution_Time`, or 50 ms when the property is absent (`defaultComputeExecutionTimePs = 50 ms`).
- The schedule is a static cyclic sequence: for every thread, in domain-id order, a pacer slot of 30 ms (`pacerComputeExecutionTimePs`) followed by the thread's slot; a padding slot fills the frame up to `Frame_Period`. Codegen reports an error when the slots exceed the frame (unless runtime monitoring is on). The structs_arrays run produced: pacer 30 ms, producer 50 ms, pacer 30 ms, consumer 50 ms, padding 840 ms, frame 1,000 ms.
- Enforcement is seL4's domain scheduler (time partitions); the pacer notifies each thread's protection domain at its slot. An MCS variant (`--scheduling UserLand`) emits a schedule table with per-domain budgets in ns (`SystemDescriptionProvider_MCS.scala`).
- Consequence for this project: HAMR's seL4 timing is a static time-partition schedule with no response-time analysis; IronOxide uses fixed-priority preemptive SRP dispatch with run-time Budgets and an RTA. HAMR's `Frame_Period` and `CASE_Scheduling::Domain` have no IronOxide meaning.

## 8. How HAMR factors code generation, and what a new back end must implement

Source: `hamr/codegen/jvm/src/main/scala/org/sireum/hamr/codegen/CodeGen.scala` (lines 99–300), `common/plugin/Plugin.scala`, `microkit/plugins/MicrokitPlugin.scala`, and the HAMR semantics chapter [HAMR modeling elements and semantics](https://hamr.sireum.org/hamr-doc/hamr-semantics-concepts/).

1. **Front ends.** AADL text is instantiated by OSATE or FMIDE into AIR (the HAMR IR, `hamr/air`), serialized as JSON; SysML v2 is parsed by HAMR's own front end into the same AIR. GUMBO annexes arrive in AIR as `GclSubclause` (state, functions, invariants, integration, initializes, compute with assumes, guarantees, cases, handlers and flows, monitor, compositions; `hamr/air/shared/.../ir/GumboAST.scala`), with Slang expressions.
2. **Common resolution.** `common/symbols` builds the symbol table (`AadlThread`, `AadlProcess`, `AadlPort` kinds, `AadlDataAccess`, processors and bindings), `common/types` the AADL types; `common/symbols/Linter.scala` applies cross-platform checks.
3. **Platform dispatch.** `CodeGen.codeGen` selects back ends by `--platform`: JVM, Linux, Cygwin, MacOS → Arsit (Slang, optionally transpiled to C); seL4 → Arsit plus ACT (CAmkES); seL4_Only and seL4_TB → ACT; Microkit → `MicrokitCodegen`; ros2 → `Ros2Codegen`. The set is a closed enumeration (`CodegenHamrPlatform`) in the generated CLI. A new platform therefore needs a change to HAMR's CLI and to `CodeGen.scala`, or a separate tool that calls HAMR's front end and symbol table as libraries (ORQ-01).
4. **Plugins.** `Plugin` (name, disable, finalize), `ModelTransformerPlugin` (rewrites the model before codegen), and for Microkit: `MicrokitLintPlugin`, `MicrokitInitPlugin`, `MicrokitPlugin` (`canHandle`/`handle` returning resources), `MicrokitFinalizePlugin`. Plugins are passed into `codeGen` as a list; whether an external jar can add plugins to the stock CLI is **unconfirmed** (ORQ-01).
5. **AADL Runtime Services (RTS).** HAMR's semantics splits each port's state into the Infrastructure Port State (the platform's queue or cell, hidden from the application) and the Application Port State (the application's view). At dispatch the infrastructure moves input values into the application state (Receive Input, which freezes them for the whole dispatch); during the dispatch the application reads with Get Value and writes with Put Value; at completion the infrastructure releases all outputs at once (Send Output). Event data input ports are queues with an overflow policy, data input ports a one-value cell. Initialize entry points may not read inputs, must set local state and output data ports, and may put event outputs. Only the Initialize and Compute entry points exist; dispatch protocols are Periodic and Sporadic (the semantics page above, rephrased).
6. **What a new back end must implement**, read off the Microkit and Arsit back ends: (a) a model linter for the platform's restrictions; (b) the IPS for each port kind and connection, and the RTS operations above; (c) dispatch: periodic timing and sporadic triggering from the dispatch-trigger port; (d) the entry-point wrappers and application skeletons with the platform API; (e) type generation for port payloads; (f) the GUMBO translation into the target's contract language, executable GUMBOX checks, and tests; (g) the system configuration (here the `rsk::app!` declaration) and the build files.

## 9. Supported and refused AADL and SysML v2 features (HAMR)

From the semantics page (rephrased) and the linters (`common/symbols/Linter.scala`, `microkit/plugins/linters/MicrokitLinterPlugin.scala`):

| Feature | HAMR status |
|---|---|
| System, process, thread group, thread, data (as types, following the Data Model annex) | supported |
| Subprograms, subprogram groups, subprogram calls and access | not supported |
| Data components as shared data between threads | not supported (data components are types only) |
| Device components | not supported as such; `--devices-as-thread` treats them as threads (CLI help) |
| Ports: data, event, event data; `in` or `out` only | supported; `in out` refused |
| Fan-out | supported; fan-in refused for data ports, and for every port kind on Microkit |
| Connections | port to port, same category and payload type, thread to thread (Microkit) |
| Dispatch protocols | Periodic and Sporadic; others refused on Microkit ("Dispatch protocol … is not currently supported") |
| Entry points | Initialize and Compute only (no finalize, recover, mode-change) |
| Modes | not listed as supported; no mode handling found in the back ends (unconfirmed in source) |
| Processor bindings | Microkit: exactly one bound processor, every process bound, virtual processors only for Linux VM guests; Arsit and ros2 do not read processor bindings (no reference found), so a multi-processor model is generated as one system |
| seL4 restrictions | one thread per process; the semantics page says sporadic threads are not yet supported on seL4, while the Microkit linter accepts Sporadic (docs and source disagree; unconfirmed which holds at runtime) |
| SysML v2 | supported through `sireum hamr sysml codegen` with the HAMR AADL libraries (run 9) |

## 10. Findings

- **F-1 (GUMBO arithmetic semantics differ between HAMR outputs).** In structs_arrays, GUMBO `square(a: S64)` becomes the Verus spec `(a * a) as i64` (mathematical product, then a truncating cast) and the GUMBOX executable `a * a` on `i64` (panics on overflow in a test build). The property test fails on the overflow. One GUMBO expression thus has three meanings: the Slang one, the Verus one, and the executable one. This project must fix one meaning (requirements.md R22.3).
- **F-2 (JVM back end and GUMBO spec methods).** The JVM codegen of structs_arrays emits the text `Need to handle spec methods`, so the Slang project does not compile at e459583. Differential testing against the JVM simulation must use models the JVM back end accepts.
- **F-3 (isolette system test).** The isolette JVM project passes 4,822 of 4,823 tests; the example system test halts on `None.get` in the temperature sensor's compute entry point, an unset input or device value under the static scheduler.
- **F-4 (multi-processor models).** Microkit requires exactly one bound processor, and Arsit ignores bindings, so no existing back end generates one side of a two-processor model. The case study needs a model splitter (requirements.md Requirement 34).

## 11. IronOxide: confirmation of the claims in the request

| Claim | Verdict | Evidence in the IronOxide repository |
|---|---|---|
| Ravenscar-style profile enforced by types, generator checks, a source lint, a binary link checker, proof obligations and the kernel | confirmed | `docs/profile.md` v0.6: PR-01 to PR-36, enforcement categories TS, MC, LN, LK, PO, RT |
| Verus-verified SRP kernel on the nRF52840 | confirmed, with its stated trust base | `verification_plan.md` section 1 (`proof-kernel-verus`: rsk-logic 325, rsk-kernel 148 and 180 items); `docs/proof_assumptions.toml`: five trusted `Cm4Hw` functions; 60 `unsafe` constructs in the Kernel_Unsafe_Module, 5 linked to Kani harnesses and 55 to recorded reviews |
| Cortex-M4F, 64 MHz, MPU only | confirmed | `crates/rsk-gen/src/target.rs` (`NRF52840`: 64 MHz CPU, 16 MHz tick, 8 MPU regions, 3 priority bits); NG-03 (no caches) |
| 7 task priority levels plus 1 kernel level | confirmed | `crates/rsk-gen/src/validate.rs` (`MAX_PRIORITY = 7`, `KERNEL_LEVELS = 1`); design.md DD-02 |
| (not in the request) at most 7 Tasks per Application | additional limit | `target.rs`: `dispatch_irqs: &[20, 21, 22, 23, 24, 25, 40]` (7 lines on nrf52840, 7 on qemu_mps2_an386); `validate.rs` rejects more Tasks than dispatch vectors (DD-01) |
| Run-time Budget and per-interrupt-source MIT enforcement | confirmed | design.md DD-06; IronOxide R18, R19; Property 9 proven (verification_plan 9.5) |
| MPU partitions with criticality levels A, B, C | partly: the Generator accepts A to E; the conformance Application uses A, B, C | `crates/rsk-gen/src/syntax.rs` ("one of A, B, C, D, E (R15.1)"); `apps/p1-demo/decl/src/lib.rs` |
| Fault responses: record, end job, restart, stop | partly: the closed set is END_JOB, RESTART_PARTITION, STOP_PARTITION, SAFE_STATE, RECORD_ONLY (deadline misses only) and RECORD_AND_CONTINUE (Overruns, highest-criticality Partitions only) | design.md DD-10; `validate.rs` (R15.7, R18.6) |
| RTA with verified arithmetic using measured kernel parameters | confirmed: the fixed point and the term derivation are proven; the per-Task composition and the verdict are plain Rust checked against a reference implementation | `crates/rsk-analyzer/src/rta.rs`, `terms.rs`; verification_plan 9.5; `docs/hil/kernel_timing_*.json` |
| End-to-end chain deadlines | confirmed, with constraints: every hop is an Endpoint; a Sporadic_Task after the first is released by that hop's Endpoint under `defer`; bound = Δ_stamp (interrupt start) + Σ (T of a periodic receiver or MIT of a deferred sporadic receiver) + Σ WCRT | `validate.rs` (chains, R41.4); `crates/rsk-analyzer/src/lib.rs` lines 193–265 |
| Endpoints: kernel-owned bounded FIFO, 2 to 4 roles, MAX_ROLES = 4, MAX_ENDPOINTS = 8, plain-data payloads, mandatory proven validation upward, peer-reset errors | confirmed, with further limits: the plain form binds exactly 2 roles; capacity 1 to 255 per directed channel; payload 4 to 65,532 bytes, a multiple of 4; all channels together at most 8 KiB (roles² × capacity × payload); `ops` 1 to 255 per Job; a send on a full channel returns `SendError::Full` and records an event against the sender with no response | `validate.rs` (`MAX_ENDPOINTS`, `MAX_ROLES`, `MAX_ENDPOINT_STORAGE`, R42.4 rules); `crates/rsk/src/lib.rs` (`SendError`, `RecvError::{PeerChanged, Rejected}`); `crates/rsk-logic/src/logic/kernel.rs` (`record_event`, R40.5) |
| Protocols: multiparty session types with Verus-proven progress and reception theorems and a verified checker | confirmed | `crates/rsk-protocol/src/multiparty.rs` (`theorem_multiparty_progress`, `theorem_multiparty_reception`), `progress.rs` (`theorem_r41_3`), `compose.rs` (`theorem_system_r41_3` under acyclic `after` gating); the bound k is the explorer's maximum, tested for tightness rather than proven (verification_plan 9.5) |
| Application bodies proven with Creusot | confirmed for level A and B Partitions | design.md DD-13; `proof-app-creusot` (verification_plan section 1); IronOxide R46.3 |
| Hardware layer checked with Kani | confirmed | `proof-kernel-kani`: 19 harnesses; `docs/kani_stubs.toml` |
| HIL rig: three nRF52840-DK boards and an 8-channel logic analyzer, `cargo xtask hil` | confirmed | `hil-rig.toml` (target, stimulus, spare boards by J-Link serial; fx2lafw analyzer, D0 to D7 at 2 MHz; spare board not wired); `xtask/src/main.rs` (`"hil"`) |
| WCETs and kernel parameters measured, not statically bounded | confirmed | verification_plan 1.3 (static WCET is IronOxide task 24, open) |
| Kernel masked span about 400 µs under fault handling | confirmed in magnitude: 18,748 cycles (293 µs) maximum in the timing-short run, up to 26,123 cycles (408 µs) under the interrupt storm | verification_plan 11.13, 11.14 and the 2026-11-07 entries |
| Analysis not proven sound against the kernel's schedules | confirmed | verification_plan 9.5 (the composition is plain Rust; R32.5's simulator and the HIL comparison of 11.14 are testing evidence) |

## 12. IronOxide facts the back end must respect

- **App_Declaration grammar** (`crates/rsk-gen/src/syntax.rs`): header `target = …, profile = Ravenscar, operating_duration = T, safe_state = PATH[, log_capacity = N][, fp_flush_to_zero = B];` then items: `partition NAME (level, crate, code, ram, stack, fault, overrun, deadline_miss, mit_threshold[, init_arena]) { peripherals = [..]; resource N: T = INIT [invariant |v: T| PRED]; state TASK.N: T = INIT; signal N; task N (periodic, period, [offset], deadline, budget, priority[, core][, fpu][, resources = [..]][, raises = [..]]); task N (sporadic, binds = PERIPH | signal = N | endpoint = N, mit, [mit_policy = defer|discard], deadline, budget, priority, …); }`; `endpoint N: TYPE { ROLE => PART.TASK [validate = PATH] [ops = N] [after = EP], … } capacity = N, payload = BYTES;`; `endpoint N: protocol P { … }`; `protocol P { <global type> }`; `chain N { PART.TASK -> PART.TASK … } deadline = T;`. Times need explicit units (cycles, ticks, ns, us, ms, s, min, h); sizes B, KiB, MiB.
- **Conversions** (IronOxide R21.2): Budgets round up to cycles; periods, MITs and deadlines round down to ticks; offsets to the nearest tick; every changed value is listed in the Task_Model's `conversions`.
- **Task_Model** (`rsk-task-model/1`, `deny_unknown_fields`): partitions, tasks (kind, period/MIT/offset/deadline ticks, budget cycles, priority, vector, source, endpoint roles, resources, raises, nesting depth, slots, Critical_Section sites), resources (ceiling, accessors), signals, endpoints (capacity, k, roles with `release`, `ops`, `after`, `validate`), chains (hops, deadline), conversions, layout and checksums.
- **Limits**: 7 Priorities, 7 Tasks (dispatch vectors), 8 Endpoints, 2 to 4 roles (plain 2), capacity ≤ 255, payload ≤ 65,532 B, Endpoint storage ≤ 8 KiB, 5 peripheral windows per Partition (`layout.rs`, 8 MPU regions), `ops` ≤ 255, 255 Tasks in the schema, power-of-two MPU regions placed largest first (IronOxide R20.4).
- **Validation functions** (IronOxide R42.4): required for every role that receives a payload-carrying message from a Partition of lower Criticality_Level (the plain form counts both directions); `validate = PATH` for plain Endpoints, `validate = { LABEL = PATH }` for Protocol Endpoints; implemented through `rsk::Validate<T>`; proven total and panic-free by the App_Prover for levels A and B.
- **Signals** are intra-Partition: raising a Release_Signal of another Partition is API misuse, a fault of the raiser (design.md, task 5.6 entry).
- **Application code** obeys `#![forbid(unsafe_code)]`, no `static mut`, no `#![feature]`, the Core_Subset (no `core::fmt` reachable, no `derive(Debug)` in Flight_Builds), bounded loops (`#[rsk::bounds]` or constants), no recursion, no `dyn` or function pointers, one crate per Partition, Task bodies `fn task(cx: rsk_ctx::Task)`; there is no per-Task initialization hook: Resources and per-Task state take constant initial values (PR-25, PR-34).
- **Diagnostics** (`crates/rsk-gen/src/decl.rs`, `Diagnostic::new(rule, item, message)`): every Generator diagnostic names a rule (PR-nn, Rn.m, DD-nn, NG-nn) and the item.

## 13. Unconfirmed items (carried to requirements.md as ORQs)

- How an external back end plugs into HAMR's CLI (ORQ-01).
- Whether the AIR produced by OSATE carries data-access connections and `Concurrency_Control_Protocol` for shared data, which HAMR does not support (ORQ-03).
- The direction convention of AADL `Priority` values used by HAMR models (ORQ-04).
- The Slang semantics of overflow on bounded GUMBO integer types (ORQ-05, finding F-1).
- How to capture port-level traces from HAMR's JVM simulation (the generated `src/main/inspector` suggests the HAMR Inspector; ORQ-13).
- Whether HAMR's seL4 back ends support sporadic threads (docs and linter disagree; not needed by this project).
- The license of `hamr.aadl.library` in sysml-aadl-libraries (ORQ-21).
- The SysML v2 AADL library's `Communication_Properties` has no `Overflow_Handling_Protocol` (`aadl.library/Communication_Properties.sysml`), so a SysML model cannot set it and the AADL default applies; the default (DropOldest) and the other AADL defaults used in requirements.md (Deadline inherits Period, Queue_Size 1, Dequeue_Protocol OneItem, connection Timing Sampled) are taken from the library comments where present (`Timing_Properties.sysml` line 34, `Thread_Properties.sysml` line 44, `AADL.sysml` line 238) and otherwise from the AADL standard, which this project does not hold (ASM-03).
