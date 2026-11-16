use crate::lexer::{lex, Spanned, Tok};
use crate::ParseError;
use aaiyyan_ast::{BinaryOp, Expr, Statement, UnaryOp};

const IMPLICIT_BP: u8 = 14;

pub struct Parser {
    toks: Vec<Spanned>,
    pos: usize,
    /// Set when a statement ends with Mathcad's evaluation operator '='.
    eval_tail: bool,
}

impl Parser {
    pub fn new(src: &str) -> Result<Parser, ParseError> {
        Ok(Parser { toks: lex(src)?, pos: 0, eval_tail: false })
    }

    fn peek(&self) -> &Tok { &self.toks[self.pos].tok }
    fn span(&self) -> (usize, usize) { let s = &self.toks[self.pos]; (s.start, s.end) }

    fn error(&self, msg: impl Into<String>) -> ParseError {
        let (s, e) = self.span();
        ParseError::new(msg, s, e)
    }

    fn expect(&mut self, tok: Tok) -> Result<(), ParseError> {
        if *self.peek() == tok {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(format!("expected {tok:?}, found {:?}", self.peek())))
        }
    }

    fn expect_ident(&mut self, name: &str) -> Result<(), ParseError> {
        match self.peek() {
            Tok::Ident(n) if n == name => { self.pos += 1; Ok(()) }
            other => Err(self.error(format!("expected '{name}', found {other:?}"))),
        }
    }

    fn skip_separators(&mut self) {
        while matches!(self.peek(), Tok::Newline | Tok::Semi) { self.pos += 1; }
    }

    pub fn program(mut self) -> Result<Vec<Statement>, ParseError> {
        let mut out = Vec::new();
        self.skip_separators();
        while *self.peek() != Tok::Eof {
            out.push(self.statement()?);
            self.skip_separators();
        }
        Ok(out)
    }

    pub fn expression(mut self) -> Result<Expr, ParseError> {
        self.skip_separators();
        let e = self.expr(0)?;
        self.skip_separators();
        if *self.peek() != Tok::Eof {
            return Err(self.error(format!("unexpected token {:?}", self.peek())));
        }
        Ok(e)
    }

    /// True when the token after the cursor could begin a right-hand side.
    fn rhs_follows(&self) -> bool {
        match self.toks.get(self.pos + 1).map(|s| &s.tok) {
            Some(t) => can_start_primary(t) || matches!(t, Tok::Minus | Tok::Plus),
            None => false,
        }
    }

    fn statement(&mut self) -> Result<Statement, ParseError> {
        self.eval_tail = false;
        let lhs = self.expr(2)?;
        // `2+3=` is a request to display the value, not a comparison.
        if self.eval_tail {
            return Ok(Statement::Expr(lhs));
        }
        if *self.peek() == Tok::Assign {
            self.pos += 1;
            let value = self.expr(2)?;
            if *self.peek() == Tok::Assign {
                return Err(self.error("chained assignment is not supported"));
            }
            return match lhs {
                Expr::Symbol(name) => Ok(Statement::Assign { name, value }),
                Expr::Call { name, args } => {
                    let mut params = Vec::new();
                    for a in args {
                        match a {
                            Expr::Symbol(p) => params.push(p),
                            _ => return Err(self.error("function parameters must be plain symbols")),
                        }
                    }
                    Ok(Statement::FuncDef { name, params, body: value })
                }
                _ => Err(self.error("left-hand side of ':=' must be a name or a function signature")),
            };
        }
        Ok(Statement::Expr(lhs))
    }

    fn expr(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.prefix()?;
        lhs = self.postfix(lhs)?;
        loop {
            let tok = self.peek().clone();
            if is_stop_keyword(&tok) { break; }
            // Mathcad evaluation operator: `2+3=` / `delta=` with no right-hand side.
            if matches!(tok, Tok::Eq) && !self.rhs_follows() {
                self.pos += 1;
                self.eval_tail = true;
                break;
            }
            if let Some((l, r, op)) = binop(&tok) {
                if l < min_bp { break; }
                self.pos += 1;
                let rhs = self.expr(r)?;
                lhs = Expr::bin(op, lhs, rhs);
                continue;
            }
            if matches!(tok, Tok::DotDot) {
                if 8 < min_bp { break; }
                self.pos += 1;
                let end = self.expr(9)?;
                lhs = Expr::Range { start: Box::new(lhs), step: None, end: Box::new(end) };
                continue;
            }
            if can_start_primary(&tok) && IMPLICIT_BP >= min_bp {
                let rhs = self.expr(IMPLICIT_BP + 1)?;
                lhs = Expr::bin(BinaryOp::Mul, lhs, rhs);
                continue;
            }
            break;
        }
        Ok(lhs)
    }

    fn prefix(&mut self) -> Result<Expr, ParseError> {
        let tok = self.peek().clone();
        match tok {
            Tok::Number(v) => { self.pos += 1; Ok(Expr::Number(v)) }
            Tok::Str(s) => { self.pos += 1; Ok(Expr::Str(s)) }
            Tok::Ident(name) => {
                self.pos += 1;
                match name.as_str() {
                    "true" => Ok(Expr::Bool(true)),
                    "false" => Ok(Expr::Bool(false)),
                    "not" => Ok(Expr::un(UnaryOp::Not, self.expr(6)?)),
                    "if" => self.conditional(),
                    _ => Ok(Expr::Symbol(name)),
                }
            }
            Tok::Minus => { self.pos += 1; Ok(Expr::un(UnaryOp::Neg, self.expr(16)?)) }
            Tok::Plus => { self.pos += 1; Ok(Expr::un(UnaryOp::Pos, self.expr(16)?)) }
            Tok::LParen => {
                self.pos += 1;
                let e = self.expr(0)?;
                self.expect(Tok::RParen)?;
                Ok(e)
            }
            Tok::LBracket => self.bracket(),
            other => Err(self.error(format!("unexpected token {other:?}"))),
        }
    }

    fn conditional(&mut self) -> Result<Expr, ParseError> {
        if *self.peek() == Tok::LParen {
            let save = self.pos;
            if let Ok(e) = self.try_call_conditional() {
                return Ok(e);
            }
            self.pos = save;
        }
        let cond = self.expr(2)?;
        self.expect_ident("then")?;
        let then = self.expr(2)?;
        self.expect_ident("else")?;
        let otherwise = self.expr(2)?;
        Ok(Expr::Conditional {
            cond: Box::new(cond), then: Box::new(then), otherwise: Box::new(otherwise),
        })
    }

    fn try_call_conditional(&mut self) -> Result<Expr, ParseError> {
        self.expect(Tok::LParen)?;
        let cond = self.expr(0)?;
        self.expect(Tok::Comma)?;
        let then = self.expr(0)?;
        self.expect(Tok::Comma)?;
        let otherwise = self.expr(0)?;
        self.expect(Tok::RParen)?;
        Ok(Expr::Conditional {
            cond: Box::new(cond), then: Box::new(then), otherwise: Box::new(otherwise),
        })
    }

    fn bracket(&mut self) -> Result<Expr, ParseError> {
        self.expect(Tok::LBracket)?;
        if *self.peek() == Tok::RBracket {
            self.pos += 1;
            return Ok(Expr::Vector(Vec::new()));
        }
        if *self.peek() == Tok::LBracket {
            let mut rows: Vec<Vec<Expr>> = Vec::new();
            loop {
                self.expect(Tok::LBracket)?;
                let mut row = Vec::new();
                loop {
                    row.push(self.expr(0)?);
                    if *self.peek() == Tok::Comma { self.pos += 1; continue; }
                    break;
                }
                self.expect(Tok::RBracket)?;
                rows.push(row);
                if *self.peek() == Tok::Comma { self.pos += 1; continue; }
                break;
            }
            self.expect(Tok::RBracket)?;
            let width = rows.first().map(|r| r.len()).unwrap_or(0);
            if rows.iter().any(|r| r.len() != width) {
                return Err(self.error("matrix rows must have equal length"));
            }
            return Ok(Expr::Matrix(rows));
        }
        let mut items = Vec::new();
        loop {
            items.push(self.expr(0)?);
            if *self.peek() == Tok::Comma { self.pos += 1; continue; }
            break;
        }
        self.expect(Tok::RBracket)?;
        Ok(Expr::Vector(items))
    }

    fn postfix(&mut self, mut expr: Expr) -> Result<Expr, ParseError> {
        loop {
            match self.peek().clone() {
                Tok::LParen if matches!(expr, Expr::Symbol(_)) => {
                    let name = match &expr { Expr::Symbol(n) => n.clone(), _ => unreachable!() };
                    self.pos += 1;
                    let mut args = Vec::new();
                    if *self.peek() != Tok::RParen {
                        loop {
                            args.push(self.expr(0)?);
                            if *self.peek() == Tok::Comma { self.pos += 1; continue; }
                            break;
                        }
                    }
                    self.expect(Tok::RParen)?;
                    expr = Expr::Call { name, args };
                }
                Tok::LBracket => {
                    self.pos += 1;
                    let mut indices = Vec::new();
                    loop {
                        indices.push(self.expr(0)?);
                        if *self.peek() == Tok::Comma { self.pos += 1; continue; }
                        break;
                    }
                    self.expect(Tok::RBracket)?;
                    expr = Expr::Index { target: Box::new(expr), indices };
                }
                Tok::Quote => { self.pos += 1; expr = Expr::un(UnaryOp::Transpose, expr); }
                Tok::Bang => { self.pos += 1; expr = Expr::un(UnaryOp::Factorial, expr); }
                _ => break,
            }
        }
        Ok(expr)
    }
}

