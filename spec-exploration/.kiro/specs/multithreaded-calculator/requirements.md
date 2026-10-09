# Requirements Document

## Introduction

A command-line calculator application written in Rust that processes multiple binary math equations concurrently using multithreading. Equations can be provided directly via CLI arguments or read from an input file. Results are written to `output.txt` in the same order as the input, with each equation and its result on a separate line.

## Glossary

- **Calculator**: The CLI application that parses, evaluates, and outputs math equation results.
- **Equation**: A single binary math expression consisting of two real numbers and one operator (e.g. `3.5 + 2`).
- **Operator**: One of the six supported binary operations: `+`, `-`, `*`, `/`, `%`, `//`.
- **Input_Source**: Either a CLI argument string or a file path provided by the user.
- **Parser**: The component responsible for splitting and parsing raw input into a list of Equations.
- **Evaluator**: The component responsible for computing the result of a single Equation.
- **Thread_Pool**: The concurrency mechanism that distributes Equations across worker threads.
- **Output_Writer**: The component responsible for writing results to `output.txt` in input order.
- **Real_Number**: A floating-point number (e.g. `1`, `3.14`, `-7.5`).

---

## Requirements

### Requirement 1: Input Parsing from CLI

**User Story:** As a user, I want to provide equations directly on the command line, so that I can quickly calculate results without creating a file.

#### Acceptance Criteria

1. WHEN the user provides a comma-and-whitespace-delimited string of equations as a CLI argument, THE Parser SHALL split the input into individual Equations.
2. WHEN an Equation string is parsed, THE Parser SHALL extract the left operand, operator, and right operand.
3. IF the CLI argument contains no valid Equations, THEN THE Calculator SHALL print an error message to stderr and exit with a non-zero status code.

---

### Requirement 2: Input Parsing from File

**User Story:** As a user, I want to provide a file containing a list of equations, so that I can process large batches of calculations.

#### Acceptance Criteria

1. WHEN the user provides a file path as a CLI argument, THE Calculator SHALL read the file contents and pass them to THE Parser.
2. IF the specified file does not exist or cannot be read, THEN THE Calculator SHALL print a descriptive error message to stderr and exit with a non-zero status code.
3. WHEN a file is read, THE Parser SHALL treat commas and whitespace as delimiters between Equations, consistent with CLI input parsing.

---

### Requirement 3: Equation Syntax Validation

**User Story:** As a user, I want clear errors for malformed equations, so that I can correct my input.

#### Acceptance Criteria

1. WHEN THE Parser encounters a token that does not match the pattern `<Real_Number> <Operator> <Real_Number>`, THEN THE Parser SHALL return a descriptive parse error for that Equation.
2. IF an Equation contains an unrecognized operator, THEN THE Parser SHALL return a descriptive parse error identifying the invalid operator.
3. THE Parser SHALL accept Real_Numbers in integer, decimal, and negative forms (e.g. `1`, `3.14`, `-7.5`).

---

### Requirement 4: Supported Operations

**User Story:** As a user, I want to perform addition, subtraction, multiplication, division, modulo, and floor division, so that I can cover common arithmetic needs.

#### Acceptance Criteria

1. WHEN an Equation uses the `+` operator, THE Evaluator SHALL return the sum of the two operands.
2. WHEN an Equation uses the `-` operator, THE Evaluator SHALL return the difference of the two operands.
3. WHEN an Equation uses the `*` operator, THE Evaluator SHALL return the product of the two operands.
4. WHEN an Equation uses the `/` operator, THE Evaluator SHALL return the quotient of the two operands as a Real_Number.
5. WHEN an Equation uses the `%` operator, THE Evaluator SHALL return the remainder of dividing the left operand by the right operand.
6. WHEN an Equation uses the `//` operator, THE Evaluator SHALL return the floor of the quotient of the two operands.

---

### Requirement 5: Division by Zero Handling

**User Story:** As a user, I want division-by-zero cases to be reported clearly, so that the application does not crash or produce silent incorrect results.

#### Acceptance Criteria

1. IF an Equation uses `/` or `%` or `//` and the right operand is zero, THEN THE Evaluator SHALL return a descriptive error for that Equation.
2. WHEN an Equation produces an evaluation error, THE Output_Writer SHALL write the error description in place of the numeric result for that Equation's output line.

---

### Requirement 6: Concurrent Evaluation

**User Story:** As a user, I want equations to be evaluated concurrently, so that large batches complete faster.

#### Acceptance Criteria

1. THE Thread_Pool SHALL evaluate each Equation on a separate worker thread.
2. WHILE Equations are being evaluated, THE Thread_Pool SHALL process multiple Equations concurrently.
3. THE Calculator SHALL collect all results before writing output, preserving the original input order.

---

### Requirement 7: Output Format

**User Story:** As a user, I want results written to `output.txt` in a readable format, so that I can review all results after the run.

#### Acceptance Criteria

1. THE Output_Writer SHALL write results to a file named `output.txt` in the current working directory.
2. THE Output_Writer SHALL write one result per line in the format `<equation> = <result>` (e.g. `1 + 2 = 3`).
3. THE Output_Writer SHALL write results in the same order as the input Equations, regardless of the order in which threads complete.
4. IF `output.txt` already exists, THE Output_Writer SHALL overwrite it.

---

### Requirement 8: Parser Round-Trip Consistency

**User Story:** As a developer, I want the parser and formatter to be consistent, so that re-parsing a formatted equation produces an equivalent result.

#### Acceptance Criteria

1. THE Parser SHALL parse each Equation into a structured representation containing the left operand, operator, and right operand.
2. THE Calculator SHALL format each Equation back into a string for output using the same operand and operator values extracted during parsing.
3. FOR ALL valid Equations, parsing then formatting then parsing SHALL produce an equivalent structured Equation (round-trip property).
