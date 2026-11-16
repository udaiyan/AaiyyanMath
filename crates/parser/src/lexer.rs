use crate::ParseError;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Number(f64),
    Ident(String),
    Str(String),
    Assign,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    Bang,
    Quote,
    DotStar,
    DotSlash,
    DotCaret,
    DotDot,
    Comma,
    Semi,
    LParen,
    RParen,
    LBracket,
    RBracket,
    AndAnd,
    OrOr,
    Newline,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Spanned {
    pub tok: Tok,
    pub start: usize,
    pub end: usize,
}

pub fn lex(src: &str) -> Result<Vec<Spanned>, ParseError> {
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let mut out: Vec<Spanned> = Vec::new();
    let mut i = 0usize;
    let mut depth = 0i32;

    let at = |k: usize| -> Option<char> { chars.get(k).map(|(_, c)| *c) };
    let off = |k: usize| -> usize { chars.get(k).map(|(o, _)| *o).unwrap_or(src.len()) };

    while i < chars.len() {
        let (start, c) = chars[i];
        if c == ' ' || c == '\t' || c == '\r' { i += 1; continue; }
        if c == '#' || (c == '/' && at(i + 1) == Some('/')) {
            while i < chars.len() && chars[i].1 != '\n' { i += 1; }
            continue;
        }
        if c == '\n' {
            if depth == 0 {
                out.push(Spanned { tok: Tok::Newline, start, end: start + 1 });
            }
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && at(i + 1).map(|n| n.is_ascii_digit()).unwrap_or(false)) {
            let mut k = i;
            while at(k).map(|n| n.is_ascii_digit()).unwrap_or(false) { k += 1; }
            if at(k) == Some('.') && at(k + 1) != Some('.') {
                k += 1;
                while at(k).map(|n| n.is_ascii_digit()).unwrap_or(false) { k += 1; }
            }
            if matches!(at(k), Some('e') | Some('E')) {
                let mut m = k + 1;
                if matches!(at(m), Some('+') | Some('-')) { m += 1; }
                if at(m).map(|n| n.is_ascii_digit()).unwrap_or(false) {
                    while at(m).map(|n| n.is_ascii_digit()).unwrap_or(false) { m += 1; }
                    k = m;
                }
            }
            let text: String = chars[i..k].iter().map(|(_, ch)| *ch).collect();
            let value: f64 = text.parse()
                .map_err(|_| ParseError::new(format!("invalid number '{text}'"), start, off(k)))?;
            out.push(Spanned { tok: Tok::Number(value), start, end: off(k) });
            i = k;
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let mut k = i + 1;
            while at(k).map(|n| n.is_alphanumeric() || n == '_').unwrap_or(false) { k += 1; }
            let text: String = chars[i..k].iter().map(|(_, ch)| *ch).collect();
            out.push(Spanned { tok: Tok::Ident(text), start, end: off(k) });
            i = k;
            continue;
        }
        if c == '"' {
            let mut k = i + 1;
            let mut value = String::new();
            loop {
                match at(k) {
                    None => return Err(ParseError::new("unterminated string", start, src.len())),
                    Some('"') => { k += 1; break; }
                    Some('\\') => {
                        let esc = at(k + 1).unwrap_or('"');
                        value.push(match esc { 'n' => '\n', 't' => '\t', other => other });
                        k += 2;
                    }
                    Some(ch) => { value.push(ch); k += 1; }
                }
            }
            out.push(Spanned { tok: Tok::Str(value), start, end: off(k) });
            i = k;
            continue;
        }

        let two: Option<(Tok, usize)> = match (c, at(i + 1)) {
            (':', Some('=')) => Some((Tok::Assign, 2)),
            ('=', Some('=')) => Some((Tok::Eq, 2)),
            ('!', Some('=')) => Some((Tok::Ne, 2)),
            ('<', Some('=')) => Some((Tok::Le, 2)),
            ('>', Some('=')) => Some((Tok::Ge, 2)),
            ('&', Some('&')) => Some((Tok::AndAnd, 2)),
            ('|', Some('|')) => Some((Tok::OrOr, 2)),
            ('.', Some('.')) => Some((Tok::DotDot, 2)),
            ('.', Some('*')) => Some((Tok::DotStar, 2)),
            ('.', Some('/')) => Some((Tok::DotSlash, 2)),
            ('.', Some('^')) => Some((Tok::DotCaret, 2)),
            _ => None,
        };
        if let Some((tok, len)) = two {
            out.push(Spanned { tok, start, end: off(i + len) });
            i += len;
            continue;
        }

        let tok = match c {
            '=' => Tok::Eq,
            '<' => Tok::Lt,
            '>' => Tok::Gt,
            '+' => Tok::Plus,
            '-' => Tok::Minus,
            '*' => Tok::Star,
            '/' => Tok::Slash,
            '^' => Tok::Caret,
            '%' => Tok::Percent,
            '!' => Tok::Bang,
            '\'' => Tok::Quote,
            ',' => Tok::Comma,
            ';' => Tok::Semi,
            '(' => { depth += 1; Tok::LParen }
            ')' => { depth -= 1; Tok::RParen }
            '[' => { depth += 1; Tok::LBracket }
            ']' => { depth -= 1; Tok::RBracket }
            ':' => return Err(ParseError::new("expected ':=' for assignment", start, start + 1)),
            other => return Err(ParseError::new(
                format!("unexpected character '{other}'"), start, start + other.len_utf8()
            )),
        };
        out.push(Spanned { tok, start, end: off(i + 1) });
        i += 1;
    }
    out.push(Spanned { tok: Tok::Eof, start: src.len(), end: src.len() });
    Ok(out)
}
