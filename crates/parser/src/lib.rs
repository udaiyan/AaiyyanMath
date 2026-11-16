//! Mathematical parser: lexer + Pratt (precedence climbing) parser.
//! Grammar is documented in docs/grammar.txt. No regular expressions are used.
pub mod grammar;
pub mod lexer;

use aaiyyan_ast::{Expr, Statement};

#[derive(Clone, Debug, thiserror::Error)]
#[error("{message}")]
pub struct ParseError {
    pub message: String,
    pub start: usize,
    pub end: usize,
}

impl ParseError {
    pub fn new(message: impl Into<String>, start: usize, end: usize) -> Self {
        ParseError { message: message.into(), start, end }
    }

    pub fn line_col(&self, source: &str) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (idx, ch) in source.char_indices() {
            if idx >= self.start { break; }
            if ch == '\n' { line += 1; col = 1; } else { col += 1; }
        }
        (line, col)
    }

    pub fn describe(&self, source: &str) -> String {
        let (l, c) = self.line_col(source);
        format!("line {l}, column {c}: {}", self.message)
    }
}

pub fn parse_program(source: &str) -> Result<Vec<Statement>, ParseError> {
    grammar::Parser::new(source)?.program()
}

pub fn parse_expression(source: &str) -> Result<Expr, ParseError> {
    grammar::Parser::new(source)?.expression()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aaiyyan_ast::BinaryOp;

    #[test]
    fn assignments_and_precedence() {
        let stmts = parse_program("a := 5\nb := 10\nc := sqrt(a^2 + b^2)").unwrap();
        assert_eq!(stmts.len(), 3);
        assert_eq!(stmts[0].defined_name(), Some("a"));
        match &stmts[2] {
            Statement::Assign { value, .. } => match value {
                Expr::Call { name, args } => {
                    assert_eq!(name, "sqrt");
                    assert!(matches!(args[0], Expr::Binary { op: BinaryOp::Add, .. }));
                }
                _ => panic!("expected call"),
            },
            _ => panic!("expected assignment"),
        }
    }

    #[test]
    fn unit_juxtaposition_is_multiplication() {
        let stmts = parse_program("w := 10 kN/m").unwrap();
        let src = aaiyyan_ast::mathstr::statement_to_string(&stmts[0]);
        assert!(src.contains("kN"));
    }

    #[test]
    fn function_definition() {
        let stmts = parse_program("f(x) := x^3 + 2*x^2 + 5*x").unwrap();
        assert!(matches!(stmts[0], Statement::FuncDef { .. }));
    }

    #[test]
    fn matrix_and_vector() {
        let stmts = parse_program("A := [[1, 2], [3, 4]]\nv := [1, 2, 3]").unwrap();
        assert!(matches!(stmts[0].value_expr(), Expr::Matrix(_)));
        assert!(matches!(stmts[1].value_expr(), Expr::Vector(_)));
    }

    #[test]
    fn conditional_keyword_form() {
        let e = parse_expression("if x > 0 then 1 else -1").unwrap();
        assert!(matches!(e, Expr::Conditional { .. }));
    }

    #[test]
    fn power_is_right_associative() {
        let e = parse_expression("2^3^2").unwrap();
        match e {
            Expr::Binary { rhs, .. } => assert!(matches!(*rhs, Expr::Binary { op: BinaryOp::Pow, .. })),
            _ => panic!("expected binary"),
        }
    }

    #[test]
    fn error_reports_position() {
        let err = parse_program("a := ").unwrap_err();
        assert!(err.describe("a := ").starts_with("line 1"));
    }
}
