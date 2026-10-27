//! Property 15 (R25.5, R33.2; task 10.1): round trips of the published
//! formats. For every valid Task_Model the Generator prints, the reader
//! recovers an equal model that prints byte-identically; for every valid
//! WCET_Record set and Kernel_Timing_Parameters set, printing then reading
//! gives the original.
#![forbid(unsafe_code)]

use proptest::prelude::*;
use rsk_model::wcet::{KernelTiming, KernelTimingParameter, Measured, WcetRecord, WcetSet, KERNEL_TIMING_SCHEMA, WCET_SCHEMA};
use rsk_model::{print, read_kernel_timing, read_task_model, read_wcet_set};

fn config() -> ProptestConfig {
    // No regression files: a failure prints its minimal case, which is the record.
    ProptestConfig { cases: 200, failure_persistence: None, ..ProptestConfig::default() }
}

fn ident() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,11}"
}

fn hash() -> impl Strategy<Value = String> {
    "[0-9a-f]{64}".prop_map(|h| format!("sha256:{h}"))
}

fn method() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["measured".to_string(), "static".to_string(), "placeholder".to_string()])
}

fn icache() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["off".to_string(), "on".to_string()])
}

fn record() -> impl Strategy<Value = WcetRecord> {
    (
        (ident(), ident()).prop_map(|(p, t)| format!("{p}.{t}")),
        any::<u64>(),
        method(),
        ident(),
        hash(),
        ident(),
        1u64..=1_000_000_000,
        icache(),
        prop::collection::vec(ident(), 0..3),
        prop::option::of((1u64..=1_000_000, any::<u64>(), 0u64..=100, 0u64..=100, prop::collection::vec(ident(), 0..3)).prop_map(|(runs, max_observed, margin_percent, coverage_branch_percent, scenarios)| Measured { runs, max_observed, margin_percent, coverage_branch_percent, scenarios })),
    )
        .prop_map(|(item, wcet_cycles, method, tool, binary_hash, target, clock_hz, icache, conditions, measured)| WcetRecord { item, wcet_cycles, method, tool, binary_hash, target, clock_hz, icache, conditions, measured })
}

fn wcet_set() -> impl Strategy<Value = WcetSet> {
    prop::collection::vec(record(), 0..6).prop_map(|records| WcetSet { schema: WCET_SCHEMA.to_string(), records })
}

const SYMBOLS: &[&str] = &["delta_lock_in", "delta_lock_out", "delta_release", "delta_dispatch", "delta_complete", "delta_timer", "delta_irq", "delta_stamp", "l_kernel", "delta_fp_save", "delta_fp_restore", "delta_detect", "delta_enforce", "delta_hm", "delta_wake", "j_rel", "delta_svc", "delta_dma_write", "delta_view", "delta_restart", "delta_stop"];

fn kernel_timing() -> impl Strategy<Value = KernelTiming> {
    (
        hash(),
        ident(),
        1u64..=1_000_000_000,
        icache(),
        prop::collection::vec(
            (prop::sample::select(SYMBOLS.to_vec()), any::<u64>(), method(), ident(), prop::collection::vec(ident(), 0..3))
                .prop_map(|(symbol, cycles, method, tool, conditions)| KernelTimingParameter { symbol: symbol.to_string(), cycles, method, tool, conditions }),
            0..8,
        ),
    )
        .prop_map(|(binary_hash, target, clock_hz, icache, parameters)| KernelTiming { schema: KERNEL_TIMING_SCHEMA.to_string(), binary_hash, target, clock_hz, icache, parameters })
}

proptest! {
    #![proptest_config(config())]

    /// R25.1, R25.5, Property 15: the Generator's Task_Model of any valid
    /// declaration conforms to the schema, reads back, and prints
    /// byte-identically (canonical key order, R25.4).
    #[test]
    fn r25_5_task_models_round_trip(d in rsk_gen::arbitrary::declaration()) {
        let out = rsk_gen::outputs(&d, "rsk-system").unwrap_or_else(|e| panic!("rejected: {e:?}"));
        let model = read_task_model(&out.task_model_json).unwrap_or_else(|e| panic!("{e}"));
        prop_assert_eq!(print(&model), out.task_model_json);
        let again = read_task_model(&print(&model)).unwrap();
        prop_assert_eq!(again, model);
    }

    /// R33.2, Property 15: a WCET_Record set prints and reads back equal.
    #[test]
    fn r33_2_wcet_record_sets_round_trip(w in wcet_set()) {
        let text = print(&w);
        let back = read_wcet_set(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        prop_assert_eq!(back, w);
    }

    /// R33.2, Property 15: a Kernel_Timing_Parameters set prints and reads back equal.
    #[test]
    fn r33_2_kernel_timing_sets_round_trip(k in kernel_timing()) {
        let text = print(&k);
        let back = read_kernel_timing(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        prop_assert_eq!(back, k);
    }
}
