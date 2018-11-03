extern crate proc_macro;
extern crate proc_macro2;
extern crate proc_macro_hack;
extern crate quote;
extern crate syn;

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

#[proc_macro_hack]
pub fn expr(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as PasteInput);
    let output = input.expanded;
    proc_macro::TokenStream::from(quote!({ #output }))
}

struct PasteInput {
    expanded: TokenStream,
}

impl Parse for PasteInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut expanded = TokenStream::new();
        while !input.is_empty() {
            match input.parse()? {
                TokenTree::Group(group) => {
                    let delimiter = group.delimiter();
                    let content = group.stream();
                    let span = group.span();
                    if delimiter == Delimiter::Bracket && is_paste_operation(&content) {
                        let segments = parse_bracket_as_segments.parse2(content)?;
                        let pasted = paste_segments(span, &segments)?;
                        pasted.to_tokens(&mut expanded);
                    } else {
                        let nested = PasteInput::parse.parse2(content)?;
                        let mut group = Group::new(delimiter, nested.expanded);
                        group.set_span(span);
                        group.to_tokens(&mut expanded);
                    }
                }
                other => other.to_tokens(&mut expanded),
            }
        }
        Ok(PasteInput { expanded })
    }
}

fn is_paste_operation(input: &TokenStream) -> bool {
    let input = input.clone();
    parse_bracket_as_segments.parse2(input).is_ok()
}

enum Segment {
    String(String),
    Apostrophe(Span),
    Env(LitStr),
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
    let mut pasted = String::new();
    let mut is_lifetime = false;

    for segment in segments {
        match segment {
            Segment::String(segment) => {
                pasted.push_str(&segment);
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
                pasted.push_str(&resolved);
            }
        }
    }

    let ident = TokenTree::Ident(Ident::new(&pasted, span));
    let tokens = if is_lifetime {
        let apostrophe = TokenTree::Punct(Punct::new('\'', Spacing::Joint));
        vec![apostrophe, ident]
    } else {
        vec![ident]
    };
    Ok(TokenStream::from_iter(tokens))
}
