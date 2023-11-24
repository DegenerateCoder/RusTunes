extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_derive(ActionToType)]
pub fn derive_action_to_type_fn(items: TokenStream) -> TokenStream {
    let mut token_stream = "#[derive(Debug)]\n".to_owned();
    let mut action_enum = None;
    let mut type_enum = "".to_owned();
    let mut variants_with_args = Vec::new();

    for item in items {
        match item {
            proc_macro::TokenTree::Ident(ident) => {
                let ident = ident.to_string();
                if ident.ends_with("Action") {
                    type_enum = ident.replace("Action", "Type");
                    action_enum = Some(ident);
                    token_stream = format!("{token_stream} {}", type_enum);
                } else {
                    token_stream = format!("{token_stream} {}", ident.to_string());
                }
            }
            proc_macro::TokenTree::Group(group) => {
                if group.delimiter() == proc_macro::Delimiter::Brace && action_enum.is_some() {
                    let group = group.stream();
                    let (processed_group, processed_variants) = proces_group(group);
                    variants_with_args = processed_variants;
                    token_stream = format!("{token_stream} {{ {processed_group} }} ",);
                } else {
                    break;
                }
            }
            proc_macro::TokenTree::Punct(punct) => {
                token_stream = format!("{token_stream} {}", punct.to_string());
            }
            proc_macro::TokenTree::Literal(_) => (),
        }
    }

    if action_enum.is_some() {
        let action_enum = action_enum.unwrap();

        token_stream = format!(
            "{token_stream}\n\nimpl {type_enum} {{\n pub fn map_type_to_action(&self, mut processed_args: Vec<Arg>) -> Option<{action_enum}>{{"
        );

        token_stream = format!("{token_stream}\nprocessed_args.reverse();\nlet t = match self {{");

        for variant_with_args in variants_with_args {
            let ident = variant_with_args.0;
            let args = variant_with_args.1;

            if !args.is_empty() {
                let args = args
                    .iter()
                    .map(|arg| format!("processed_args.pop()?.extract_{}()?", arg.to_lowercase()));
                let args = args.fold("".to_owned(), |acc, x| acc + &x);

                token_stream = format!(
                    "{token_stream}\n\t{type_enum}::{ident} => {action_enum}::{ident}({args}),"
                );
            } else {
                token_stream =
                    format!("{token_stream}\n\t{type_enum}::{ident} => {action_enum}::{ident},");
            }
        }

        token_stream = format!("{token_stream}\n}};\nSome(t)\n}}\n}}");
        token_stream.parse().unwrap()
    } else {
        TokenStream::new()
    }
}

#[inline(always)]
fn proces_group(group: TokenStream) -> (String, Vec<(String, Vec<String>)>) {
    let mut token_stream = String::new();

    let mut variants_with_args = Vec::new();

    for item in group {
        match item {
            proc_macro::TokenTree::Ident(ident) => {
                let ident = ident.to_string();
                token_stream = format!("{token_stream} {}", ident);
                variants_with_args.push((ident, Vec::new()));
            }
            proc_macro::TokenTree::Punct(punct) => {
                token_stream = format!("{token_stream} {}", punct.to_string());
            }
            proc_macro::TokenTree::Group(group) => {
                if group.delimiter() == proc_macro::Delimiter::Parenthesis {
                    if let Some(variant_with_args) = variants_with_args.last_mut() {
                        variant_with_args
                            .1
                            .append(&mut extract_types(group.stream()));
                    }
                }
            }
            _ => (),
        }
    }

    (token_stream, variants_with_args)
}

#[inline(always)]
fn extract_types(group: TokenStream) -> Vec<String> {
    let mut types = Vec::new();

    for item in group {
        match item {
            proc_macro::TokenTree::Ident(ident) => {
                types.push(ident.to_string());
            }
            _ => (),
        }
    }

    types
}
