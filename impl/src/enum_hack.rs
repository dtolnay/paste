use proc_macro::{TokenStream, TokenTree};
use quote::quote;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn wrap(output: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let mut hasher = DefaultHasher::default();
    output.to_string().hash(&mut hasher);
    let mangled_name = format!("_paste_{}", hasher.finish());
    let ident = proc_macro2::Ident::new(&mangled_name, proc_macro2::Span::call_site());

    quote! {
        #[derive(paste::EnumHack)]
        enum #ident {
            Value = (stringify! {
                #output
            }, 0).1,
        }
    }
}

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
