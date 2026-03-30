//! Proc macros for dash-sdk.
//!
//! Provides `#[box_async]` — a drop-in replacement for `#[async_trait]` that avoids
//! the HRTB Send inference bug ([rust-lang/rust#96865]).
//!
//! # Why not `#[async_trait]`?
//!
//! `async_trait` introduces a unified `'async_trait` lifetime with complex `where`
//! clauses that trigger a Rust compiler bug when many generic trait implementations
//! exist (e.g. 28+ `impl Fetch` types). The compiler fails to prove the resulting
//! future is `Send`.
//!
//! `#[box_async]` performs the same transformation — boxing the return into
//! `Pin<Box<dyn Future + Send>>` — but uses explicit per-reference lifetimes,
//! avoiding the problematic HRTB inference path.
//!
//! # Usage
//!
//! ```ignore
//! use dash_sdk_macros::box_async;
//!
//! #[box_async]
//! pub trait MyTrait {
//!     async fn do_work(&self, sdk: &Sdk) -> Result<(), Error> {
//!         sdk.call().await
//!     }
//! }
//!
//! #[box_async]
//! impl MyTrait for MyStruct {
//!     async fn do_work(&self, sdk: &Sdk) -> Result<(), Error> {
//!         sdk.other_call().await
//!     }
//! }
//! ```
//!
//! [rust-lang/rust#96865]: https://github.com/rust-lang/rust/issues/96865

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, FnArg, GenericParam, ImplItem, Lifetime, LifetimeParam,
    ReturnType, Signature, TraitItem, Type,
};

/// Transform `async fn` methods to return boxed futures.
///
/// Apply to a trait definition or an `impl` block. Each `async fn` is converted to
/// a regular `fn` returning `Pin<Box<dyn Future<Output = T> + Send + 'a>>`, where
/// `'a` captures all reference parameter lifetimes.
///
/// The method body is wrapped in `Box::pin(async move { ... })`.
#[proc_macro_attribute]
pub fn box_async(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_clone = item.clone();

    // Try parsing as trait first, then as impl
    if let Ok(mut trait_item) = syn::parse::<syn::ItemTrait>(item_clone) {
        // Add Send as a supertrait bound (async_trait does this implicitly)
        let has_send = trait_item.supertraits.iter().any(|bound| {
            if let syn::TypeParamBound::Trait(t) = bound {
                t.path.is_ident("Send")
            } else {
                false
            }
        });
        if !has_send {
            trait_item
                .supertraits
                .push(parse_quote!(Send));
        }

        for item in &mut trait_item.items {
            if let TraitItem::Fn(method) = item {
                if method.sig.asyncness.is_some() {
                    transform_signature(&mut method.sig);
                    if let Some(body) = &method.default {
                        let stmts = &body.stmts;
                        method.default = Some(parse_quote!({
                            Box::pin(async move { #(#stmts)* })
                        }));
                    }
                }
            }
        }
        return TokenStream::from(quote! { #trait_item });
    }

    let mut impl_item = parse_macro_input!(item as syn::ItemImpl);
    for item in &mut impl_item.items {
        if let ImplItem::Fn(method) = item {
            if method.sig.asyncness.is_some() {
                transform_signature(&mut method.sig);
                let stmts = &method.block.stmts;
                method.block = parse_quote!({
                    Box::pin(async move { #(#stmts)* })
                });
            }
        }
    }
    TokenStream::from(quote! { #impl_item })
}

/// Transform an async fn signature to return a boxed future.
///
/// - Removes `async`
/// - Adds lifetime `'__boxfut` to all unnamed reference params
/// - Adds `'__boxfut` bound to all generic type params
/// - Changes return type to `Pin<Box<dyn Future<Output = T> + Send + '__boxfut>>`
fn transform_signature(sig: &mut Signature) {
    sig.asyncness = None;

    let output_ty = match &sig.output {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    };

    // Check if any parameters are references (need lifetime)
    let has_references = sig.inputs.iter().any(|arg| match arg {
        FnArg::Receiver(r) => r.reference.is_some(),
        FnArg::Typed(t) => matches!(*t.ty, Type::Reference(_)),
    });

    if !has_references {
        // No references — return 'static future
        sig.output = parse_quote! {
            -> ::std::pin::Pin<Box<
                dyn ::std::future::Future<Output = #output_ty> + Send
            >>
        };
        return;
    }

    let lifetime = Lifetime::new("'__boxfut", Span::call_site());

    // Add lifetime parameter at position 0
    sig.generics.params.insert(
        0,
        GenericParam::Lifetime(LifetimeParam::new(lifetime.clone())),
    );

    // Apply lifetime to all reference params (including nested, e.g. Option<&T>).
    // All references captured by the async block must share the same lifetime.
    for arg in &mut sig.inputs {
        match arg {
            FnArg::Receiver(r) => {
                if let Some((_, ref mut lt)) = r.reference {
                    if lt.is_none() {
                        *lt = Some(lifetime.clone());
                    }
                }
            }
            FnArg::Typed(t) => {
                apply_lifetime_to_refs(&mut t.ty, &lifetime);
            }
        }
    }

    // Add lifetime bound to all generic type params
    for param in &mut sig.generics.params {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(parse_quote! { #lifetime });
        }
    }

    // Transform return type
    sig.output = parse_quote! {
        -> ::std::pin::Pin<Box<
            dyn ::std::future::Future<Output = #output_ty> + Send + #lifetime
        >>
    };
}

/// Recursively apply a lifetime to all anonymous references in a type,
/// including references nested inside generic types like `Option<&T>`.
fn apply_lifetime_to_refs(ty: &mut Type, lifetime: &Lifetime) {
    match ty {
        Type::Reference(type_ref) => {
            if type_ref.lifetime.is_none() {
                type_ref.lifetime = Some(lifetime.clone());
            }
            apply_lifetime_to_refs(&mut type_ref.elem, lifetime);
        }
        Type::Path(type_path) => {
            // Recurse into generic arguments: Option<&T>, Vec<&T>, BTreeMap<K, &V>, etc.
            for segment in &mut type_path.path.segments {
                if let syn::PathArguments::AngleBracketed(args) = &mut segment.arguments {
                    for arg in &mut args.args {
                        if let syn::GenericArgument::Type(inner_ty) = arg {
                            apply_lifetime_to_refs(inner_ty, lifetime);
                        }
                    }
                }
            }
        }
        Type::Tuple(type_tuple) => {
            for elem in &mut type_tuple.elems {
                apply_lifetime_to_refs(elem, lifetime);
            }
        }
        _ => {}
    }
}
