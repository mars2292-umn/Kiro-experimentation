/// A parsed binary equation
#[derive(Debug, Clone, PartialEq)]
pub struct Equation {
    pub lhs: f64,
    pub operator: Operator,
    pub rhs: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add,      // +
    Sub,      // -
    Mul,      // *
    Div,      // /
    Mod,      // %
    FloorDiv, // //
}

/// Result of evaluating one equation
pub type EvalResult = Result<f64, CalcError>;

#[derive(Debug, Clone, PartialEq)]
pub enum CalcError {
    Parse(ParseError),
    Eval(EvalError),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    InvalidOperator(String),
    InvalidOperand(String),
    MalformedEquation(String),
    NoValidEquations,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    DivisionByZero,
}

#[derive(Debug)]
pub enum InputError {
    FileNotFound(String),
    FileReadError(String),
}
