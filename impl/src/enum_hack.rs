use proc_macro::{TokenStream, TokenTree};

pub fn extract(input: TokenStream) -> TokenStream {
    let mut tokens = input.into_iter();
    let _ = tokens.next().expect("enum");
    let _ = tokens.next().expect("#ident");
    let mut braces = match tokens.next().expect("{...}") {
        TokenTree::Group(group) => group.stream().into_iter(),
        _ => unreachable!("{...}"),
    };
    let _ = braces.next().expect("Value");
    let _ = braces.next().expect("=");
    let mut parens = match braces.next().expect("(...)") {
        TokenTree::Group(group) => group.stream().into_iter(),
        _ => unreachable!("(...)"),
    };
    let _ = parens.next().expect("stringify");
    let _ = parens.next().expect("!");
    let token_stream = match parens.next().expect("{...}") {
        TokenTree::Group(group) => group.stream(),
        _ => unreachable!("{...}"),
    };
    let _ = parens.next().expect(",");
    let _ = parens.next().expect("0");
    let _ = braces.next().expect(".");
    let _ = braces.next().expect("1");
    let _ = braces.next().expect(",");
    token_stream
}
