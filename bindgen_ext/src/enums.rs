use crate::config::{CodeGenInfo, EnumOptions};
use crate::{error, warn};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::de::Unexpected::Enum;
use syn::{Item, ItemEnum};

struct EnumInfo {
    ffi_ident: Ident,
    new_ident: Ident,
    ffi_name: String,
    new_name: String,
    options: EnumOptions,
    ffi_variants: Vec<Ident>,
    new_variants: Vec<Ident>,
}

impl EnumInfo {
    fn new(enm: &ItemEnum, cg: EnumOptions) -> EnumInfo {
        let ffi_ident = enm.ident.clone();
        let ffi_name = ffi_ident.to_string();
        let new_name = match cg.rename.as_ref() {
            "auto" => ffi_name.strip_prefix("HAPI_").unwrap().to_owned(),
            n => n.to_owned(),
        };
        let new_ident = Ident::new(&new_name, Span::call_site());
        let ffi_variants: Vec<_> = enm.variants.iter().map(|v| v.ident.clone()).collect();
        let new_variants: Vec<_> = ffi_variants
            .iter()
            .map(|i| {
                let n = i.to_string();
                let mut var = strip_variant(&n, cg.variant);
                let is_digit = var.chars().take(1).map(|c| c.is_digit(10)).any(|v| v);
                if is_digit {
                    var = strip_variant(&n, cg.variant + 1)
                }
                Ident::new(var, Span::call_site())
            })
            .collect();

        EnumInfo {
            ffi_ident,
            new_ident,
            ffi_name,
            new_name,
            options: cg,
            ffi_variants,
            new_variants,
        }
    }
}

fn gen_enum_impl(info: &EnumInfo) -> TokenStream {
    let ffi_i = &info.ffi_ident;
    let new_i = &info.new_ident;
    let mut match_arms_to_new = vec![];
    let mut match_arms_from_new = vec![];
    for (ffi_var, new_var) in info.ffi_variants.iter().zip(info.new_variants.iter()) {
        match_arms_to_new.push(quote! {
            #ffi_i::#ffi_var => #new_i::#new_var
        });
        match_arms_from_new.push(quote! {
            #new_i::#new_var => #ffi_i::#ffi_var
        });
    }
    quote![
        impl From<ffi::#ffi_i> for #new_i {
            fn from(e: ffi::#ffi_i) -> Self {
                match e {
                    #(#match_arms_to_new),*
                }
            }
        }

        impl From<#new_i> for ffi::#ffi_i {
            fn from(e: #new_i) -> Self {
                match e {
                    #(#match_arms_from_new),*
                }
            }
        }
    ]
}

fn strip_variant(name: &str, idx: i32) -> &str {
    assert_ne!(idx, 0);
    let mut iter = name.match_indices('_');
    let p = if idx > 0 {
        iter.nth_back((idx - 1) as usize)
    } else {
        iter.nth((idx + 1).abs() as usize)
    };
    match p {
        Some((i, _)) => &name[i + 1..name.len()],
        None => {
            warn!("field {} length < {}", name, idx);
            name
        }
    }
}

fn gen_enum_body(info: &EnumInfo) -> TokenStream {
    let new_ident = &info.new_ident;
    let new_variants = &info.new_variants;
    quote! {
        pub enum #new_ident {
            #(#new_variants),*
        }
    }
}

pub fn generate_enums(items: &Vec<Item>, cfg: &CodeGenInfo) -> Vec<TokenStream> {
    let mut tokens = vec![];
    for item in items {
        match item {
            Item::Enum(en) => {
                let name = en.ident.to_string();
                match cfg.enum_opt(&name) {
                    Some(opt) => {
                        let e = EnumInfo::new(en, opt);
                        tokens.push(gen_enum_body(&e));
                        tokens.push(gen_enum_impl(&e));
                    }
                    None => warn!("No config for enum {}", name),
                }
            }
            _ => continue,
        };
    }
    tokens
}
