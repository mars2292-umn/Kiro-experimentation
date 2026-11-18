//! What the Generator exports in the Task_Model, read back from the
//! committed models of the demo (`apps/p1-demo/gen*/task_model.json`, which
//! `cargo xtask gen --check` keeps equal to the Generator's output) and
//! through the published reader.
#![forbid(unsafe_code)]

use rsk_model::read_task_model;
use serde_json::Value;
use std::collections::BTreeSet;

fn text(dir: &str, file: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(dir).join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn model(dir: &str) -> Value {
    let t = text(dir, "task_model.json");
    read_task_model(&t).unwrap_or_else(|e| panic!("{dir}: the reader refuses the committed model: {e:?}"));
    serde_json::from_str(&t).unwrap()
}

fn list<'a>(v: &'a Value, key: &str) -> &'a Vec<Value> {
    v.get(key).and_then(Value::as_array).unwrap_or_else(|| panic!("no list `{key}`"))
}

fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or_else(|| panic!("no string `{key}` in {v}"))
}

/// The model with `key` removed from the first element of `list`, printed.
fn without(m: &Value, list_key: &str, key: &str) -> String {
    let mut m = m.clone();
    let removed = m[list_key][0].as_object_mut().unwrap().remove(key);
    assert!(removed.is_some(), "`{list_key}[0]` has no `{key}`");
    serde_json::to_string(&m).unwrap()
}

/// R15.4: each Partition is exported with its Criticality_Level, its
/// members (Tasks, Resources, Release_Signals, peripherals), its region
/// sizes, its MIT_Violation threshold and its three responses; the members
/// named are the Tasks and Resources that name the Partition; and a model
/// that lacks one of these is refused by the reader.
#[test]
fn r15_4_each_partition_is_exported_with_level_members_regions_threshold_and_responses() {
    let m = model("apps/p1-demo/gen");
    let partitions = list(&m, "partitions");
    assert!(partitions.len() >= 2);
    for p in partitions {
        let id = s(p, "id");
        assert!(["A", "B", "C", "D", "E"].contains(&s(p, "level")), "{id}");
        for response in ["fault", "overrun", "deadline_miss"] {
            assert!(!s(p, response).is_empty(), "{id}.{response}");
        }
        assert!(p["mit_threshold"].is_u64(), "{id}.mit_threshold");
        for region in ["code", "ram", "stack"] {
            assert!(p["regions"][region].as_u64().is_some_and(|b| b > 0), "{id}.regions.{region}");
        }
        assert!(p["peripherals"].is_array() && p["signals"].is_array());
        // The members are exactly the items that name this Partition.
        let named = |key: &str| -> BTreeSet<String> {
            list(&m, key).iter().filter(|x| s(x, "partition") == id).map(|x| s(x, "id").to_string()).collect()
        };
        let listed = |key: &str| -> BTreeSet<String> { list(p, key).iter().map(|x| x.as_str().unwrap().to_string()).collect() };
        assert!(!listed("tasks").is_empty(), "{id}: no Task");
        assert_eq!(listed("tasks"), named("tasks"), "{id}: Tasks");
        assert_eq!(listed("resources"), named("resources"), "{id}: Resources");
    }
    for field in ["level", "fault", "overrun", "deadline_miss", "mit_threshold", "regions", "tasks", "resources"] {
        assert!(read_task_model(&without(&m, "partitions", field)).is_err(), "a Partition without `{field}` was accepted");
    }
}

