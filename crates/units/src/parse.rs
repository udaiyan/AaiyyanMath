//! Small recursive-descent parser for compound unit strings such as "kN/m^2".
use crate::registry::{lookup, Unit};
use crate::UnitError;

pub fn parse_unit_string(text: &str) -> Result<Unit, UnitError> {
    let tokens = tokenize(text)?;
    let mut p = Parser { tokens, pos: 0 };
    let unit = p.parse_expr()?;
    if p.pos != p.tokens.len() {
        return Err(UnitError::InvalidExpression(text.to_string()));
    }
    Ok(unit)
}

#[derive(Clone, Debug, PartialEq)]
enum T {
    Name(String),
    Number(f64),
    Mul,
    Div,
    Pow,
    LParen,
    RParen,
}

fn tokenize(text: &str) -> Result<Vec<T>, UnitError> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' => { i += 1; }
            '*' | '\u{00B7}' => { out.push(T::Mul); i += 1; }
            '/' => { out.push(T::Div); i += 1; }
            '^' => { out.push(T::Pow); i += 1; }
            '(' => { out.push(T::LParen); i += 1; }
            ')' => { out.push(T::RParen); i += 1; }
            _ => {
                if c.is_ascii_digit() || c == '.' {
                    let start = i;
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') { i += 1; }
                    let s: String = chars[start..i].iter().collect();
                    let v: f64 = s.parse().map_err(|_| UnitError::InvalidExpression(s.clone()))?;
                    out.push(T::Number(v));
                } else if c.is_alphabetic() || c == '\u{03A9}' || c == '\u{00B5}' || c == '\u{03BC}' {
                    let start = i;
                    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') { i += 1; }
                    out.push(T::Name(chars[start..i].iter().collect()));
                } else {
                    return Err(UnitError::InvalidExpression(format!("unexpected character '{c}'")));
                }
            }
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<T>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&T> { self.tokens.get(self.pos) }

    fn parse_expr(&mut self) -> Result<Unit, UnitError> {
        let mut lhs = self.parse_term()?;
        loop {
            match self.peek() {
                Some(T::Mul) => { self.pos += 1; let rhs = self.parse_term()?; lhs = lhs.mul_unit(&rhs); }
                Some(T::Div) => { self.pos += 1; let rhs = self.parse_term()?; lhs = lhs.div_unit(&rhs); }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_term(&mut self) -> Result<Unit, UnitError> {
        let base = match self.peek().cloned() {
            Some(T::Name(n)) => {
                self.pos += 1;
                lookup(&n).ok_or_else(|| UnitError::UnknownUnit(n))?
            }
            Some(T::Number(v)) => {
                self.pos += 1;
                if (v - 1.0).abs() > 1e-12 {
                    return Err(UnitError::InvalidExpression("only 1 is allowed as a numeric unit".into()));
                }
                Unit::dimensionless()
            }
            Some(T::LParen) => {
                self.pos += 1;
                let inner = self.parse_expr()?;
                if self.peek() != Some(&T::RParen) {
                    return Err(UnitError::InvalidExpression("expected ')'".into()));
                }
                self.pos += 1;
                inner
            }
            _ => return Err(UnitError::InvalidExpression("expected a unit".into())),
        };
        if self.peek() == Some(&T::Pow) {
            self.pos += 1;
            let exp = match self.peek().cloned() {
                Some(T::Number(v)) => { self.pos += 1; v }
                _ => return Err(UnitError::InvalidExpression("expected exponent".into())),
            };
            return Ok(base.pow_unit(exp));
        }
        Ok(base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compound_units() {
        let u = parse_unit_string("kN/m^2").unwrap();
        assert!((u.factor - 1000.0).abs() < 1e-9);
        assert!(u.dim.compatible(&crate::Dimension::pressure()));
    }
}
