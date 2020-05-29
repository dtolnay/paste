extern crate proc_macro;

mod enum_hack;

use proc_macro2::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};
use proc_macro_hack::proc_macro_hack;
use quote::{quote, ToTokens};
use std::iter::FromIterator;
use syn::parse::{Error, Parse, ParseStream, Parser, Result};
use syn::{parenthesized, parse_macro_input, Lit, LitStr, Token};

#[proc_macro]
pub fn item(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as PasteInput);
    proc_macro::TokenStream::from(input.expanded)
}

#[proc_macro]
pub fn item_with_macros(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as PasteInput);
    proc_macro::TokenStream::from(enum_hack::wrap(input.expanded))
}

#[proc_macro_hack]
pub fn expr(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as PasteInput);
    let output = input.expanded;
    proc_macro::TokenStream::from(quote!({ #output }))
}

#[doc(hidden)]
#[proc_macro_derive(EnumHack)]
pub fn enum_hack(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    enum_hack::extract(input)
}

struct PasteInput {
    expanded: TokenStream,
}

impl Parse for PasteInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut contains_paste = false;
        let expanded = parse(input, &mut contains_paste)?;
        Ok(PasteInput { expanded })
    }
}

fn parse(input: ParseStream, contains_paste: &mut bool) -> Result<TokenStream> {
    let mut expanded = TokenStream::new();
    let (mut prev_colons, mut colons) = (false, false);
    while !input.is_empty() {
        let save = input.fork();
        match input.parse()? {
            TokenTree::Group(group) => {
                let delimiter = group.delimiter();
                let content = group.stream();
                let span = group.span();
                if delimiter == Delimiter::Bracket && is_paste_operation(&content) {
                    let segments = parse_bracket_as_segments.parse2(content)?;
                    let pasted = paste_segments(span, &segments)?;
                    pasted.to_tokens(&mut expanded);
                    *contains_paste = true;
                } else if is_none_delimited_flat_group(delimiter, &content) {
                    content.to_tokens(&mut expanded);
                    *contains_paste = true;
                } else {
                    let mut group_contains_paste = false;
                    let nested = (|input: ParseStream| parse(input, &mut group_contains_paste))
                        .parse2(content)?;
                    let group = if group_contains_paste {
                        let mut group = Group::new(delimiter, nested);
                        group.set_span(span);
                        *contains_paste = true;
                        group
                    } else {
                        group.clone()
                    };
                    let in_path = prev_colons || input.peek(Token![::]);
                    if in_path && delimiter == Delimiter::None {
                        group.stream().to_tokens(&mut expanded);
                        *contains_paste = true;
                    } else {
                        group.to_tokens(&mut expanded);
                    }
                }
            }
            other => other.to_tokens(&mut expanded),
        }
        prev_colons = colons;
        colons = save.peek(Token![::]);
    }
    Ok(expanded)
}

fn is_paste_operation(input: &TokenStream) -> bool {
    let input = input.clone();
    parse_bracket_as_segments.parse2(input).is_ok()
}

// https://github.com/dtolnay/paste/issues/26
fn is_none_delimited_flat_group(delimiter: Delimiter, input: &TokenStream) -> bool {
    if delimiter != Delimiter::None {
        return false;
    }

    #[derive(PartialEq)]
    enum State {
        Init,
        Ident,
        Literal,
        Apostrophe,
        Lifetime,
        Colon1,
        Colon2,
    }

    let mut state = State::Init;
    for tt in input.clone() {
        state = match (state, &tt) {
            (State::Init, TokenTree::Ident(_)) => State::Ident,
            (State::Init, TokenTree::Literal(_)) => State::Literal,
            (State::Init, TokenTree::Punct(punct)) if punct.as_char() == '\'' => State::Apostrophe,
            (State::Apostrophe, TokenTree::Ident(_)) => State::Lifetime,
            (State::Ident, TokenTree::Punct(punct))
                if punct.as_char() == ':' && punct.spacing() == Spacing::Joint =>
            {
                State::Colon1
            }
            (State::Colon1, TokenTree::Punct(punct))
                if punct.as_char() == ':' && punct.spacing() == Spacing::Alone =>
            {
                State::Colon2
            }
            (State::Colon2, TokenTree::Ident(_)) => State::Ident,
            _ => return false,
        };
    }

    state == State::Ident || state == State::Literal || state == State::Lifetime
}

enum Segment {
    String(String),
    Apostrophe(Span),
    Env(LitStr),
    Modifier(Token![:], Ident),
}

