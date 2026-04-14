use std::fmt::Write as FmtWrite;
use std::fs;

use crate::models::{CalcError, EvalResult, ParseError};
use crate::models::Equation;
use crate::parser::format_equation;

#[derive(Debug)]
pub enum OutputError {
    WriteError(String),
}

/// Format a single output line: `<equation_string> = <result_or_error>`
pub(crate) fn format_line(eq_result: &Result<Equation, ParseError>, eval_result: &EvalResult) -> String {
    let lhs = match eq_result {
        Ok(eq) => format_equation(eq),
        Err(e) => format_parse_error_label(e),
    };

    let rhs = match eval_result {
        Ok(value) => format_result_value(*value),
        Err(CalcError::Eval(crate::models::EvalError::DivisionByZero)) => {
            "error: division by zero".to_string()
        }
        Err(CalcError::Parse(e)) => format!("error: {}", parse_error_description(e)),
    };

    format!("{} = {}", lhs, rhs)
}

fn format_result_value(v: f64) -> String {
    if v.fract() == 0.0 && v.is_finite() {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}

fn parse_error_description(e: &ParseError) -> String {
    match e {
        ParseError::InvalidOperator(msg) => msg.clone(),
        ParseError::InvalidOperand(msg) => msg.clone(),
        ParseError::MalformedEquation(msg) => msg.clone(),
        ParseError::NoValidEquations => "no valid equations".to_string(),
    }
}

fn format_parse_error_label(e: &ParseError) -> String {
    format!("error: {}", parse_error_description(e))
}

/// Write results to `output.txt` in the current working directory.
///
/// Zips `equations` with `results` and writes one line per pair.
pub fn write_results(
    equations: &[Result<Equation, ParseError>],
    results: &[EvalResult],
) -> Result<(), OutputError> {
    let mut output = String::new();

    for (eq_result, eval_result) in equations.iter().zip(results.iter()) {
        let line = format_line(eq_result, eval_result);
        writeln!(output, "{}", line).expect("writing to String never fails");
    }

    fs::write("output.txt", &output).map_err(|e| OutputError::WriteError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CalcError, EvalError, Equation, EvalResult, Operator, ParseError};
    use proptest::prelude::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Serialize tests that mutate the process-wide cwd.
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    fn make_eq(lhs: f64, op: Operator, rhs: f64) -> Equation {
        Equation { lhs, operator: op, rhs }
    }

    // Helper: run write_results from a temp directory so tests don't pollute cwd.
    fn run_in_tempdir<F: FnOnce()>(f: F) -> (TempDir, String) {
        let _guard = CWD_LOCK.lock().unwrap();
        let dir = TempDir::new().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        f();
        let content = fs::read_to_string("output.txt").unwrap_or_default();
        std::env::set_current_dir(original).unwrap();
        (dir, content)
    }

    // --- Unit tests ---

    #[test]
    fn test_successful_result_line() {
        let eq = make_eq(3.5, Operator::Add, 2.0);
        let result: EvalResult = Ok(5.5);
        let line = format_line(&Ok(eq), &result);
        assert_eq!(line, "3.5 + 2 = 5.5");
    }

    #[test]
    fn test_division_by_zero_line() {
        let eq = make_eq(4.0, Operator::Div, 0.0);
        let result: EvalResult = Err(CalcError::Eval(EvalError::DivisionByZero));
        let line = format_line(&Ok(eq), &result);
        assert_eq!(line, "4 / 0 = error: division by zero");
    }

    #[test]
    fn test_parse_error_line() {
        let parse_err = ParseError::MalformedEquation("expected 3 tokens but got 2: '1 +'".to_string());
        let result: EvalResult = Err(CalcError::Parse(parse_err.clone()));
        let line = format_line(&Err(parse_err), &result);
        assert!(line.contains("error:"), "line should contain 'error:': {}", line);
    }

    #[test]
    fn test_integer_result_no_decimal() {
        let eq = make_eq(1.0, Operator::Add, 2.0);
        let result: EvalResult = Ok(3.0);
        let line = format_line(&Ok(eq), &result);
        assert_eq!(line, "1 + 2 = 3");
    }

    #[test]
    fn test_write_results_creates_output_file() {
        let equations = vec![Ok(make_eq(1.0, Operator::Add, 2.0))];
        let results: Vec<EvalResult> = vec![Ok(3.0)];

        let (_dir, content) = run_in_tempdir(|| {
            write_results(&equations, &results).unwrap();
        });

        assert_eq!(content.trim(), "1 + 2 = 3");
    }

    #[test]
    fn test_write_results_overwrites_existing_file() {
        let equations1 = vec![Ok(make_eq(1.0, Operator::Add, 2.0))];
        let results1: Vec<EvalResult> = vec![Ok(3.0)];

        let equations2 = vec![Ok(make_eq(10.0, Operator::Mul, 5.0))];
        let results2: Vec<EvalResult> = vec![Ok(50.0)];

        let (_dir, content) = run_in_tempdir(|| {
            write_results(&equations1, &results1).unwrap();
            write_results(&equations2, &results2).unwrap();
        });

        // Only the second write's content should remain
        assert_eq!(content.trim(), "10 * 5 = 50");
        assert!(!content.contains("1 + 2"), "old content should be gone");
    }

    #[test]
    fn test_write_results_multiple_lines_in_order() {
        let equations = vec![
            Ok(make_eq(1.0, Operator::Add, 2.0)),
            Ok(make_eq(4.0, Operator::Div, 0.0)),
            Ok(make_eq(10.0, Operator::FloorDiv, 3.0)),
        ];
        let results: Vec<EvalResult> = vec![
            Ok(3.0),
            Err(CalcError::Eval(EvalError::DivisionByZero)),
            Ok(3.0),
        ];

        let (_dir, content) = run_in_tempdir(|| {
            write_results(&equations, &results).unwrap();
        });

        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[0], "1 + 2 = 3");
        assert_eq!(lines[1], "4 / 0 = error: division by zero");
        assert_eq!(lines[2], "10 // 3 = 3");
    }

    // Strategies for generating arbitrary Equation and EvalResult values

    fn arb_operator() -> impl Strategy<Value = Operator> {
        prop_oneof![
            Just(Operator::Add),
            Just(Operator::Sub),
            Just(Operator::Mul),
            Just(Operator::Div),
            Just(Operator::Mod),
            Just(Operator::FloorDiv),
        ]
    }

    fn arb_finite_f64() -> impl Strategy<Value = f64> {
        // Use a bounded range to avoid NaN/Inf edge cases in formatting
        -1_000_000.0f64..=1_000_000.0f64
    }

    fn arb_equation() -> impl Strategy<Value = Equation> {
        (arb_finite_f64(), arb_operator(), arb_finite_f64()).prop_map(|(lhs, operator, rhs)| {
            Equation { lhs, operator, rhs }
        })
    }

    fn arb_eval_result() -> impl Strategy<Value = EvalResult> {
        prop_oneof![
            arb_finite_f64().prop_map(Ok),
            Just(Err(CalcError::Eval(EvalError::DivisionByZero))),
            Just(Err(CalcError::Parse(ParseError::MalformedEquation(
                "bad input".to_string()
            )))),
            Just(Err(CalcError::Parse(ParseError::InvalidOperator(
                "invalid operator '?'".to_string()
            )))),
        ]
    }

    // Feature: multithreaded-calculator, Property 5: Output lines match the required format
    // Validates: Requirements 5.2, 7.2
    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]
        #[test]
        fn prop_output_line_format(
            eq in arb_equation(),
            eval_result in arb_eval_result(),
        ) {
            let line = format_line(&Ok(eq), &eval_result);

            // The line must contain " = " as separator
            prop_assert!(
                line.contains(" = "),
                "line does not contain ' = ': {:?}",
                line
            );

            // Split on the first " = " to get lhs and rhs parts
            let sep = " = ";
            let sep_pos = line.find(sep).expect("separator must exist");
            let lhs_part = &line[..sep_pos];
            let rhs_part = &line[sep_pos + sep.len()..];

            // Both sides must be non-empty
            prop_assert!(!lhs_part.is_empty(), "lhs part is empty in line: {:?}", line);
            prop_assert!(!rhs_part.is_empty(), "rhs part is empty in line: {:?}", line);
        }
    }
}
