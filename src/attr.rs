use crate::error::Result;
use crate::segment::{self, Segment};
use proc_macro::{Delimiter, Group, Span, TokenStream, TokenTree};
use std::iter;
use std::mem;
use std::str::FromStr;

pub fn expand_attr(
    attr: TokenStream,
    span: Span,
    contains_paste: &mut bool,
) -> Result<TokenStream> {
    let mut tokens = attr.clone().into_iter();
    match tokens.next() {
        Some(TokenTree::Ident(..)) => {}
        _ => return Ok(attr),
    }

    let group = match tokens.next() {
        Some(TokenTree::Punct(punct)) if punct.as_char() == '=' => {
            let mut count = 0;
            if tokens.inspect(|_| count += 1).all(|tt| is_stringlike(&tt)) {
                if count > 1 {
                    *contains_paste = true;
                    return do_paste_name_value_attr(attr, span);
                }
            }
            return Ok(attr);
        }
        Some(TokenTree::Group(group)) => group,
        _ => return Ok(attr),
    };

    if group.delimiter() != Delimiter::Parenthesis {
        return Ok(attr);
    }

    // There can't be anything else after the first group in a valid attribute.
    if tokens.next().is_some() {
        return Ok(attr);
    }

    let mut group_contains_paste = false;
    let mut expanded = TokenStream::new();
    let mut nested_attr = TokenStream::new();
    for tt in group.stream().into_iter() {
        match &tt {
            TokenTree::Punct(punct) if punct.as_char() == ',' => {
                expanded.extend(expand_attr(
                    nested_attr,
                    group.span(),
                    &mut group_contains_paste,
                )?);
                expanded.extend(iter::once(tt));
                nested_attr = TokenStream::new();
            }
            _ => nested_attr.extend(iter::once(tt)),
        }
    }

    if !nested_attr.is_empty() {
        expanded.extend(expand_attr(
            nested_attr,
            group.span(),
            &mut group_contains_paste,
        )?);
    }

    if group_contains_paste {
        *contains_paste = true;
        let mut group = Group::new(Delimiter::Parenthesis, expanded);
        group.set_span(span);
        Ok(attr
            .into_iter()
            // Just keep the initial ident in `#[ident(...)]`.
            .take(1)
            .chain(iter::once(TokenTree::Group(group)))
            .collect())
    } else {
        Ok(attr)
    }
}

fn do_paste_name_value_attr(attr: TokenStream, span: Span) -> Result<TokenStream> {
    let mut expanded = TokenStream::new();
    let mut tokens = attr.into_iter().peekable();
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
