use crate::config::{CodeGenInfo, TypeOptions};
use crate::helpers::*;
use log::warn;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens, format_ident};
use syn::{Attribute, Item, ItemConst, ItemMod, ItemType, Variant, Type};

struct FlagsInfo {
    ffi_ident: Ident,
    new_ident: Ident,
    options: TypeOptions,
    attribs: Vec<Attribute>,
    flag_type: Box<Type>,
    new_consts: Vec<TokenStream>,
}

impl FlagsInfo {
    fn new(imd: &ItemMod, cg: TypeOptions) -> FlagsInfo {
        let ffi_ident = imd.ident.clone();
        let ffi_name = ffi_ident.to_string();
        let new_ident = Ident::new(cg.new_name(&ffi_name), Span::call_site());
        let (_, content) = imd.content.as_ref().expect("No content in bitflag mod");
        let mut new_consts = vec![];
        let mut flag_type = None;
        for item in content {
            match item {
                Item::Type(ty) => {
                    new_consts.push(ty.to_token_stream());
                    flag_type.replace(ty.ty.clone());
                },
                Item::Const(i) => {
                    let const_name = i.ident.to_string();
                    let name =
                        change_case(strip_long_name(&const_name, cg.mode), CaseMode::EnumVariant);
                    let new_ident = Ident::new(&name, Span::call_site());
                    let mut new_item = i.clone();
                    new_item.ident = new_ident;
                    new_consts.push(new_item.to_token_stream());
                }
                e => warn!("Unexpected type in bitflag mod: {:?}", e),
            }
        }

        FlagsInfo {
            ffi_ident,
            new_ident,
            options: cg,
            attribs: imd.attrs.clone(),
            flag_type:flag_type.expect("Didn't find flag type"),
            new_consts,
        }
    }
}

fn gen_new_mod(info: &FlagsInfo) -> TokenStream {
    let new_ident = &info.new_ident;
    let new_consts = &info.new_consts;
    let bits_ident = format_ident!("{}Bits", new_ident);
    let bits_type = &info.flag_type;
    let attribs = &info.attribs;
    quote! {
        #(#attribs)*
        pub mod #new_ident {
            #(#new_consts)*
        }
        pub type #bits_ident = #bits_type;
    }
}

pub fn generate_bitflags(items: &Vec<Item>, cfg: &CodeGenInfo) -> Vec<TokenStream> {
    let mut tokens = vec![];
    for item in items {
        if let Item::Mod(imd) = item {
            let name = imd.ident.to_string();
            match cfg.flags_opt(&name) {
                Some(opt) => {
                    let fi = FlagsInfo::new(imd, opt);
                    tokens.push(gen_new_mod(&fi));
                }
                None => warn!("No config for mod {}", name),
            }
        }
    }
    tokens
}