fn is_stop_keyword(tok: &Tok) -> bool {
    matches!(tok, Tok::Ident(n) if n == "then" || n == "else")
}

fn can_start_primary(tok: &Tok) -> bool {
    matches!(tok, Tok::Number(_) | Tok::Ident(_) | Tok::Str(_) | Tok::LParen | Tok::LBracket)
}

fn binop(tok: &Tok) -> Option<(u8, u8, BinaryOp)> {
    Some(match tok {
        Tok::OrOr => (2, 3, BinaryOp::Or),
        Tok::AndAnd => (4, 5, BinaryOp::And),
        Tok::Eq => (6, 7, BinaryOp::Eq),
        Tok::Ne => (6, 7, BinaryOp::Ne),
        Tok::Lt => (6, 7, BinaryOp::Lt),
        Tok::Le => (6, 7, BinaryOp::Le),
        Tok::Gt => (6, 7, BinaryOp::Gt),
        Tok::Ge => (6, 7, BinaryOp::Ge),
        Tok::Plus => (10, 11, BinaryOp::Add),
        Tok::Minus => (10, 11, BinaryOp::Sub),
        Tok::Star => (12, 13, BinaryOp::Mul),
        Tok::Slash => (12, 13, BinaryOp::Div),
        Tok::Percent => (12, 13, BinaryOp::Mod),
        Tok::DotStar => (12, 13, BinaryOp::ElemMul),
        Tok::DotSlash => (12, 13, BinaryOp::ElemDiv),
        Tok::Caret => (18, 17, BinaryOp::Pow),
        Tok::DotCaret => (18, 17, BinaryOp::ElemPow),
        Tok::Ident(n) if n == "and" => (4, 5, BinaryOp::And),
        Tok::Ident(n) if n == "or" => (2, 3, BinaryOp::Or),
        Tok::Ident(n) if n == "mod" => (12, 13, BinaryOp::Mod),
        _ => return None,
    })
}
