//! Property 15, UPPAAL part (R31.8; task 16.1): for every analysed Task
//! set, the reader recovers from the export each Task's identifier,
//! Priority, period or MIT, deadline, execution-time bound, jitter,
//! blocking term, and release kind.
#![forbid(unsafe_code)]

use proptest::prelude::*;
use rsk_analyzer::inputs::AnalysedTask;
use rsk_analyzer::uppaal::{export, read_export};

fn task(i: usize) -> impl Strategy<Value = AnalysedTask> {
    (1u8..=7, 1u64..=10_000_000, 1u64..=10_000_000, 0u64..=1_000_000, 0u64..=100_000, 0u64..=10_000, any::<bool>(), any::<bool>()).prop_map(
        move |(priority, period_cycles, deadline_cycles, c, b, j, sporadic, stops_overruns)| AnalysedTask {
            id: format!("p{}.t{i}", i % 2),
            partition: format!("p{}", i % 2),
            level: "A".into(),
            priority,
            period_cycles,
            deadline_cycles,
            c,
            b,
            b_source: "L_kernel".into(),
            o: 300,
            j,
            e: c + 300,
            stops_overruns,
            sporadic,
            longest_cs: 0,
            wcet_provenance: vec![],
            wcet: c,
        },
    )
}

fn task_set() -> impl Strategy<Value = Vec<AnalysedTask>> {
    (1usize..=8).prop_flat_map(|n| (0..n).map(task).collect::<Vec<_>>())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, failure_persistence: None, ..ProptestConfig::default() })]

    /// R31.8, R31.3, R31.7, Property 15: the export's reader recovers every
    /// Task's parameters, the query names every Task's miss location, and
    /// every automaton's identifier is recorded.
    #[test]
    fn r31_8_exports_round_trip(tasks in task_set()) {
        let (xml, query) = export(&tasks);
        let back = read_export(&xml).unwrap_or_else(|e| panic!("{e}"));
        prop_assert_eq!(back.len(), tasks.len());
        for (t, x) in tasks.iter().zip(&back) {
            prop_assert_eq!(&x.id, &t.id);
            prop_assert_eq!((x.priority, x.period_cycles, x.deadline_cycles, x.exec_cycles, x.jitter_cycles, x.blocking_cycles, x.sporadic), (t.priority, t.period_cycles, t.deadline_cycles, t.e, t.j, t.b, t.sporadic));
            let comment = format!("// {}: {}", back.iter().position(|y| y.id == x.id).unwrap(), t.id);
            prop_assert!(xml.contains(&comment));
        }
        for i in 0..tasks.len() {
            let missed = format!("T{i}.Missed");
            prop_assert!(query.contains(&missed));
        }
    }
}