/// R23.5: every derived value is exported marked as derived: a Resource's
/// Ceiling, a Task's nesting depth, vector, slots and Critical_Section
/// sites, the capacities, the MPU layout and each Partition's regions of
/// it, and (since 2026-11-17) the priority-to-hardware mapping and the
/// Kernel-reserved levels; a model without the last marker is refused.
#[test]
fn r23_5_every_derived_value_is_exported_with_its_marker() {
    let m = model("apps/p1-demo/gen");
    let marks = |v: &Value| -> BTreeSet<String> { list(v, "derived").iter().map(|x| x.as_str().unwrap().to_string()).collect() };
    for r in list(&m, "resources") {
        assert!(marks(r).contains("ceiling") && r["ceiling"].is_u64(), "{}", s(r, "id"));
    }
    for t in list(&m, "tasks") {
        let d = marks(t);
        for field in ["nesting_depth", "vector", "slots", "cs_sites"] {
            assert!(d.contains(field) && !t[field].is_null(), "{}.{field}", s(t, "id"));
        }
    }
    assert_eq!(m["capacities"]["derived"], Value::Bool(true));
    assert_eq!(m["layout"]["derived"], Value::Bool(true));
    for p in list(&m, "partitions") {
        assert_eq!(p["mpu"]["derived"], Value::Bool(true), "{}", s(p, "id"));
    }
    assert!(!list(&m, "priority_map").is_empty() && !list(&m, "kernel_levels").is_empty());
    assert_eq!(marks(&m), ["kernel_levels", "priority_map"].iter().map(|x| x.to_string()).collect());
    let mut unmarked = m.clone();
    unmarked.as_object_mut().unwrap().remove("derived");
    assert!(read_task_model(&serde_json::to_string(&unmarked).unwrap()).is_err(), "a model without the top-level marker was accepted");
}

/// R25.3: the Task_Model carries the identifiers that WCET_Records are
/// associated by: an entry point for every Task and one Critical_Section
/// site for each Resource the Task accesses, named from the Task and the
/// Resource; and every record of the rig's committed set for the stress
/// configuration names a Task or a site of that configuration's model.
/// (A release condition carries application code only in the Jorvik
/// variant, R5.5; in this variant it is a Kernel flag and has no record.)
#[test]
fn r25_3_wcet_records_are_associated_by_the_models_task_and_site_identifiers() {
    let m = model("apps/p1-demo/gen-nrf52840_stress");
    let mut ids = BTreeSet::new();
    for t in list(&m, "tasks") {
        let id = s(t, "id");
        assert!(s(t, "entry").contains("::"), "{id}: no entry point");
        let sites: Vec<&str> = list(t, "cs_sites").iter().map(|x| x.as_str().unwrap()).collect();
        let resources: Vec<&str> = list(t, "resources").iter().map(|x| x.as_str().unwrap()).collect();
        assert_eq!(sites.len(), resources.len(), "{id}: one site per accessed Resource");
        for r in &resources {
            let short = r.rsplit('.').next().unwrap();
            assert!(sites.contains(&format!("{id}#cs:{short}").as_str()), "{id}: no site for {r}: {sites:?}");
        }
        ids.insert(id.to_string());
        ids.extend(sites.iter().map(|x| x.to_string()));
    }
    let records: Value = serde_json::from_str(&text("docs/hil", "wcet_nrf52840_stress.json")).unwrap();
    let recorded: BTreeSet<String> = list(&records, "records").iter().map(|r| s(r, "item").to_string()).collect();
    assert!(!recorded.is_empty());
    let unknown: Vec<&String> = recorded.difference(&ids).collect();
    assert!(unknown.is_empty(), "records of items the model does not name: {unknown:?}");
}

/// R27.1: the core field of a Task, the core-or-global field of a Resource
/// and the core pair of an Endpoint are in the model, and each is required:
/// a model without one is refused, there being no default.
#[test]
fn r27_1_the_core_fields_are_present_and_required_with_no_default() {
    let m = model("apps/p1-demo/gen");
    assert!(list(&m, "tasks").iter().all(|t| t["core"] == Value::from(0)));
    assert!(list(&m, "resources").iter().all(|r| !r["core"].is_null()));
    assert!(list(&m, "endpoints").iter().all(|e| e["cores"].as_array().is_some_and(|c| c.len() == 2)));
    for (items, field) in [("tasks", "core"), ("resources", "core"), ("endpoints", "cores")] {
        assert!(read_task_model(&without(&m, items, field)).is_err(), "a model whose first of `{items}` lacks `{field}` was accepted");
    }
}
