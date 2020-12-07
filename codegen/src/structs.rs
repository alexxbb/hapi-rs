use crate::config::{CodeGenConfig, StructOptions};
use crate::helpers::*;
use once_cell::sync::Lazy;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::collections::HashMap;
use syn::export::ToTokens;
use syn::{Attribute, Fields, Item, ItemStruct, Type};

static TYPEMAP: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("HAPI_StringHandle", "Result<String>");
    map.insert("HAPI_Bool", "bool");
    map.insert("HAPI_Bool", "bool");
    map
});

/// Rules
/// check return type:
///   if it's an enum or a struct: Lookup its new rusty name in the config
///   if it's a HAPI_StringHandle, remove SH from the field name and generate the appropriate func call

#[derive(Debug)]
pub struct StructInfo {
    ffi_ident: Ident,
    new_ident: Ident,
    derives: Vec<String>,
    pub_fields: Vec<Ident>,
    string_fields: Vec<Ident>,
    simple_getters: Vec<(TokenStream, TokenStream)>,
}

impl StructInfo {
    fn new(st: &ItemStruct, opt: StructOptions, cfg: &CodeGenConfig) -> Self {
        let ffi_ident = st.ident.clone();
        let ffi_name = ffi_ident.to_string();
        let new_ident = Ident::new(opt.new_name(&ffi_name), Span::call_site());
        let mut simple_getters = vec![];
        let mut string_fields = vec![];
        let mut pub_fields = vec![];
        if let Fields::Named(fields) = &st.fields {
            for field in &fields.named {
                let ident = field.ident.as_ref().expect("Encountered tuple struct");
                let mut orig_name = ident.to_string();
                let mut fld_name = change_case(&orig_name, CaseMode::EnumVariant);
                let typ = field.ty.to_token_stream().to_string();
                let new_typ = cfg.new_name(&typ);
                if new_typ == "HAPI_StringHandle" {
                    let fld_name = fld_name.strip_suffix("SH").unwrap_or(&fld_name);
                    string_fields.push(Ident::new(fld_name, Span::call_site()))
                } else {
                    pub_fields.push(Ident::new(&fld_name, Span::call_site()))
                }
            }
        }

        StructInfo {
            ffi_ident,
            new_ident,
            derives: opt.derive.clone(),
            pub_fields,
            string_fields,
            simple_getters,
        }
    }
}

pub fn gen_struct(info: StructInfo) -> TokenStream {
    let new_name = &info.new_ident;
    let ffi_name = &info.ffi_ident;
    let pub_fields = &info.pub_fields;
    let def = quote! [
        pub struct #new_name {
            pub(crate) inner: #ffi_name,
            #(pub #pub_fields: i32),*

        }
    ];
    def
}

pub fn generate_structs(items: &Vec<Item>, cfg: &CodeGenConfig) -> Vec<TokenStream> {
    let mut tokens = vec![];
    for i in items {
        if let Item::Struct(s) = i {
            let name = s.ident.to_string();
            if let Some(opts) = cfg.struct_opt(&name) {
                dbg!(&opts);
                let info = StructInfo::new(s, opts, cfg);
                let s = gen_struct(info);
                tokens.push(s);
            }
        }
    }
    tokens
}
