//! The text parser: tokens are identifiers, punctuation, and the payload
//! type text between a label's parentheses (kept verbatim with its spaces
//! normalized, so `app :: Fix` reads as `app::Fix`).

use crate::ast::{Global, Message};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Tok {
    Ident(String),
    /// The text inside a label's parentheses.
    Paren(String),
    Colon,
    Arrow,
    Semi,
    LBrace,
    RBrace,
}

fn tokens(text: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Tok::Ident(chars[start..i].iter().collect()));
        } else if c == '(' {
            let mut depth = 1;
            let start = i + 1;
            i += 1;
            while i < chars.len() && depth > 0 {
                match chars[i] {
                    '(' | '<' | '[' => depth += 1,
                    ')' | '>' | ']' => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            if depth != 0 {
                return Err("an unclosed `(` in a message's payload".into());
            }
            let inner: String = chars[start..i - 1].iter().collect();
            out.push(Tok::Paren(inner.split_whitespace().collect::<Vec<_>>().join(" ").replace(" :: ", "::").replace(":: ", "::").replace(" ::", "::")));
        } else if c == ':' {
            out.push(Tok::Colon);
            i += 1;
        } else if c == '-' && i + 1 < chars.len() && chars[i + 1] == '>' {
            out.push(Tok::Arrow);
            i += 2;
        } else if c == ';' {
            out.push(Tok::Semi);
            i += 1;
        } else if c == '{' {
            out.push(Tok::LBrace);
            i += 1;
        } else if c == '}' {
            out.push(Tok::RBrace);
            i += 1;
        } else {
            return Err(format!("unexpected character `{c}` in the Protocol"));
        }
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn expect(&mut self, t: Tok, what: &str) -> Result<(), String> {
        match self.next() {
            Some(x) if x == t => Ok(()),
            Some(x) => Err(format!("expected {what}, found {x:?}")),
            None => Err(format!("expected {what}, found the end of the Protocol")),
        }
    }

    fn ident(&mut self, what: &str) -> Result<String, String> {
        match self.next() {
            Some(Tok::Ident(s)) => Ok(s),
            Some(x) => Err(format!("expected {what}, found {x:?}")),
            None => Err(format!("expected {what}, found the end of the Protocol")),
        }
    }

    /// A block: statements up to its terminator or the closing brace.
    fn block(&mut self) -> Result<Global, String> {
        match self.peek() {
            None | Some(Tok::RBrace) => Err("a block must end with `end;`, `continue NAME;`, a `choice`, or a `rec`".into()),
            Some(Tok::Ident(k)) if k == "end" => {
                self.next();
                if self.peek() == Some(&Tok::Semi) {
                    self.next();
                }
                self.end_of_block()?;
                Ok(Global::End)
            }
            Some(Tok::Ident(k)) if k == "continue" => {
                self.next();
                let name = self.ident("the name of the enclosing `rec`")?;
                if self.peek() == Some(&Tok::Semi) {
                    self.next();
                }
                self.end_of_block()?;
                Ok(Global::Continue(name))
            }
            Some(Tok::Ident(k)) if k == "rec" => {
                self.next();
                let name = self.ident("the `rec` name")?;
                self.expect(Tok::LBrace, "`{` after the `rec` name")?;
                let body = self.block()?;
                self.expect(Tok::RBrace, "`}` closing the `rec`")?;
                self.end_of_block()?;
                Ok(Global::Rec { name, body: Box::new(body) })
            }
            Some(Tok::Ident(k)) if k == "choice" => {
                self.next();
                let at_kw = self.ident("`at`")?;
                if at_kw != "at" {
                    return Err(format!("expected `at` after `choice`, found `{at_kw}`"));
                }
                let at = self.ident("the choosing role")?;
                let mut branches = Vec::new();
                loop {
                    self.expect(Tok::LBrace, "`{` opening a branch of the choice")?;
                    branches.push(self.block()?);
                    self.expect(Tok::RBrace, "`}` closing a branch of the choice")?;
                    match self.peek() {
                        Some(Tok::Ident(k)) if k == "or" => {
                            self.next();
                        }
                        _ => break,
                    }
                }
                self.end_of_block()?;
                Ok(Global::Choice { at, branches })
            }
            Some(Tok::Ident(_)) => {
                // Label(Type): from -> to; rest
                let label = self.ident("a message label")?;
                let payload = match self.peek() {
                    Some(Tok::Paren(p)) => {
                        let p = p.clone();
                        self.next();
                        if p.is_empty() { None } else { Some(p) }
                    }
                    _ => return Err(format!("message `{label}` needs its payload in parentheses, `{label}(Type)` or `{label}()`")),
                };
                self.expect(Tok::Colon, &format!("`:` after `{label}(..)`"))?;
                let from = self.ident("the sending role")?;
                self.expect(Tok::Arrow, "`->` between the roles")?;
                let to = self.ident("the receiving role")?;
                self.expect(Tok::Semi, &format!("`;` after `{label}(..): {from} -> {to}`"))?;
                let rest = self.block()?;
                Ok(Global::Msg(Message { label, payload, from, to }, Box::new(rest)))
            }
            Some(t) => Err(format!("unexpected {t:?} at the start of a statement")),
        }
    }

    /// After a terminator nothing follows in the block.
    fn end_of_block(&self) -> Result<(), String> {
        match self.peek() {
            None | Some(Tok::RBrace) => Ok(()),
            Some(t) => Err(format!("unexpected {t:?} after the end of a block (an `end`, `continue`, `choice`, or `rec` closes it)")),
        }
    }
}

/// Parses a Protocol body. The AST is the verified lexer and parser's
/// (`lex_v::parse_text`, 2026-11-09: the tokens a text denotes and the
/// grammar as specifications, the lexer and the parser refining them,
/// printing then parsing the identity); this module's own lexer and parser
/// are run for the diagnosis when the verified ones refuse, and a
/// disagreement between the two is reported as such. The one step outside
/// the proof is the standard library's decoding of the `str` into its
/// characters (`lex_v::chars_of`).
pub fn parse(text: &str) -> Result<Global, String> {
    match crate::lex_v::parse_text(text) {
        Some(g) => Ok(g),
        None => {
            let toks = tokens(text)?;
            let mut p = Parser { toks, pos: 0 };
            let g = p.block()?;
            if p.pos != p.toks.len() {
                return Err("tokens follow the end of the Protocol".into());
            }
            let _ = g;
            Err("the verified parser refuses the Protocol the diagnostic parser accepts; the two disagree".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_request_response_loop() {
        let g = parse("rec Loop { choice at ctl { Fix(u32): ctl -> sink; Ack(u32): sink -> ctl; continue Loop; } or { Stop(): ctl -> sink; end; } }").unwrap();
        assert_eq!(g.roles(), vec!["ctl".to_string(), "sink".to_string()]);
        assert!(g.render().starts_with("rec Loop { choice at ctl { Fix(u32): ctl -> sink; Ack(u32): sink -> ctl; continue Loop } or { Stop(): ctl -> sink; end } }"));
    }

    #[test]
    fn payload_paths_keep_their_shape() {
        let g = parse("M(app :: Fix < u8 >): a -> b; end").unwrap();
        match g {
            Global::Msg(m, _) => assert_eq!(m.payload.as_deref(), Some("app::Fix < u8 >")),
            _ => panic!(),
        }
    }

    #[test]
    fn rejects_an_unterminated_block_and_a_missing_payload() {
        assert!(parse("M(u8): a -> b;").unwrap_err().contains("must end"));
        assert!(parse("M: a -> b; end").unwrap_err().contains("parentheses"));
        assert!(parse("end; M(u8): a -> b; end").unwrap_err().contains("after the end of a block"));
    }
}
