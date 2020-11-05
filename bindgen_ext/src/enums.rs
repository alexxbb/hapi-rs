use crate::config::CodeGenInfo;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::Item;

fn test(enm: &syn::ItemEnum) -> TokenStream {
    let old_e = &enm.ident;
    let new_e = Ident::new("Test", Span::call_site());
    let old_f = Ident::new("Old", Span::call_site());
    let new_f = Ident::new("New", Span::call_site());
    let mut orig_enum_variants: Vec<_> = enm.variants.iter().map(|v| &v.ident).collect();
    let mut new_enum_variants: Vec<_> = orig_enum_variants
        .iter()
        .map(|i| Ident::new("Dummy", Span::call_site()))
        .collect();
    let mut match_arms: Vec<_> = orig_enum_variants
        .iter()
        .zip(new_enum_variants.iter())
        .map(|(orig_var, new_var)| {
            quote! {
                #old_e::#orig_var => #new_e::#new_var
            }
        })
        .collect();
    quote! [
        impl From<ffi::#old_e> for #new_e {
            fn from(e: ffi::#old_e) -> Self {
                match e {
                    #(#match_arms),*

                }
            }
        }
    ]
}

fn new_type() -> TokenStream {
    quote![
        pub enum Foo {}
    ]
}

pub fn generate_enums(items: &Vec<Item>, cfg: &CodeGenInfo) -> Vec<TokenStream> {
    let mut tokens = vec![];
    for item in items {
        match item {
            Item::Enum(en) => {
                let r = test(en);
                tokens.push(r);
            }
            _ => continue,
        };
    }
    tokens
}
