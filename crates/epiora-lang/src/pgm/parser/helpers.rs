//! Shared token-walking helpers: `qid` lists, `NAME =` option
//! values and the register-name resolver used by both `unsup` and
//! the statement dispatch.

use super::{Lexed, Parser, Tok};
use crate::pgm::register::register;

impl<'a> Parser<'a> {
    /// A qualified-identifier list at `toks[j]` → (names, next j).
    /// Commas are skipped; the list ends before a `stop`-word or at
    /// a `NAME =` option boundary.
    pub(crate) fn idlist(&self, toks: &[Lexed], j: usize, stop: &[&str]) -> (Vec<String>, usize) {
        let mut names = Vec::new();
        let mut k = j;
        loop {
            if matches!(toks.get(k).map(|t| &t.tok), Some(Tok::Comma)) {
                k += 1;
            }
            let Some((q, end)) = self.qid(toks, k) else {
                break;
            };
            if stop.iter().any(|w| q.eq_ignore_ascii_case(w))
                || matches!(toks.get(end).map(|t| &t.tok), Some(Tok::Eq))
            {
                break;
            }
            names.push(q);
            k = end;
        }
        (names, k)
    }

    /// The value of a `NAME =` option starting at `toks[j]`: a
    /// string, number, date/time, `*` or an identifier list
    /// (`STRATAVAR = a b`); an identifier list ends at the next
    /// `NAME =` or EOL → (JSON value, next j).
    pub(crate) fn opt_value(&self, toks: &[Lexed], j: usize) -> (Option<serde_json::Value>, usize) {
        use serde_json::{json, Value as J};
        match toks.get(j).map(|t| &t.tok) {
            Some(Tok::Str(s)) | Some(Tok::File(s)) | Some(Tok::Brace(s)) => {
                (Some(J::String(s.clone())), j + 1)
            }
            Some(Tok::Int(v)) => (Some(json!(v)), j + 1),
            Some(Tok::Real(v)) => (Some(json!(v)), j + 1),
            Some(Tok::Date(_)) | Some(Tok::Time(_)) => (
                Some(J::String(self.span(toks, j, j + 1).to_string())),
                j + 1,
            ),
            Some(Tok::Star) => (Some(J::String("*".into())), j + 1),
            Some(Tok::Ident(_)) => {
                let (names, end) = self.idlist(toks, j, &[]);
                match names.len() {
                    0 => (None, j),
                    1 => (Some(J::String(names[0].clone())), end),
                    _ => (Some(json!(names)), end),
                }
            }
            _ => (None, j),
        }
    }

    /// The longest register command name that is a word-prefix of
    /// this statement: `CANCEL READ` → `CANCEL READ`, hyphen-joined
    /// (`SAVE-RECORD`), else the bare head if register-known.
    pub(crate) fn register_name(&self, toks: &[Lexed]) -> Option<String> {
        let reg = register();
        // Head words: `Ident` tokens, where `Ident - Ident` joins
        // into one hyphenated word.
        let mut words: Vec<String> = Vec::new();
        let mut t = 0usize;
        while words.len() < 2 {
            let Some(Tok::Ident(w)) = toks.get(t).map(|x| &x.tok) else {
                break;
            };
            let mut word = w.to_ascii_uppercase();
            t += 1;
            while matches!(toks.get(t).map(|x| &x.tok), Some(Tok::Minus))
                && matches!(toks.get(t + 1).map(|x| &x.tok), Some(Tok::Ident(_)))
            {
                let Tok::Ident(w2) = &toks[t + 1].tok else {
                    break;
                };
                word.push('-');
                word.push_str(&w2.to_ascii_uppercase());
                t += 2;
            }
            words.push(word);
        }
        if words.len() >= 2 {
            let two = format!("{} {}", words[0], words[1]);
            if reg.command(&two).is_some() {
                return Some(two);
            }
        }
        if let Some(w) = words.first() {
            if reg.command(w).is_some() {
                return Some(w.clone());
            }
        }
        None
    }
}
