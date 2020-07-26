use proc_macro::{Delimiter, Span, TokenStream, TokenTree};
use std::iter;
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

pub fn do_paste_doc(attr: &TokenStream, span: Span) -> TokenStream {
    let mut expanded = TokenStream::new();
    let mut tokens = attr.clone().into_iter();
    expanded.extend(tokens.by_ref().take(2)); // `doc =`

    let mut lit = String::new();
    lit.push('"');
    for token in tokens {
        lit += &escaped_string_value(&token).unwrap();
    }
    lit.push('"');

    let mut lit = TokenStream::from_str(&lit)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    lit.set_span(span);
    expanded.extend(iter::once(lit));
    expanded
}

fn is_stringlike(token: &TokenTree) -> bool {
    escaped_string_value(token).is_some()
}

fn escaped_string_value(token: &TokenTree) -> Option<String> {
    match token {
        TokenTree::Ident(ident) => Some(ident.to_string()),
        TokenTree::Literal(literal) => {
            let mut repr = literal.to_string();
            if repr.starts_with('b') || repr.starts_with('\'') {
                None
            } else if repr.starts_with('"') {
                repr.truncate(repr.len() - 1);
                repr.remove(0);
                Some(repr)
            } else if repr.starts_with('r') {
                let begin = repr.find('"').unwrap() + 1;
                let end = repr.rfind('"').unwrap();
                let mut escaped = String::new();
                for ch in repr[begin..end].chars() {
                    escaped.extend(ch.escape_default());
                }
                Some(escaped)
            } else {
                Some(repr)
            }
        }
        TokenTree::Group(group) => {
            if group.delimiter() != Delimiter::None {
                return None;
            }
            let mut inner = group.stream().into_iter();
            let first = inner.next()?;
            if inner.next().is_none() {
                escaped_string_value(&first)
            } else {
                None
            }
        }
        TokenTree::Punct(_) => None,
    }
}
