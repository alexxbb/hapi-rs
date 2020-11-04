use heck::SnakeCase;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::io::Write;
// use once_cell::sync::Lazy;
use syn;
use syn::spanned::Spanned;
use syn::{
    parse::{Parse, ParseStream},
    Field, Ident, Item, ItemStruct, LitInt, Type,
};

pub struct ReturnType {
    pub fld: Field,
    pub ret_type: TokenStream,
}

impl ReturnType {
    fn new(fld: Field, ret_type: TokenStream) -> Self {
        ReturnType { fld, ret_type }
    }
    fn is_bool(&self) -> bool {
        &self.ret_type.to_string() == "bool"
    }
}

pub fn return_type(fld: &syn::Field) -> ReturnType {
    let fld_c = fld.clone();
    match &fld.ty {
        Type::Array(val) => ReturnType::new(fld_c, quote! {#val}),
        Type::Path(path) => {
            match path.path.get_ident() {
                // Simple return type
                Some(idn) => {
                    let name = idn.to_string();
                    match name.as_ref() {
                        "HAPI_Bool" => ReturnType::new(fld_c, quote! {bool}),
                        _ => ReturnType::new(fld_c, quote! {#idn}),
                    }
                }
                // Path
                None => ReturnType::new(fld_c, quote! {#path}),
            }
        }
        e => ReturnType::new(fld_c, quote! {#e}),
    }
}

static GEN_BUILDER: &[&str] = &["HAPI_PartType"];

pub fn renamed_struct(old: &Ident) -> Ident {
    let n = old.to_string();
    Ident::new(n.strip_prefix("HAPI_").unwrap(), Span::call_site())
}

pub enum MethodType {
    Getter,
    Setter,
}

pub struct StructIter<'a> {
    inner: &'a Vec<Item>,
    count: usize,
}

impl<'a> Iterator for StructIter<'a> {
    type Item = &'a ItemStruct;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.get(self.count) {
                Some(t) => {
                    self.count += 1;
                    match t {
                        syn::Item::Struct(s) => return Some(s),
                        _ => continue,
                    }
                }
                None => return None,
            }
        }
    }
}

pub fn iter_structs(items: &Vec<Item>) -> StructIter {
    StructIter {
        inner: items,
        count: 0,
    }
}

pub fn method_name(fld: &Ident, tp: MethodType) -> Ident {
    let field_name = fld.to_string();
    let mut new = field_name.to_snake_case();
    if new == "type" {
        new.push('_');
    }
    match tp {
        MethodType::Getter => Ident::new(&new, Span::call_site()),
        MethodType::Setter => Ident::new(&new, Span::call_site()),
    }
}

pub fn getter(name: &Ident, fld: &Ident, ret: &ReturnType) -> TokenStream {
    let idx = syn::Index::from(0);
    let ret_token = &ret.ret_type;
    if ret.is_bool() {
        quote! [
        pub fn #name(&self) -> #ret_token {
            self.#idx.#fld != 0
        }
        ]
    } else {
        quote! [
        pub fn #name(&self) -> #ret_token {
            self.#idx.#fld
        }
        ]
    }
}

pub fn wrapper_struct(ffi_struct: &ItemStruct) -> TokenStream {
    let orig_name = &ffi_struct.ident;
    let new_name = renamed_struct(&orig_name);
    quote! {
        #[derive(Debug)]
        pub struct #new_name(ffi::#orig_name);
    }
}

pub fn impl_struct(name: &Ident, methods: &Vec<TokenStream>) -> TokenStream {
    quote! {
        impl #name {
           #(#methods)*

            // pub fn ffi_type(&self) -> &ffi::HAPI_#Ident {
            //     self.0
            // }
        }
    }
}

/*

   for item in tree.items.iter() {
       match item {
           syn::Item::Struct(s) => {
               let struct_name = &s.ident;
               let wrap_name =
                   syn::Ident::new(&s.ident.to_string().replace("HAPI_", ""), Span::call_site());
               let ffi_name = quote! {ffi::#struct_name};
               let mut methods = vec![];
               for fld in s.fields.iter() {
                   let return_ty = {
                       // LitInt::new("10", Span::call_site())
                       let i = &fld.ty;
                       dbg!(&i);
                       Type::Verbatim(quote! {#i})
                           // Figure out struct fields type and generate their tokens
                           // match f.ty {
                           //     Type::Path(p) | Type::Verbatim(t) => {
                           //         // struct field type
                           //     }
                           // }
                   };
                   // let field = &fld.ident.unwrap();

                   let fld_token = &fld.ident.as_ref().unwrap();
                   let field_name = &fld.ident.as_ref().unwrap().to_string().to_lowercase();
                   let func_name = syn::Ident::new(&field_name, Span::call_site());

                   let func = quote! {
                       pub fn #func_name(&self) -> #return_ty {
                           self.inner.#fld_token
                       }
                   };
                   methods.push(func)
               }

               // let new = quote! { pub struct #wrap_name(#name); };
               let imp = quote! {
                   pub struct #wrap_name(#ffi_name);
                   impl #wrap_name {
                       #(#methods)*
                   }
               };
               new_tree.push(imp);
               // f.write_all(new.to_string().as_bytes());
               // let name = syn::Ident::new(&format!("ffi::{}", s.ident.to_string()), Span::call_site());
               // let name = syn::Expr::new(&format!("ffi::{}", s.ident.to_string()), Span::call_site());
               // s.ident = syn::Ident::new("Foo", s.span());
               // for f in s.fields.iter_mut() {
               //     f.ident.replace(syn::Ident::new("bar", f.span()));
               // }
               // let t = quote! {
               //     #s
               // };
               // let t = s.into_token_stream();
               // f.write_all(t.to_string().as_bytes());
           }
           _ => {}
       }
   }
   for i in &new_tree {
       f.write_all(i.to_string().as_bytes());
   }

*/
