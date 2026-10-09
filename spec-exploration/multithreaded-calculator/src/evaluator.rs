use crate::models::{Equation, EvalError, Operator};

#[cfg(test)]
mod tests {
    use super::*;
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

    fn arb_div_operator() -> impl Strategy<Value = Operator> {
        prop_oneof![
            Just(Operator::Div),
            Just(Operator::Mod),
            Just(Operator::FloorDiv),
        ]
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]
        // Feature: multithreaded-calculator, Property 4: Division by zero yields an error
        #[test]
        fn prop_division_by_zero(
            lhs in proptest::num::f64::NORMAL,
            op in arb_div_operator(),
        ) {
            // Validates: Requirements 5.1
            let eq = Equation { lhs, operator: op, rhs: 0.0 };
            let result = evaluate(&eq);
            prop_assert_eq!(result, Err(EvalError::DivisionByZero));
        }
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]
        // Feature: multithreaded-calculator, Property 3: Arithmetic correctness for all operators
        #[test]
        fn prop_arithmetic_correctness(
            lhs in proptest::num::f64::NORMAL,
            rhs in proptest::num::f64::NORMAL,
            op in arb_operator(),
        ) {
            // Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6
            prop_assume!(rhs != 0.0);

            let eq = Equation { lhs, operator: op.clone(), rhs };
            let result = evaluate(&eq).expect("expected Ok for non-zero rhs");

            let expected = match op {
                Operator::Add     => lhs + rhs,
                Operator::Sub     => lhs - rhs,
                Operator::Mul     => lhs * rhs,
                Operator::Div     => lhs / rhs,
                Operator::Mod     => lhs % rhs,
                Operator::FloorDiv => (lhs / rhs).floor(),
            };

            prop_assert_eq!(result, expected);
        }
    }

    fn eq(lhs: f64, op: Operator, rhs: f64) -> Equation {
        Equation { lhs, operator: op, rhs }
    }

    // --- Specific operator examples ---

    #[test]
    fn test_add() {
        assert_eq!(evaluate(&eq(3.0, Operator::Add, 4.0)), Ok(7.0));
    }

    #[test]
    fn test_floor_div() {
        assert_eq!(evaluate(&eq(10.0, Operator::FloorDiv, 3.0)), Ok(3.0));
    }

    #[test]
    fn test_mod() {
        assert_eq!(evaluate(&eq(10.0, Operator::Mod, 3.0)), Ok(1.0));
    }

    #[test]
    fn test_div() {
        assert_eq!(evaluate(&eq(6.0, Operator::Div, 2.0)), Ok(3.0));
    }

    #[test]
    fn test_sub() {
        assert_eq!(evaluate(&eq(5.0, Operator::Sub, 3.0)), Ok(2.0));
    }

    #[test]
    fn test_mul() {
        assert_eq!(evaluate(&eq(4.0, Operator::Mul, 3.0)), Ok(12.0));
    }

    // --- Negative operands ---

    #[test]
    fn test_neg_lhs_add() {
        assert_eq!(evaluate(&eq(-5.0, Operator::Add, 3.0)), Ok(-2.0));
    }

    #[test]
    fn test_neg_rhs_div() {
        assert_eq!(evaluate(&eq(10.0, Operator::Div, -2.0)), Ok(-5.0));
    }

    // --- Zero lhs with non-zero rhs ---

    #[test]
    fn test_zero_lhs_add() {
        assert_eq!(evaluate(&eq(0.0, Operator::Add, 5.0)), Ok(5.0));
    }

    #[test]
    fn test_zero_lhs_mul() {
        assert_eq!(evaluate(&eq(0.0, Operator::Mul, 100.0)), Ok(0.0));
    }

    #[test]
    fn test_zero_lhs_div() {
        assert_eq!(evaluate(&eq(0.0, Operator::Div, 5.0)), Ok(0.0));
    }

    // --- Division by zero ---

    #[test]
    fn test_div_by_zero() {
        assert_eq!(evaluate(&eq(5.0, Operator::Div, 0.0)), Err(EvalError::DivisionByZero));
    }

    #[test]
    fn test_mod_by_zero() {
        assert_eq!(evaluate(&eq(5.0, Operator::Mod, 0.0)), Err(EvalError::DivisionByZero));
    }

    #[test]
    fn test_floor_div_by_zero() {
        assert_eq!(evaluate(&eq(5.0, Operator::FloorDiv, 0.0)), Err(EvalError::DivisionByZero));
    }

    // --- Decimal operands ---

    #[test]
    fn test_decimal_add() {
        assert_eq!(evaluate(&eq(3.5, Operator::Add, 1.5)), Ok(5.0));
    }
}

pub fn evaluate(eq: &Equation) -> Result<f64, EvalError> {
    let lhs = eq.lhs;
    let rhs = eq.rhs;

    match eq.operator {
        Operator::Add => Ok(lhs + rhs),
        Operator::Sub => Ok(lhs - rhs),
        Operator::Mul => Ok(lhs * rhs),
        Operator::Div => {
            if rhs == 0.0 {
                Err(EvalError::DivisionByZero)
            } else {
                Ok(lhs / rhs)
            }
        }
        Operator::Mod => {
            if rhs == 0.0 {
                Err(EvalError::DivisionByZero)
            } else {
                Ok(lhs % rhs)
            }
        }
        Operator::FloorDiv => {
            if rhs == 0.0 {
                Err(EvalError::DivisionByZero)
            } else {
                Ok((lhs / rhs).floor())
            }
        }
    }
}
