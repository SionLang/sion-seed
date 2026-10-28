//! sion-seed: throwaway bootstrap compiler for Sion-0. Stage 0: lexer.
use std::{env, fs, process};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    TypeId(String),
    Kw(&'static str),
    Int(String),
    Float(String),
    Str(String),
    Char(char),
    Op(&'static str),
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug)]
pub struct LexError {
    pub code: &'static str,
    pub msg: String,
    pub line: usize,
    pub col: usize,
}

const KEYWORDS: &[&str] = &[
    "fn", "type", "let", "mut", "if", "else", "match", "for", "in", "while", "loop", "break",
    "continue", "return", "use", "pub", "mod", "as", "test", "need", "give", "uses", "effect",
    "trait", "impl", "const", "move", "and", "or", "not", "true", "false", "some", "none", "ok",
    "err", "spawn", "then", "band", "bor", "bxor", "shl", "shr",
];
const OPS3: &[&str] = &["..="];
const OPS2: &[&str] = &[
    "..", "=>", "->", "==", "!=", "<=", ">=", "+%", "-%", "*%", "+|", "-|", "*|", "+=", "-=",
    "*=", "/=",
];
const OPS1: &[&str] = &[
    "+", "-", "*", "/", "%", "=", "<", ">", "!", "?", ".", ",", ":", ";", "(", ")", "[", "]", "{",
    "}", "&", "|",
];
/// A line ending in one of these continues on the next line.
const CONT_OPS: &[&str] = &[",", "+", "-", "*", "/", "%", "==", "!=", "<=", ">="];

fn tk(tok: Tok, line: usize, col: usize) -> Token {
    Token { tok, line, col }
}

fn err(code: &'static str, msg: &str, line: usize, col: usize) -> LexError {
    LexError { code, msg: msg.to_string(), line, col }
}

