use crate::models::{Equation, Operator, ParseError};

/// Parse a string of comma/whitespace-delimited equations into a list of results.
pub fn parse_equations(input: &str) -> Vec<Result<Equation, ParseError>> {
    // Replace commas with spaces, then split on whitespace to get all tokens.
    let normalized = input.replace(',', " ");
    let tokens: Vec<&str> = normalized.split_whitespace().collect();

    if tokens.is_empty() {
        return vec![];
    }

    // Group tokens into triples (lhs, op, rhs).
    tokens
        .chunks(3)
        .map(|chunk| {
            if chunk.len() != 3 {
                return Err(ParseError::MalformedEquation(format!(
                    "expected 3 tokens but got {}: '{}'",
                    chunk.len(),
                    chunk.join(" ")
                )));
            }
            parse_triple(chunk[0], chunk[1], chunk[2])
        })
        .collect()
}

fn parse_triple(lhs_str: &str, op_str: &str, rhs_str: &str) -> Result<Equation, ParseError> {
    let lhs = parse_operand(lhs_str)?;
    let operator = parse_operator(op_str)?;
    let rhs = parse_operand(rhs_str)?;
    Ok(Equation { lhs, operator, rhs })
}

fn parse_operand(s: &str) -> Result<f64, ParseError> {
    s.parse::<f64>().map_err(|_| {
        ParseError::InvalidOperand(format!("'{}' is not a valid number", s))
    })
}

fn parse_operator(s: &str) -> Result<Operator, ParseError> {
    match s {
        "+" => Ok(Operator::Add),
        "-" => Ok(Operator::Sub),
        "*" => Ok(Operator::Mul),
        "/" => Ok(Operator::Div),
        "%" => Ok(Operator::Mod),
        "//" => Ok(Operator::FloorDiv),
        other => Err(ParseError::InvalidOperator(format!(
            "'{}' is not a recognized operator",
            other
        ))),
    }
}

