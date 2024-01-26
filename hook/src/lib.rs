#![feature(log_syntax)]

extern crate proc_macro;
use proc_macro::*;
use syn::*;
use quote::quote;

extern crate alloc;

// TODO make this use a RW lock instead

// Use as #[hook(hook_name)]
// above a function.

#[proc_macro_attribute]
pub fn hook(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemFn);
    let ItemFn { attrs, vis, sig, block } = input;
    let stmts = &block.stmts;
    // We should be fairly sure that we are looking at a function
    // declaration at this point, and we have names for some of the
    // parts

    // Stuff for generating a seperate static container for hook types
    let original_inputs = sig.inputs.clone();
    let original_output = sig.output.clone();
    let hook_name = parse_macro_input!(attr as syn::Ident);

    // List of types for declaring the hook signature
    let mut hook_inputs: punctuated::Punctuated<
            alloc::boxed::Box<Type>,token::Comma> = punctuated::Punctuated::new();
    // list of names with `mut`, but no types
    let mut original_input_names: punctuated::Punctuated<
            alloc::boxed::Box<Pat>, token::Comma> = punctuated::Punctuated::new();
    // list of namees with out mutability qualifiers
    let mut original_names_no_mut: punctuated::Punctuated<
            alloc::boxed::Box<Pat>, token::Comma> = punctuated::Punctuated::new();

    // populate the above
    for arg in original_inputs.iter() {
        match arg.clone() {
            FnArg::Receiver(_self_ref) => {
                todo!("Hooks on methods with self");
                // hook_inputs.push(self_ref.ty);
                // original_input_names.push(Box::new(Pat::Verbatim(quote!(self))));
            },
            FnArg::Typed(other) => {
                hook_inputs.push(other.ty);
                original_input_names.push(other.pat.clone());
                original_names_no_mut.push(
                    match *other.pat {
                        Pat::Ident(i) => {
                            Box::new(
                                Pat::Ident(
                                    PatIdent {
                                        attrs: i.attrs,
                                        by_ref: i.by_ref,
                                        mutability: None,
                                        ident: i.ident,
                                        subpat: i.subpat,
                                    }
                                )
                            )
                        },
                        _ => {
                            panic!("Non-ident pattern in function args?");
                        }
                    }
                )
            },
        };
    }

    // If a hook consumes the call and returns early, what type should it return?
    let hook_output = match original_output {
        ReturnType::Default => {
            // This is mostly what we need this for
            quote!(())
        },
        ReturnType::Type(_, boxed_type) => {
            quote!(#boxed_type)
        }
    };

    // This is the new static we are inserting to hold the installed hook
    let hook_item = syn::ItemStatic {
        attrs: vec!(), // type: Vec<Attribute>
        vis: Visibility::Public( token::Pub::default()),
        static_token: token::Static::default(),
        mutability: StaticMutability::Mut(token::Mut::default()),
        // ^ We actually want this to be immutable, because we want it
        // locked. but for some reason when you mark it without mut,
        // then we get errors saying that our dyn FnMut type is not
        // Send TODO fix
        ident: hook_name.clone(),
        colon_token: token::Colon::default(),
        ty: alloc::boxed::Box::new(
                Type::Verbatim(quote!{
                    crate::lock::mutex::Mutex<alloc::vec::Vec<alloc::boxed::Box<dyn core::ops::FnMut
                        ((#hook_inputs)) -> crate::hook::HookReturn<(#hook_inputs), #hook_output>
                        >>>
                })),
        eq_token: token::Eq::default(),
        expr: alloc::boxed::Box::new(Expr::Verbatim(quote!{
            crate::lock::mutex::Mutex::new(vec!())
        })),                                          // initial contents, empty
        semi_token: token::Semi::default(),
    };

    // The token stream we are actually releasing to the compiler
    TokenStream::from(quote! {
        // log_syntax!(#hook_item);
        #hook_item
        // ^ This inserts the static
        #(#attrs)* #vis #sig {
            // ^ this is exactly the fn signature we were passed
            let mut __inputs = (#original_names_no_mut);
            // ^ collect the original arguments
            unsafe {
                let mut container = #hook_name.lock();
                // iterate over hooks, either replacing arguments or consuming and returning early
                for hook in container.iter_mut() {
                    match hook(__inputs) {
                        crate::hook::HookReturn::Consume(output) => {
                            return output
                        },
                        crate::hook::HookReturn::Compose(new_input) => {
                            __inputs = new_input
                        },
                    }
                }
            }
            // Insert another block here so we can safely shadow the
            // original arguments without bothering the original
            // function body
            {
                // this shadowing of the original arguments allows us
                // to invisibly change original non-mutable objects,
                // although mutability is not a solved problem just
                // yet
                let (#original_input_names) = __inputs;
                #(#stmts)*
                // ^ the original function body
            }
        }
    }
)}
