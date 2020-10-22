use crate::error::Result;
use crate::segment::{self, Segment};
use proc_macro::{Delimiter, Span, TokenStream, TokenTree};
use std::iter;
use std::mem;
use std::str::FromStr;

pub fn is_pasted_doc(input: &TokenStream) -> bool {
    #[derive(PartialEq)]
    enum State {
        Init,
        Doc,
        Equal,
        First,
        Rest,
    }

    let mut state = State::Init;
    for tt in input.clone() {
        state = match (state, &tt) {
            (State::Init, TokenTree::Ident(ident)) if ident.to_string() == "doc" => State::Doc,
            (State::Doc, TokenTree::Punct(punct)) if punct.as_char() == '=' => State::Equal,
            (State::Equal, tt) if is_stringlike(tt) => State::First,
            (State::First, tt) | (State::Rest, tt) if is_stringlike(tt) => State::Rest,
            _ => return false,
        };
    }

    state == State::Rest
}

pub fn do_paste_doc(attr: &TokenStream, span: Span) -> Result<TokenStream> {
    let mut expanded = TokenStream::new();
    let mut tokens = attr.clone().into_iter().peekable();
    expanded.extend(tokens.by_ref().take(2)); // `doc =`

    let mut segments = segment::parse(&mut tokens)?;

    for segment in &mut segments {
        if let Segment::String(string) = segment {
            if let Some(open_quote) = string.value.find('"') {
                if open_quote == 0 {
                    string.value.truncate(string.value.len() - 1);
                    string.value.remove(0);
                } else {
                    let begin = open_quote + 1;
                    let end = string.value.rfind('"').unwrap();
                    let raw_string = mem::replace(&mut string.value, String::new());
                    for ch in raw_string[begin..end].chars() {
                        string.value.extend(ch.escape_default());
                    }
                }
            }
        }
    }

    let mut lit = segment::paste(&segments)?;
    lit.insert(0, '"');
    lit.push('"');

    let mut lit = TokenStream::from_str(&lit)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    lit.set_span(span);
    expanded.extend(iter::once(lit));
    Ok(expanded)
}

fn is_stringlike(token: &TokenTree) -> bool {
    match token {
        TokenTree::Ident(_) => true,
        TokenTree::Literal(literal) => {
            let repr = literal.to_string();
            !repr.starts_with('b') && !repr.starts_with('\'')
        }
        TokenTree::Group(group) => {
            if group.delimiter() != Delimiter::None {
                return false;
            }
            let mut inner = group.stream().into_iter();
            match inner.next() {
                Some(first) => inner.next().is_none() && is_stringlike(&first),
                None => false,
            }
        }
        TokenTree::Punct(punct) => punct.as_char() == '\'' || punct.as_char() == ':',
    }
}
