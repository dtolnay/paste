use crate::error::{Error, Result};
use proc_macro::{token_stream, Delimiter, Ident, Span, TokenTree};
use std::iter::Peekable;

pub(crate) enum Segment {
    String(String),
    Apostrophe(Span),
    Env(LitStr),
    Modifier(Colon, Ident),
}

pub(crate) struct LitStr {
    pub value: String,
    pub span: Span,
}

pub(crate) struct Colon {
    pub span: Span,
}

pub(crate) fn parse(tokens: &mut Peekable<token_stream::IntoIter>) -> Result<Vec<Segment>> {
    let mut segments = Vec::new();
    while match tokens.peek() {
        None => false,
        Some(TokenTree::Punct(punct)) => punct.as_char() != '>',
        Some(_) => true,
    } {
        match tokens.next().unwrap() {
            TokenTree::Ident(ident) => {
                let mut fragment = ident.to_string();
                if fragment.starts_with("r#") {
                    fragment = fragment.split_off(2);
                }
                if fragment == "env"
                    && match tokens.peek() {
                        Some(TokenTree::Punct(punct)) => punct.as_char() == '!',
                        _ => false,
                    }
                {
                    let bang = tokens.next().unwrap(); // `!`
                    let expect_group = tokens.next();
                    let parenthesized = match &expect_group {
                        Some(TokenTree::Group(group))
                            if group.delimiter() == Delimiter::Parenthesis =>
                        {
                            group
                        }
                        Some(wrong) => return Err(Error::new(wrong.span(), "expected `(`")),
                        None => {
                            return Err(Error::new2(
                                ident.span(),
                                bang.span(),
                                "expected `(` after `env!`",
                            ));
                        }
                    };
                    let mut inner = parenthesized.stream().into_iter();
                    let lit = match inner.next() {
                        Some(TokenTree::Literal(lit)) => lit,
                        Some(wrong) => {
                            return Err(Error::new(wrong.span(), "expected string literal"))
                        }
                        None => {
                            return Err(Error::new2(
                                ident.span(),
                                parenthesized.span(),
                                "expected string literal as argument to env! macro",
                            ))
                        }
                    };
                    let lit_string = lit.to_string();
                    if lit_string.starts_with('"')
                        && lit_string.ends_with('"')
                        && lit_string.len() >= 2
                    {
                        // TODO: maybe handle escape sequences in the string if
                        // someone has a use case.
                        segments.push(Segment::Env(LitStr {
                            value: lit_string[1..lit_string.len() - 1].to_owned(),
                            span: lit.span(),
                        }));
                    } else {
                        return Err(Error::new(lit.span(), "expected string literal"));
                    }
                    if let Some(unexpected) = inner.next() {
                        return Err(Error::new(
                            unexpected.span(),
                            "unexpected token in env! macro",
                        ));
                    }
                } else {
                    segments.push(Segment::String(fragment));
                }
            }
            TokenTree::Literal(lit) => {
                let mut lit_string = lit.to_string();
                if lit_string.contains(&['#', '\\', '.', '+'][..]) {
                    return Err(Error::new(lit.span(), "unsupported literal"));
                }
                lit_string = lit_string
                    .replace('"', "")
                    .replace('\'', "")
                    .replace('-', "_");
                segments.push(Segment::String(lit_string));
            }
            TokenTree::Punct(punct) => match punct.as_char() {
                '_' => segments.push(Segment::String("_".to_owned())),
                '\'' => segments.push(Segment::Apostrophe(punct.span())),
                ':' => {
                    let colon_span = punct.span();
                    let colon = Colon { span: colon_span };
                    let ident = match tokens.next() {
                        Some(TokenTree::Ident(ident)) => ident,
                        wrong => {
                            let span = wrong.as_ref().map_or(colon_span, TokenTree::span);
                            return Err(Error::new(span, "expected identifier after `:`"));
                        }
                    };
                    segments.push(Segment::Modifier(colon, ident));
                }
                _ => return Err(Error::new(punct.span(), "unexpected punct")),
            },
            TokenTree::Group(group) => {
                if group.delimiter() == Delimiter::None {
                    let mut inner = group.stream().into_iter().peekable();
                    let nested = parse(&mut inner)?;
                    if let Some(unexpected) = inner.next() {
                        return Err(Error::new(unexpected.span(), "unexpected token"));
                    }
                    segments.extend(nested);
                } else {
                    return Err(Error::new(group.span(), "unexpected token"));
                }
            }
        }
    }
    Ok(segments)
}
