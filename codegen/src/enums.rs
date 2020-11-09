use crate::config::{CodeGenInfo, EnumOptions};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use log::warn;
use syn::{Item, ItemEnum, Variant, Attribute};
use crate::helpers;

struct EnumInfo {
    ffi_ident: Ident,
    new_ident: Ident,
    options: EnumOptions,
    attribs: Vec<Attribute>,
    new_variants: Vec<Variant>,
}

impl EnumInfo {
    fn new(enm: &ItemEnum, cg: EnumOptions) -> EnumInfo {
        let ffi_ident = enm.ident.clone();
        let ffi_name = ffi_ident.to_string();
        let new_ident = Ident::new(cg.new_name(&ffi_name), Span::call_site());
        let new_variants: Vec<_> = enm.variants
            .iter()
            .map(|v| {
                let n = v.ident.to_string();
                let mut var_name = helpers::strip_long_name(&n, cg.mode);
                let mut var = v.clone();
                var.ident = Ident::new(var_name, Span::call_site());
                var
            })
            .collect();

            EnumInfo {
            ffi_ident,
            new_ident,
            options: cg,
            attribs: enm.attrs.clone(),
            new_variants,
        }
    }
}

fn gen_enum_impl(info: &EnumInfo) -> TokenStream {
    let ffi_i = &info.ffi_ident;
    let new_i = &info.new_ident;
    quote![
        impl From<ffi::#ffi_i> for #new_i {
            fn from(e: ffi::#ffi_i) -> Self {
                unsafe { std::mem::transmute(e) }
            }
        }

        impl From<#new_i> for ffi::#ffi_i {
            fn from(e: #new_i) -> Self {
                unsafe { std::mem::transmute(e) }
            }
        }
    ]
}

fn gen_enum_body(info: &EnumInfo) -> TokenStream {
    let new_ident = &info.new_ident;
    let new_variants = &info.new_variants;
    let attribs = &info.attribs;
    quote! {
        #(#attribs)*
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