fn parse_bracket_as_segments(input: ParseStream) -> Result<Vec<Segment>> {
    input.parse::<Token![<]>()?;

    let segments = parse_segments(input)?;

    input.parse::<Token![>]>()?;
    if !input.is_empty() {
        return Err(input.error("invalid input"));
    }
    Ok(segments)
}

fn parse_segments(input: ParseStream) -> Result<Vec<Segment>> {
    let mut segments = Vec::new();
    while !(input.is_empty() || input.peek(Token![>])) {
        match input.parse()? {
            TokenTree::Ident(ident) => {
                let mut fragment = ident.to_string();
                if fragment.starts_with("r#") {
                    fragment = fragment.split_off(2);
                }
                if fragment == "env" && input.peek(Token![!]) {
                    input.parse::<Token![!]>()?;
                    let arg;
                    parenthesized!(arg in input);
                    let var: LitStr = arg.parse()?;
                    segments.push(Segment::Env(var));
                } else {
                    segments.push(Segment::String(fragment));
                }
            }
            TokenTree::Literal(lit) => {
                let value = match syn::parse_str(&lit.to_string())? {
                    Lit::Str(string) => string.value().replace('-', "_"),
                    Lit::Int(_) => lit.to_string(),
                    _ => return Err(Error::new(lit.span(), "unsupported literal")),
                };
                segments.push(Segment::String(value));
            }
            TokenTree::Punct(punct) => match punct.as_char() {
                '_' => segments.push(Segment::String("_".to_string())),
                '\'' => segments.push(Segment::Apostrophe(punct.span())),
                ':' => segments.push(Segment::Modifier(Token![:](punct.span()), input.parse()?)),
                _ => return Err(Error::new(punct.span(), "unexpected punct")),
            },
            TokenTree::Group(group) => {
                if group.delimiter() == Delimiter::None {
                    let nested = parse_segments.parse2(group.stream())?;
                    segments.extend(nested);
                } else {
                    return Err(Error::new(group.span(), "unexpected token"));
                }
            }
        }
    }
    Ok(segments)
}

fn paste_segments(span: Span, segments: &[Segment]) -> Result<TokenStream> {
    let mut evaluated = Vec::new();
    let mut is_lifetime = false;

    for segment in segments {
        match segment {
            Segment::String(segment) => {
                evaluated.push(segment.clone());
            }
            Segment::Apostrophe(span) => {
                if is_lifetime {
                    return Err(Error::new(*span, "unexpected lifetime"));
                }
                is_lifetime = true;
            }
            Segment::Env(var) => {
                let resolved = match std::env::var(var.value()) {
                    Ok(resolved) => resolved,
                    Err(_) => {
                        return Err(Error::new(var.span(), "no such env var"));
                    }
                };
                let resolved = resolved.replace('-', "_");
                evaluated.push(resolved);
            }
            Segment::Modifier(colon, ident) => {
                let span = quote!(#colon #ident);
                let last = match evaluated.pop() {
                    Some(last) => last,
                    None => return Err(Error::new_spanned(span, "unexpected modifier")),
                };
                if ident == "lower" {
                    evaluated.push(last.to_lowercase());
                } else if ident == "upper" {
                    evaluated.push(last.to_uppercase());
                } else if ident == "snake" {
                    let mut acc = String::new();
                    let mut prev = '_';
                    for ch in last.chars() {
                        if ch.is_uppercase() && prev != '_' {
                            acc.push('_');
                        }
                        acc.push(ch);
                        prev = ch;
                    }
                    evaluated.push(acc.to_lowercase());
                } else if ident == "camel" {
                    let mut acc = String::new();
                    let mut prev = '_';
                    for ch in last.chars() {
                        if ch != '_' {
                            if prev == '_' {
                                for chu in ch.to_uppercase() {
                                    acc.push(chu);
                                }
                            } else if prev.is_uppercase() {
                                for chl in ch.to_lowercase() {
                                    acc.push(chl);
                                }
                            } else {
                                acc.push(ch);
                            }
                        }
                        prev = ch;
                    }
                    evaluated.push(acc);
                } else {
                    return Err(Error::new_spanned(span, "unsupported modifier"));
                }
            }
        }
    }

    let pasted = evaluated.into_iter().collect::<String>();
    let ident = TokenTree::Ident(Ident::new(&pasted, span));
    let tokens = if is_lifetime {
        let apostrophe = TokenTree::Punct(Punct::new('\'', Spacing::Joint));
        vec![apostrophe, ident]
    } else {
        vec![ident]
    };
    Ok(TokenStream::from_iter(tokens))
}
