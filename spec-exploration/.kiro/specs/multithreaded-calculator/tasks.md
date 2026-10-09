# Implementation Plan: Multithreaded Calculator

## Overview

Implement a Rust CLI application that parses binary math equations from a CLI string or file, evaluates them concurrently via rayon, and writes ordered results to `output.txt`. The pipeline is: input resolution → parsing → concurrent evaluation → output writing.

## Tasks

- [ ] 1. Initialize Rust project and define core data models
  - Run `cargo new multithreaded-calculator` (or add to existing workspace)
  - Create `src/models.rs` with `Equation`, `Operator`, `CalcError`, `ParseError`, `EvalError`, `EvalResult`, and `InputError` types as defined in the design
  - Add `rayon` and `proptest` (dev-dependency) to `Cargo.toml`
  - _Requirements: 1.1, 3.1, 4.1–4.6, 5.1_

- [x] 2. Implement the parser
  - [x] 2.1 Implement `parse_equations` and `format_equation` in `src/parser.rs`
    - Split input on commas and whitespace runs into token triples
    - Parse each triple into `Result<Equation, ParseError>`, covering `InvalidOperator`, `InvalidOperand`, and `MalformedEquation` variants
    - Implement `format_equation` to serialize an `Equation` back to `"<lhs> <op> <rhs>"`
    - _Requirements: 1.1, 1.2, 2.3, 3.1, 3.2, 3.3, 8.1, 8.2_

  - [x] 2.2 Write property test for delimiter splitting (Property 1)
    - **Property 1: Delimiter splitting produces correct equation count**
    - **Validates: Requirements 1.1, 2.3**
    - Generate a `Vec<Equation>`, format each, join with random comma/whitespace delimiters, parse, assert count of `Ok` results equals original list length
    - Tag: `// Feature: multithreaded-calculator, Property 1: Delimiter splitting produces correct equation count`

  - [x] 2.3 Write property test for malformed input rejection (Property 2)
    - **Property 2: Malformed tokens produce parse errors**
    - **Validates: Requirements 3.1, 3.2**
    - Generate strings that are not valid equations (random tokens, bad operators, missing operands), assert each returns a `ParseError`
    - Tag: `// Feature: multithreaded-calculator, Property 2: Malformed tokens produce parse errors`

  - [x] 2.4 Write property test for parser round-trip consistency (Property 7)
    - **Property 7: Parser round-trip consistency**
    - **Validates: Requirements 8.1, 8.2, 8.3**
    - Generate valid equation strings, assert `parse(format(parse(s))) == parse(s)`
    - Tag: `// Feature: multithreaded-calculator, Property 7: Parser round-trip consistency`

  - [x] 2.5 Write unit tests for parser
    - Test specific operator parsing, negative/decimal operands, empty input, and descriptive error messages
    - _Requirements: 1.2, 1.3, 3.1, 3.2, 3.3_

- [x] 3. Checkpoint — Ensure all parser tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 4. Implement the evaluator
  - [x] 4.1 Implement `evaluate` in `src/evaluator.rs`
    - Pattern-match on `Operator` and apply the correct arithmetic for all six operators
    - Return `EvalError::DivisionByZero` when rhs is zero for `/`, `%`, or `//`
    - _Requirements: 4.1–4.6, 5.1_

  - [x] 4.2 Write property test for arithmetic correctness (Property 3)
    - **Property 3: Arithmetic correctness for all operators**
    - **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6**
    - Generate `(f64, Operator, f64)` with `rhs != 0`, evaluate, compare to reference arithmetic
    - Tag: `// Feature: multithreaded-calculator, Property 3: Arithmetic correctness for all operators`

  - [x] 4.3 Write property test for division by zero (Property 4)
    - **Property 4: Division by zero yields an error**
    - **Validates: Requirements 5.1**
    - Generate `(f64, div_op, 0.0)` for `/`, `%`, `//`, assert result is `Err(EvalError::DivisionByZero)`
    - Tag: `// Feature: multithreaded-calculator, Property 4: Division by zero yields an error`

  - [x] 4.4 Write unit tests for evaluator
    - Test specific examples (`3 + 4 = 7`, `10 // 3 = 3`), negative operands, zero lhs with non-zero rhs
    - _Requirements: 4.1–4.6, 5.1_

- [x] 5. Implement input resolution
  - [x] 5.1 Implement `resolve_input` in `src/input.rs`
    - Check if `arg` is a readable file path; if yes, read and return contents
    - Otherwise treat `arg` as a raw equation string
    - Return `InputError::FileNotFound` or `InputError::FileReadError` as appropriate
    - _Requirements: 2.1, 2.2_

  - [x] 5.2 Write unit tests for input resolution
    - Write a temp file, resolve input, verify contents returned correctly
    - Test non-existent path returns `InputError::FileNotFound`
    - _Requirements: 2.1, 2.2_

- [x] 6. Implement concurrent evaluation (thread pool)
  - [x] 6.1 Implement `evaluate_all` in `src/pool.rs`
    - Use `rayon::par_iter` to map over `Vec<Result<Equation, ParseError>>`
    - Pass `ParseError` items through unchanged; evaluate `Equation` items via `evaluator::evaluate`
    - Collect into `Vec<EvalResult>` preserving index order
    - _Requirements: 6.1, 6.2, 6.3_

  - [x] 6.2 Write property test for output order preservation (Property 6)
    - **Property 6: Output order matches input order**
    - **Validates: Requirements 6.3, 7.3**
    - Generate a `Vec<Equation>`, evaluate concurrently, assert result indices match input indices
    - Tag: `// Feature: multithreaded-calculator, Property 6: Output order matches input order`

- [x] 7. Implement output writing
  - [x] 7.1 Implement `write_results` in `src/output.rs`
    - Zip equations with results, format each line as `<equation_string> = <result_or_error>`
    - Write to `output.txt` in the current working directory, overwriting if present
    - _Requirements: 5.2, 7.1, 7.2, 7.3, 7.4_

  - [x] 7.2 Write property test for output line format (Property 5)
    - **Property 5: Output lines match the required format**
    - **Validates: Requirements 5.2, 7.2**
    - Generate equations and results, format lines, assert each matches `<lhs> <op> <rhs> = <result_or_error>`
    - Tag: `// Feature: multithreaded-calculator, Property 5: Output lines match the required format`

  - [x] 7.3 Write unit tests for output writer
    - Test successful result line format, division-by-zero error line, parse error line
    - Test that running twice overwrites `output.txt` with only new content
    - _Requirements: 5.2, 7.1, 7.2, 7.3, 7.4_

- [x] 8. Wire everything together in `main.rs`
  - [x] 8.1 Implement `main` in `src/main.rs`
    - Parse CLI argument via `std::env::args` or `clap`
    - Call `resolve_input`, then `parse_equations`, then `evaluate_all`, then `write_results`
    - Handle top-level errors: missing arg, file not found, no valid equations — print to stderr and exit non-zero
    - _Requirements: 1.3, 2.2, 6.1–6.3_

  - [x] 8.2 Write integration tests in `tests/integration.rs`
    - Test CLI string input end-to-end, verify `output.txt` contents
    - Test file input end-to-end with a temp file
    - Test missing file path exits non-zero with descriptive stderr message
    - Test empty/no-valid-equations input exits non-zero
    - _Requirements: 1.3, 2.2, 7.1–7.4_

- [x] 9. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- Each task references specific requirements for traceability
- Property tests use the `proptest` crate with a minimum of 100 iterations each
- Errors for individual equations (parse/eval) do not abort the run — all other equations are still processed
