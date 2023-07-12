#![feature(log_syntax)]

extern crate proc_macro;
use proc_macro::*;
use syn::*;
use quote::quote;

extern crate alloc;

#[proc_macro_attribute]
pub fn hook(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemFn);
    let ItemFn { attrs, vis, sig, block } = input;
    let stmts = &block.stmts;

    // Stuff for generating a seperate static container for hook types
    let original_inputs = sig.inputs.clone();
    let original_output = sig.output.clone();
    let hook_name = parse_macro_input!(attr as syn::Ident);

    let mut hook_inputs: punctuated::Punctuated<alloc::boxed::Box<Type>, token::Comma> = punctuated::Punctuated::new();

    for arg in original_inputs.iter() {
        hook_inputs.push(match arg.clone() {
            FnArg::Receiver(self_ref) => {
                self_ref.ty
            },
            FnArg::Typed(other) => {
                other.ty
            },
        });
    }

    let hook_output = match original_output {
        ReturnType::Default => {
            quote!(bool)        // TODO this type is meaningless, but is here to keep the signatures similar
        },
        ReturnType::Type(_, boxed_type) => {
            quote!(#boxed_type)
        }
    };

    let hook_item = syn::ItemStatic {
        attrs: vec!(), // type: Vec<Attribute>, TODO add a comment or something
        vis: Visibility::Public( token::Pub::default()),
        static_token: token::Static::default(),
        mutability: StaticMutability::Mut(token::Mut::default()),   // We need this to be mut, but we'll put this in a lock
        ident: hook_name,
        colon_token: token::Colon::default(),
        ty: alloc::boxed::Box::new(
                Type::Verbatim(quote!{
                    crate::lock::mutex::Mutex<alloc::vec::Vec<alloc::boxed::Box<dyn core::ops::FnMut
                        (#hook_inputs) -> (#hook_output, bool)
                        >>>
                })
                ),      // type: Box<Type>, we want something
        // like Verbatim(TokenStream) to get a Lock around a containter
        eq_token: token::Eq::default(),
        expr: alloc::boxed::Box::new(Expr::Verbatim(quote!{
            crate::lock::mutex::Mutex::new(vec!())    // TODO depends on type
        })),
        semi_token: token::Semi::default(),
    };

    // The token stream we are actually releasing to the compiler
    TokenStream::from(quote! {
        use alloc::vec;
        // log_syntax!(#hook_item);
        #hook_item
        #(#attrs)* #vis #sig {
            let _ = 1+1;
            #(#stmts)*
        }
    }
)}
