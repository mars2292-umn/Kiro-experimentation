pub mod evaluator;
pub mod input;
pub mod models;
pub mod output;
pub mod parser;
pub mod pool;

use models::InputError;
use output::OutputError;

fn main() -> std::process::ExitCode {
    // 1. Parse CLI argument
    let arg = match std::env::args().nth(1) {
        Some(a) => a,
        None => {
            eprintln!("error: no input provided\nUsage: multithreaded-calculator <equations|file>");
            return std::process::ExitCode::FAILURE;
        }
    };

    // 2. Resolve input (file or raw string)
    let raw_input = match input::resolve_input(&arg) {
        Ok(s) => s,
        Err(InputError::FileNotFound(msg)) | Err(InputError::FileReadError(msg)) => {
            eprintln!("error: {}", msg);
            return std::process::ExitCode::FAILURE;
        }
    };

    // 3. Parse equations
    let equations = parser::parse_equations(&raw_input);

    // 4. Check that at least one equation parsed successfully
    if !equations.iter().any(|r| r.is_ok()) {
        eprintln!("error: no valid equations found");
        return std::process::ExitCode::FAILURE;
    }

    // 5. Evaluate concurrently
    let results = pool::evaluate_all(equations.clone());

    // 6. Write results to output.txt
    match output::write_results(&equations, &results) {
        Ok(()) => {}
        Err(OutputError::WriteError(msg)) => {
            eprintln!("error: cannot write output.txt: {}", msg);
            return std::process::ExitCode::FAILURE;
        }
    }

    std::process::ExitCode::SUCCESS
}