/// Serialize an `Equation` back to `"<lhs> <op> <rhs>"` string form.
pub fn format_equation(eq: &Equation) -> String {
    let op_str = match eq.operator {
        Operator::Add => "+",
        Operator::Sub => "-",
        Operator::Mul => "*",
        Operator::Div => "/",
        Operator::Mod => "%",
        Operator::FloorDiv => "//",
    };
    // Format numbers: omit the decimal point when the value is a whole number.
    format!("{} {} {}", format_number(eq.lhs), op_str, format_number(eq.rhs))
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Operator;
    use proptest::prelude::*;

    // --- Property-based tests ---

    // Feature: multithreaded-calculator, Property 2: Malformed tokens produce parse errors
    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]
        #[test]
        fn prop_malformed_tokens_produce_parse_errors(
            // Pick one of four malformed patterns:
            // 0 = single non-numeric word
            // 1 = two tokens (missing rhs)
            // 2 = three tokens with invalid operator
            // 3 = three tokens with non-numeric lhs or rhs
            pattern in 0usize..4,
            word in "[a-zA-Z][a-zA-Z0-9]{0,7}",
            bad_op in prop_oneof![
                Just("^"), Just("**"), Just("&&"), Just("||"), Just("??"), Just("!"),
            ],
            lhs_num in prop::num::f64::NORMAL,
            rhs_num in prop::num::f64::NORMAL,
            bad_word in "[a-zA-Z][a-zA-Z0-9]{0,7}",
            use_lhs_bad in prop::bool::ANY,
        ) {
            let input = match pattern {
                // Strategy 1: single random word (not a number)
                0 => word.clone(),
                // Strategy 2: two tokens — missing rhs
                1 => format!("{} +", lhs_num),
                // Strategy 3: three tokens with invalid operator
                2 => format!("{} {} {}", lhs_num, bad_op, rhs_num),
                // Strategy 4: three tokens where lhs or rhs is not a number
                _ => {
                    if use_lhs_bad {
                        format!("{} + {}", bad_word, rhs_num)
                    } else {
                        format!("{} + {}", lhs_num, bad_word)
                    }
                }
            };

            let results = parse_equations(&input);
            let has_error = results.iter().any(|r| r.is_err());
            prop_assert!(has_error, "expected at least one ParseError for input: {:?}", input);
        }
    }

    // Feature: multithreaded-calculator, Property 1: Delimiter splitting produces correct equation count
    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]
        #[test]
        fn prop_delimiter_splitting_correct_count(
            equations in prop::collection::vec(
                (
                    prop::num::f64::NORMAL,
                    prop_oneof![
                        Just(Operator::Add),
                        Just(Operator::Sub),
                        Just(Operator::Mul),
                        Just(Operator::Div),
                        Just(Operator::Mod),
                        Just(Operator::FloorDiv),
                    ],
                    // rhs != 0 to avoid any issues
                    prop::num::f64::NORMAL.prop_filter("rhs must be non-zero", |v| *v != 0.0),
                ),
                1..=10,
            ),
            // delimiter: 0 = ",", 1 = " ", 2 = ", "
            delimiters in prop::collection::vec(0usize..3, 0..=9),
        ) {
            let eq_list: Vec<Equation> = equations
                .into_iter()
                .map(|(lhs, operator, rhs)| Equation { lhs, operator, rhs })
                .collect();

            let formatted: Vec<String> = eq_list.iter().map(|e| format_equation(e)).collect();
            let n = formatted.len();

            let delimiter_strs = [",", " ", ", "];
            let joined = formatted
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    if i + 1 < n {
                        let delim = delimiter_strs[delimiters.get(i).copied().unwrap_or(0) % 3];
                        format!("{}{}", s, delim)
                    } else {
                        s.clone()
                    }
                })
                .collect::<String>();

            let results = parse_equations(&joined);
            let ok_count = results.iter().filter(|r| r.is_ok()).count();
            prop_assert_eq!(ok_count, n);
        }
    }

    // --- parse_equations unit tests ---

    #[test]
    fn test_parse_single_addition() {
        let results = parse_equations("3 + 4");
        assert_eq!(results.len(), 1);
        let eq = results[0].as_ref().unwrap();
        assert_eq!(eq.lhs, 3.0);
        assert_eq!(eq.operator, Operator::Add);
        assert_eq!(eq.rhs, 4.0);
    }

    #[test]
    fn test_parse_all_operators() {
        let cases = vec![
            ("1 + 2", Operator::Add),
            ("1 - 2", Operator::Sub),
            ("1 * 2", Operator::Mul),
            ("1 / 2", Operator::Div),
            ("1 % 2", Operator::Mod),
            ("1 // 2", Operator::FloorDiv),
        ];
        for (input, expected_op) in cases {
            let results = parse_equations(input);
            assert_eq!(results.len(), 1, "input: {}", input);
            assert_eq!(results[0].as_ref().unwrap().operator, expected_op);
        }
    }

    #[test]
    fn test_parse_decimal_operands() {
        let results = parse_equations("3.14 * 2.0");
        let eq = results[0].as_ref().unwrap();
        assert!((eq.lhs - 3.14).abs() < 1e-10);
        assert!((eq.rhs - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_negative_operands() {
        let results = parse_equations("-7.5 + 1");
        let eq = results[0].as_ref().unwrap();
        assert_eq!(eq.lhs, -7.5);
        assert_eq!(eq.rhs, 1.0);
    }

    #[test]
    fn test_parse_multiple_comma_delimited() {
        let results = parse_equations("1 + 2, 3 * 4, 10 // 3");
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn test_parse_multiple_whitespace_delimited() {
        let results = parse_equations("1 + 2   3 * 4");
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn test_parse_empty_input() {
        let results = parse_equations("");
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_invalid_operator() {
        let results = parse_equations("1 ^ 2");
        assert_eq!(results.len(), 1);
        match &results[0] {
            Err(ParseError::InvalidOperator(msg)) => {
                assert!(msg.contains("^"), "error should mention the bad operator");
            }
            other => panic!("expected InvalidOperator, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_invalid_operand() {
        let results = parse_equations("foo + 2");
        assert_eq!(results.len(), 1);
        match &results[0] {
            Err(ParseError::InvalidOperand(msg)) => {
                assert!(msg.contains("foo"), "error should mention the bad token");
            }
            other => panic!("expected InvalidOperand, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_malformed_too_few_tokens() {
        let results = parse_equations("1 +");
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], Err(ParseError::MalformedEquation(_))));
    }

    // --- format_equation unit tests ---

    #[test]
    fn test_format_integer_operands() {
        let eq = Equation { lhs: 3.0, operator: Operator::Add, rhs: 4.0 };
        assert_eq!(format_equation(&eq), "3 + 4");
    }

    #[test]
    fn test_format_floor_div() {
        let eq = Equation { lhs: 10.0, operator: Operator::FloorDiv, rhs: 3.0 };
        assert_eq!(format_equation(&eq), "10 // 3");
    }

    #[test]
    fn test_format_decimal_operands() {
        let eq = Equation { lhs: 3.14, operator: Operator::Mul, rhs: 2.5 };
        assert_eq!(format_equation(&eq), "3.14 * 2.5");
    }

    #[test]
    fn test_format_negative_operand() {
        let eq = Equation { lhs: -7.5, operator: Operator::Sub, rhs: 1.0 };
        assert_eq!(format_equation(&eq), "-7.5 - 1");
    }

    #[test]
    fn test_parse_zero_operand() {
        let results = parse_equations("0 + 5");
        assert_eq!(results.len(), 1);
        let eq = results[0].as_ref().unwrap();
        assert_eq!(eq.lhs, 0.0);
        assert_eq!(eq.operator, Operator::Add);
        assert_eq!(eq.rhs, 5.0);
    }

    #[test]
    fn test_parse_zero_both_operands() {
        let results = parse_equations("0 * 0");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn test_parse_mixed_valid_and_invalid() {
        // Two valid equations and one invalid in the middle
        let results = parse_equations("1 + 2, foo ^ bar, 3 * 4");
        assert_eq!(results.len(), 3);
        assert!(results[0].is_ok(), "first equation should be valid");
        assert!(results[1].is_err(), "second equation should be a parse error");
        assert!(results[2].is_ok(), "third equation should be valid");
    }

    #[test]
    fn test_parse_all_invalid_no_valid_equations() {
        let results = parse_equations("foo ^ bar, baz ?? qux");
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_err()), "all results should be errors");
    }

    #[test]
    fn test_error_message_invalid_operator_is_descriptive() {
        let results = parse_equations("1 ** 2");
        match &results[0] {
            Err(ParseError::InvalidOperator(msg)) => {
                assert!(!msg.is_empty(), "error message should not be empty");
                assert!(msg.contains("**"), "error should identify the bad operator '**'");
            }
            other => panic!("expected InvalidOperator, got {:?}", other),
        }
    }

    #[test]
    fn test_error_message_invalid_operand_is_descriptive() {
        let results = parse_equations("3 + abc");
        match &results[0] {
            Err(ParseError::InvalidOperand(msg)) => {
                assert!(!msg.is_empty(), "error message should not be empty");
                assert!(msg.contains("abc"), "error should identify the bad token 'abc'");
            }
            other => panic!("expected InvalidOperand, got {:?}", other),
        }
    }

    #[test]
    fn test_error_message_malformed_is_descriptive() {
        let results = parse_equations("1 +");
        match &results[0] {
            Err(ParseError::MalformedEquation(msg)) => {
                assert!(!msg.is_empty(), "error message should not be empty");
            }
            other => panic!("expected MalformedEquation, got {:?}", other),
        }
    }

    #[test]
    fn test_round_trip() {
        let original = "10 // 3";
        let parsed = parse_equations(original);
        let eq = parsed[0].as_ref().unwrap();
        let formatted = format_equation(eq);
        let re_parsed = parse_equations(&formatted);
        assert_eq!(re_parsed[0].as_ref().unwrap(), eq);
    }

    // Feature: multithreaded-calculator, Property 7: Parser round-trip consistency
    // Validates: Requirements 8.1, 8.2, 8.3
    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]
        #[test]
        fn prop_parser_round_trip_consistency(
            lhs in prop::num::f64::NORMAL,
            operator in prop_oneof![
                Just(Operator::Add),
                Just(Operator::Sub),
                Just(Operator::Mul),
                Just(Operator::Div),
                Just(Operator::Mod),
                Just(Operator::FloorDiv),
            ],
            rhs in prop::num::f64::NORMAL,
        ) {
            let eq = Equation { lhs, operator, rhs };

            // Step 1: format the generated equation to a string
            let s1 = format_equation(&eq);

            // Step 2: parse(s1) — first parse
            let first_parse = parse_equations(&s1);
            prop_assume!(first_parse.len() == 1 && first_parse[0].is_ok());
            let eq1 = first_parse[0].as_ref().unwrap();

            // Step 3: format(eq1) — format the first parse result
            let s2 = format_equation(eq1);

            // Step 4: parse(s2) — second parse
            let second_parse = parse_equations(&s2);
            prop_assume!(second_parse.len() == 1 && second_parse[0].is_ok());
            let eq2 = second_parse[0].as_ref().unwrap();

            // Assert: parse(format(parse(s))) == parse(s)
            // i.e., both parsed equations are structurally equal
            prop_assert_eq!(&eq1.operator, &eq2.operator);
            prop_assert!(
                (eq1.lhs - eq2.lhs).abs() < f64::EPSILON * eq1.lhs.abs().max(1.0),
                "lhs mismatch: {} vs {}",
                eq1.lhs,
                eq2.lhs
            );
            prop_assert!(
                (eq1.rhs - eq2.rhs).abs() < f64::EPSILON * eq1.rhs.abs().max(1.0),
                "rhs mismatch: {} vs {}",
                eq1.rhs,
                eq2.rhs
            );
        }
    }
}
