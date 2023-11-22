extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_derive(ActionToType)]
pub fn derive_action_to_type_fn(items: TokenStream) -> TokenStream {
    let mut token_stream = "#[derive(Debug)]\n".to_owned();
    let mut action_enum = false;

    for item in items {
        match item {
            proc_macro::TokenTree::Ident(ident) => {
                let ident = ident.to_string();
                if ident.ends_with("Action") {
                    token_stream = format!("{token_stream} {}Type", ident.replace("Action", ""));
                    action_enum = true;
                } else {
                    token_stream = format!("{token_stream} {}", ident.to_string());
                }
            }
            proc_macro::TokenTree::Group(group) => {
                if group.delimiter() == proc_macro::Delimiter::Brace && action_enum {
                    let group = group.stream();
                    token_stream = format!("{token_stream} {{ {} }} ", proces_group(group));
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

    if action_enum {
        token_stream.parse().unwrap()
    } else {
        TokenStream::new()
    }
}

#[inline(always)]
fn proces_group(group: TokenStream) -> String {
    let mut token_stream = String::new();

    for item in group {
        match item {
            proc_macro::TokenTree::Ident(ident) => {
                token_stream = format!("{token_stream} {}", ident.to_string());
            }
            proc_macro::TokenTree::Punct(punct) => {
                token_stream = format!("{token_stream} {}", punct.to_string());
            }
            _ => (),
        }
    }

    token_stream
}
