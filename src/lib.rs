// A lot of this code was copied/modified from
// https://github.com/dtolnay/syn/blob/master/examples/lazy-static/lazy-static/src/lib.rs
// and https://github.com/rust-lang-nursery/lazy-static.rs, both under the MIT License.

//! Eager + Lazy = Eazy.
//!
//! This crate contains a basic macro which imitates lazy-static, but also produces a function
//! which can eagerly load all static variables defined in that macro block.

extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TS2;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Attribute, Expr, Ident, Token, Type, Visibility};

struct EazyStatic {
    visibility: Visibility,
    name: Ident,
    ty: Type,
    init: Expr,
}

struct EazyStatics {
    statics: Vec<EazyStatic>,
}

impl Parse for EazyStatics {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut statics = Vec::default();
        while !input.is_empty() {
            Attribute::parse_outer(input)?;
            let visibility: Visibility = input.parse()?;
            input.parse::<Token![static]>()?;
            input.parse::<Token![ref]>()?;
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let ty: Type = input.parse()?;
            input.parse::<Token![=]>()?;
            let init: Expr = input.parse()?;
            input.parse::<Token![;]>()?;
            statics.push(EazyStatic {
                visibility,
                name,
                ty,
                init,
            })
        }
        Ok(Self { statics })
    }
}

#[proc_macro]
/// Lazily initialise one or more static variables at run-time, and supply a function to eagerly
/// initialise said values.
///
/// Wherever this macro is used, a public function `init_all` will be in scope, which when called
/// access each of the variables defined in the macro block. This will initialise every variable
/// that has not already been initialised.
/// ```
/// use eazy_static::eazy_static;
///
/// use std::sync::{atomic::{AtomicBool, Ordering}};
///
/// static X: AtomicBool = AtomicBool::new(true);
/// static Y: AtomicBool = AtomicBool::new(true);
///
/// assert!(X.load(Ordering::SeqCst));
/// assert!(Y.load(Ordering::SeqCst));
///
/// eazy_static!{
///     static ref XEDITED: &'static str = {
///         X.store(false, Ordering::SeqCst);
///         "X has been edited!"
///     };
///     static ref YEDITED: &'static str = {
///         Y.store(false, Ordering::SeqCst);
///         "Y has been edited!"
///     };
/// }
///
/// assert!(X.load(Ordering::SeqCst));
/// assert!(Y.load(Ordering::SeqCst));
///
/// println!("{}", *XEDITED);
/// assert_eq!(X.load(Ordering::SeqCst), false);
/// assert!(Y.load(Ordering::SeqCst));
///
/// init_all();
/// assert_eq!(Y.load(Ordering::SeqCst), false);
/// ```
pub fn eazy_static(input: TokenStream) -> TokenStream {
    let EazyStatics { statics } = parse_macro_input!(input as EazyStatics);

    let mut iter = statics.iter();

    let mut out: TokenStream = TokenStream::default();

    let mut deref_all: TS2 = TS2::default();

    while let Some(EazyStatic {
        visibility,
        name,
        ty,
        init,
    }) = iter.next()
    {
        if let Expr::Tuple(ref init) = init {
            if init.elems.is_empty() {
                init.span().unwrap();
                return TokenStream::new();
            }
        }

        let assert_sync = quote_spanned! {ty.span()=>
            struct _AssertSync where #ty: std::marker::Sync;
        };

        let assert_sized = quote_spanned! {ty.span()=>
            struct _AssertSized where #ty: std::marker::Sized;
        };

        let init_ptr = quote_spanned! {init.span()=>
            Box::into_raw(Box::new(#init))
        };

        let expanded = quote! {
            #[allow(missing_copy_implementations)]
            #[allow(non_camel_case_types)]
            #[allow(dead_code)]
            #visibility struct #name { __ : () }
            #[doc(hidden)]
            #visibility static #name: #name = #name { __ : () };

            impl std::ops::Deref for #name {
                type Target = #ty;

                fn deref(&self) -> &#ty {
                    #assert_sync
                    #assert_sized

                    static ONCE: std::sync::Once = std::sync::Once::new();
                    static mut VALUE: *mut #ty = 0 as *mut #ty;

                    unsafe {
                        ONCE.call_once(|| VALUE = #init_ptr);
                        &*VALUE
                    }
                }
            }
        };
        out.extend(TokenStream::from(expanded));
        deref_all.extend(quote! {
            let _ = std::ops::Deref::deref(&#name);
        })
    }
    let init_all = quote! {
        pub fn init_all() {
            #deref_all
        }
    };
    out.extend(TokenStream::from(init_all));
    out
}
