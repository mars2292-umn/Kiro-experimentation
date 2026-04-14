use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::evaluator;
use crate::models::{CalcError, Equation, EvalResult, ParseError};

pub fn evaluate_all(equations: Vec<Result<Equation, ParseError>>) -> Vec<EvalResult> {
    equations
        .into_par_iter()
        .map(|item| match item {
            Err(parse_error) => Err(CalcError::Parse(parse_error)),
            Ok(equation) => evaluator::evaluate(&equation).map_err(CalcError::Eval),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::evaluate;
    use crate::models::Operator;
    use proptest::prelude::*;

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

    fn arb_equation() -> impl Strategy<Value = Equation> {
        (
            proptest::num::f64::NORMAL,
            arb_operator(),
            // non-zero rhs to avoid division-by-zero complications
            proptest::num::f64::NORMAL.prop_filter("rhs must be non-zero", |v| *v != 0.0),
        )
            .prop_map(|(lhs, operator, rhs)| Equation { lhs, operator, rhs })
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]
        // Feature: multithreaded-calculator, Property 6: Output order matches input order
        #[test]
        fn prop_output_order_matches_input_order(
            equations in proptest::collection::vec(arb_equation(), 1..=20),
        ) {
            // Validates: Requirements 6.3, 7.3

            // Sequential reference results
            let sequential: Vec<EvalResult> = equations
                .iter()
                .map(|eq| evaluate(eq).map_err(CalcError::Eval))
                .collect();

            // Concurrent results via evaluate_all
            let input: Vec<Result<Equation, ParseError>> =
                equations.iter().cloned().map(Ok).collect();
            let concurrent = evaluate_all(input);

            prop_assert_eq!(concurrent.len(), sequential.len());
            for (i, (seq, con)) in sequential.iter().zip(concurrent.iter()).enumerate() {
                prop_assert_eq!(
                    con, seq,
                    "result at index {} does not match: concurrent={:?}, sequential={:?}",
                    i, con, seq
                );
            }
        }
    }
}
