use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use std::fs::read_to_string;
use std::io::Write;
use syn;
use syn::{parse::{Parse, ParseStream}, Type, Ident, LitInt, ReturnType};
use syn::spanned::Spanned;

fn main() {
    // let bindings_rs = "target/debug/build/hapi-sys-139b52ef038ee66c/out/bindings.rs";
    let bindings_rs = "bindgen_ext/src/simple.rs";
    // let bindings_rs = "bindgen_ext/src/f.rs";
    assert!(std::path::Path::new(bindings_rs).exists());
    let source = read_to_string(bindings_rs).unwrap();
    let mut tree: syn::File = syn::parse_file(&source).expect("Could not parse source");
    let mut f = std::fs::File::create("/tmp/bla.rs").unwrap();

    let mut new_tree = vec![];

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
                        Type::Verbatim(quote! {i32})
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
}
