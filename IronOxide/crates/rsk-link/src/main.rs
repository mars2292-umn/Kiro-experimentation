//! `rsk-link`: the Link_Checker (tasks 12.1 and 12.2; R37, PR-17, PR-18).
//! Usage: `rsk-link <flight-build.elf> <task_model.json> [--out <stack_bounds.json>]`.
//! Exit status 0 when every function is bounded and every stack region
//! holds its worst case, 1 on a violation (R37.2, R37.3), 2 on unusable
//! inputs.
#![forbid(unsafe_code)]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut positional = Vec::new();
    let mut out: Option<String> = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == "--out" {
            out = it.next();
        } else {
            positional.push(a);
        }
    }
    let [elf_path, model_path] = positional.as_slice() else {
        eprintln!("usage: rsk-link <flight-build.elf> <task_model.json> [--out <stack_bounds.json>]");
        std::process::exit(2);
    };
    let elf = match std::fs::read(elf_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("rsk-link: cannot read {elf_path}: {e}");
            std::process::exit(2);
        }
    };
    let model = match std::fs::read_to_string(model_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("rsk-link: cannot read {model_path}: {e}");
            std::process::exit(2);
        }
    };
    match rsk_link::check(&elf, &model) {
        Ok(result) => {
            for n in &result.notes {
                println!("note: {n}");
            }
            for v in &result.violations {
                println!("error[{}] {}", v.rule, v.message);
            }
            if let Some(path) = out {
                if let Err(e) = std::fs::write(&path, result.export()) {
                    eprintln!("rsk-link: cannot write {path}: {e}");
                    std::process::exit(2);
                }
                println!("note: stack bounds written to {path}");
            }
            if result.violations.is_empty() {
                println!("rsk-link: {} functions bounded, {} stack regions within their worst case", result.functions.len(), result.regions.len());
                std::process::exit(0);
            }
            println!("rsk-link: {} violations", result.violations.len());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("rsk-link: {e}");
            std::process::exit(2);
        }
    }
}
