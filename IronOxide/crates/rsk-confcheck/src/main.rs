//! `rsk-confcheck`: the Config_Checker (task 13.1; R26). Usage:
//! `rsk-confcheck <task_model.json> [<flight-build.elf>]`. Exit status 0
//! when every recomputed value matches, 1 on any discrepancy (R26.3), 2 on
//! unusable inputs.
#![forbid(unsafe_code)]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (model_path, elf_path) = match args.as_slice() {
        [m] => (m.clone(), None),
        [m, e] => (m.clone(), Some(e.clone())),
        _ => {
            eprintln!("usage: rsk-confcheck <task_model.json> [<flight-build.elf>]");
            std::process::exit(2);
        }
    };
    let model_text = match std::fs::read_to_string(&model_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("rsk-confcheck: cannot read {model_path}: {e}");
            std::process::exit(2);
        }
    };
    let elf = match &elf_path {
        Some(p) => match std::fs::read(p) {
            Ok(b) => Some(b),
            Err(e) => {
                eprintln!("rsk-confcheck: cannot read {p}: {e}");
                std::process::exit(2);
            }
        },
        None => None,
    };
    match rsk_confcheck::check(&model_text, elf.as_deref()) {
        Ok(outcome) => {
            for line in &outcome.notes {
                println!("note: {line}");
            }
            for d in &outcome.discrepancies {
                println!("error[R26.3] {}: expected {}, found {}", d.item, d.expected, d.found);
            }
            if outcome.discrepancies.is_empty() {
                println!("rsk-confcheck: {} values checked, every recomputed value matches", outcome.checked);
                std::process::exit(0);
            }
            println!("rsk-confcheck: {} discrepancies", outcome.discrepancies.len());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("rsk-confcheck: {e}");
            std::process::exit(2);
        }
    }
}