fn match_op(chars: &[char], i: usize, n: usize, list: &[&'static str]) -> Option<&'static str> {
    if i + n > chars.len() {
        return None;
    }
    let s: String = chars[i..i + n].iter().collect();
    list.iter().copied().find(|o| *o == s)
}

fn continues(out: &[Token]) -> bool {
    match out.last().map(|t| &t.tok) {
        Some(Tok::Op(o)) => CONT_OPS.contains(o),
        Some(Tok::Kw(k)) => *k == "and" || *k == "or",
        _ => false,
    }
}

fn escape(c: char, line: usize, col: usize) -> Result<char, LexError> {
    match c {
        'n' => Ok('\n'),
        't' => Ok('\t'),
        '\\' => Ok('\\'),
        '"' => Ok('"'),
        '\'' => Ok('\''),
        '0' => Ok('\0'),
        _ => Err(err("E006", "unknown escape sequence", line, col)),
    }
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let chars: Vec<char> = src.chars().collect();
    let len = chars.len();
    let mut out: Vec<Token> = Vec::new();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut line_start = 0usize;
    let mut depth = 0usize;
    let mut indents: Vec<usize> = vec![0];
    let mut at_line_start = true;

    while i < len {
        if at_line_start && depth == 0 {
            let mut j = i;
            let mut spaces = 0usize;
            while j < len && chars[j] == ' ' {
                spaces += 1;
                j += 1;
            }
            if j < len && chars[j] == '\t' {
                return Err(err("E001", "tabs are not allowed; use 2 spaces", line, spaces + 1));
            }
            // blank line or comment-only line: skip entirely
            if j >= len || chars[j] == '\n' || chars[j] == '\r' || chars[j] == '#' {
                while j < len && chars[j] != '\n' {
                    j += 1;
                }
                if j < len {
                    j += 1;
                    line += 1;
                    line_start = j;
                }
                i = j;
                continue;
            }
            if spaces % 2 != 0 {
                return Err(err("E002", "indentation must be a multiple of 2 spaces", line, 1));
            }
            let level = spaces / 2;
            let cur = *indents.last().unwrap();
            if level > cur {
                if level != cur + 1 {
                    return Err(err("E002", "indent may increase by one level only", line, 1));
                }
                indents.push(level);
                out.push(tk(Tok::Indent, line, 1));
            } else {
                while level < *indents.last().unwrap() {
                    indents.pop();
                    out.push(tk(Tok::Dedent, line, 1));
                }
            }
            i = j;
            at_line_start = false;
            continue;
        }

        let c = chars[i];
        let col = i - line_start + 1;

        if c == ' ' || c == '\r' {
            i += 1;
        } else if c == '\t' {
            return Err(err("E001", "tabs are not allowed; use 2 spaces", line, col));
        } else if c == '#' {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '\n' {
            if depth == 0 && !continues(&out) {
                if let Some(t) = out.last() {
                    if !matches!(t.tok, Tok::Newline | Tok::Indent | Tok::Dedent) {
                        out.push(tk(Tok::Newline, line, col));
                    }
                }
                at_line_start = true;
            } else {
                at_line_start = false;
            }
            i += 1;
            line += 1;
            line_start = i;
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let tok = if c.is_uppercase() {
                Tok::TypeId(word)
            } else if let Some(k) = KEYWORDS.iter().copied().find(|k| *k == word) {
                Tok::Kw(k)
            } else {
                Tok::Ident(word)
            };
            out.push(tk(tok, line, col));
        } else if c.is_ascii_digit() {
            let start = i;
            let mut is_float = false;
            if c == '0' && i + 1 < len && (chars[i + 1] == 'x' || chars[i + 1] == 'b') {
                i += 2;
                while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
            } else {
                while i < len && (chars[i].is_ascii_digit() || chars[i] == '_') {
                    i += 1;
                }
                if i + 1 < len && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
                    is_float = true;
                    i += 1;
                    while i < len && (chars[i].is_ascii_digit() || chars[i] == '_') {
                        i += 1;
                    }
                }
                if i < len && (chars[i] == 'e' || chars[i] == 'E') {
                    let mut k = i + 1;
                    if k < len && (chars[k] == '+' || chars[k] == '-') {
                        k += 1;
                    }
                    if k < len && chars[k].is_ascii_digit() {
                        is_float = true;
                        i = k;
                        while i < len && chars[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                }
                while i < len && chars[i].is_ascii_alphanumeric() {
                    i += 1; // type suffix such as u8, i64, f32
                }
            }
            let text: String = chars[start..i].iter().collect();
            out.push(tk(if is_float { Tok::Float(text) } else { Tok::Int(text) }, line, col));
        } else if c == '"' {
            if i + 2 < len && chars[i + 1] == '"' && chars[i + 2] == '"' {
                return Err(err("E005", "multi-line strings are not in Sion-0", line, col));
            }
            i += 1;
            let mut s = String::new();
            loop {
                if i >= len || chars[i] == '\n' {
                    return Err(err("E003", "unterminated string", line, col));
                }
                let ch = chars[i];
                if ch == '"' {
                    i += 1;
                    break;
                }
                if ch == '\\' {
                    i += 1;
                    if i >= len {
                        return Err(err("E003", "unterminated string", line, col));
                    }
                    s.push(escape(chars[i], line, i - line_start + 1)?);
                    i += 1;
                } else {
                    s.push(ch);
                    i += 1;
                }
            }
            out.push(tk(Tok::Str(s), line, col));
        } else if c == '\'' {
            let (ch, close) = if i + 2 < len && chars[i + 1] == '\\' {
                (escape(chars[i + 2], line, col)?, i + 3)
            } else if i + 2 < len {
                (chars[i + 1], i + 2)
            } else {
                return Err(err("E003", "unterminated char literal", line, col));
            };
            if close >= len || chars[close] != '\'' {
                return Err(err("E003", "unterminated char literal", line, col));
            }
            i = close + 1;
            out.push(tk(Tok::Char(ch), line, col));
        } else {
            let op = match_op(&chars, i, 3, OPS3)
                .or_else(|| match_op(&chars, i, 2, OPS2))
                .or_else(|| match_op(&chars, i, 1, OPS1));
            match op {
                Some(o) => {
                    i += o.chars().count();
                    if o == "(" || o == "[" || o == "{" {
                        depth += 1;
                    } else if o == ")" || o == "]" || o == "}" {
                        if depth == 0 {
                            return Err(err("E004", "unbalanced closing bracket", line, col));
                        }
                        depth -= 1;
                    }
                    out.push(tk(Tok::Op(o), line, col));
                }
                None => {
                    return Err(err("E004", &format!("unexpected character '{}'", c), line, col));
                }
            }
        }
    }

    if depth != 0 {
        return Err(err("E004", "unclosed bracket at end of file", line, 1));
    }
    if let Some(t) = out.last() {
        if !matches!(t.tok, Tok::Newline | Tok::Indent | Tok::Dedent) {
            out.push(tk(Tok::Newline, line, 1));
        }
    }
    while indents.len() > 1 {
        indents.pop();
        out.push(tk(Tok::Dedent, line, 1));
    }
    out.push(tk(Tok::Eof, line, 1));
    Ok(out)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "lex" {
        eprintln!("usage: sion-seed lex <file.sn>");
        process::exit(2);
    }
    let src = match fs::read_to_string(&args[2]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {}: {}", args[2], e);
            process::exit(2);
        }
    };
    match lex(&src) {
        Ok(tokens) => {
            for t in tokens {
                println!("{}:{}\t{:?}", t.line, t.col, t.tok);
            }
        }
        Err(e) => {
            // diagnostics are JSON, same shape as the final compiler
            println!(
                "{{\"code\":\"{}\",\"sev\":\"error\",\"file\":\"{}\",\"span\":[{},{}],\"msg\":\"{}\"}}",
                e.code,
                args[2].replace('\\', "/"),
                e.line,
                e.col,
                e.msg.replace('"', "'")
            );
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<Tok> {
        lex(src).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn indent_and_dedent() {
        assert_eq!(
            kinds("fn f() =\n  1\n"),
            vec![
                Tok::Kw("fn"),
                Tok::Ident("f".into()),
                Tok::Op("("),
                Tok::Op(")"),
                Tok::Op("="),
                Tok::Newline,
                Tok::Indent,
                Tok::Int("1".into()),
                Tok::Newline,
                Tok::Dedent,
                Tok::Eof
            ]
        );
    }

    #[test]
    fn tab_is_error() {
        assert_eq!(lex("\tx").unwrap_err().code, "E001");
    }

    #[test]
    fn odd_indent_is_error() {
        assert_eq!(lex("a\n b\n").unwrap_err().code, "E002");
    }

    #[test]
    fn range_is_not_float() {
        assert_eq!(
            kinds("1..5"),
            vec![Tok::Int("1".into()), Tok::Op(".."), Tok::Int("5".into()), Tok::Newline, Tok::Eof]
        );
    }

    #[test]
    fn wrapping_operator() {
        assert_eq!(
            kinds("a +% b"),
            vec![Tok::Ident("a".into()), Tok::Op("+%"), Tok::Ident("b".into()), Tok::Newline, Tok::Eof]
        );
    }

    #[test]
    fn newline_ignored_in_brackets() {
        assert_eq!(
            kinds("f(1,\n  2)"),
            vec![
                Tok::Ident("f".into()),
                Tok::Op("("),
                Tok::Int("1".into()),
                Tok::Op(","),
                Tok::Int("2".into()),
                Tok::Op(")"),
                Tok::Newline,
                Tok::Eof
            ]
        );
    }
}
