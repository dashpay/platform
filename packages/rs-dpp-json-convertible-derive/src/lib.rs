extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Item, Type};

const DEFAULT_BASE_PATH: &str = "crate::serialization";

/// Attribute macro that adds `#[serde(with = "...")]` to `u64`/`i64` fields
/// and implements `JsonSafeFields` for the type (when used inside the `dpp` crate).
///
/// Works on both **structs** and **enums** (with named fields in variants).
///
/// Matches literal `u64`, `i64`, `Option<u64>`, `Option<i64>`, and known type aliases
/// (e.g. `Credits`, `TokenAmount`, `TimestampMillis`, `BlockHeight`).
/// The macro skips fields that already have `#[serde(with)]`.
///
/// When serde derives are behind `cfg_attr(feature = "...")`, the `cfg_attr` is
/// evaluated by the compiler BEFORE this macro runs. If the feature is off, serde
/// derives aren't visible and `#[serde(with)]` is NOT generated — which is correct
/// because `serde(with)` requires an active serde derive.
///
/// # Inside `dpp` crate (default)
/// ```ignore
/// #[json_safe_fields]
/// pub struct MyStructV0 {
///     pub supply: u64,          // → auto-annotated with crate::serialization::json_safe_u64
///     pub name: String,         // → untouched
/// }
/// ```
///
/// # Outside `dpp` crate
/// Use `crate = "..."` to specify the path to the dpp serialization module.
/// When `crate` is specified, the `JsonSafeFields` impl is NOT generated.
/// ```ignore
/// #[json_safe_fields(crate = "dash_sdk::dpp")]
/// pub struct MyWasmStruct {
///     pub balance: u64,         // → annotated with dash_sdk::dpp::serialization::json_safe_u64
/// }
/// ```
#[proc_macro_attribute]
pub fn json_safe_fields(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse optional `crate = "..."` attribute
    let (base_path, is_external) = if attr.is_empty() {
        (DEFAULT_BASE_PATH.to_string(), false)
    } else {
        match syn::parse::<syn::MetaNameValue>(attr) {
            Ok(meta) if meta.path.is_ident("crate") => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = &meta.value
                {
                    (format!("{}::serialization", lit_str.value()), true)
                } else {
                    return syn::Error::new_spanned(meta.value, "expected string literal")
                        .to_compile_error()
                        .into();
                }
            }
            Ok(meta) => {
                return syn::Error::new_spanned(
                    meta.path,
                    "expected `crate = \"...\"`",
                )
                .to_compile_error()
                .into();
            }
            Err(e) => return e.to_compile_error().into(),
        }
    };

    let input = syn::parse::<Item>(item);
    let item = match input {
        Ok(item) => item,
        Err(e) => return e.to_compile_error().into(),
    };

    match item {
        Item::Struct(mut s) => {
            // Check if serde derives are present.
            // NOTE: cfg_attr is evaluated by the compiler BEFORE attribute macros run.
            // If serde derives are behind cfg_attr with a disabled feature, we won't
            // see them — which is correct (no serde(with) without an active derive).
            let mut nested_types = Vec::new();
            if has_serde_derive(&s.attrs) {
                if let Fields::Named(ref mut fields) = s.fields {
                    nested_types = annotate_fields(fields, &base_path);
                }
            }

            // Only generate JsonSafeFields impl when used inside dpp (no crate override)
            if is_external {
                quote! { #s }.into()
            } else {
                let name = &s.ident;
                let generics = &s.generics;
                let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

                let expanded = if nested_types.is_empty() {
                    quote! {
                        #s

                        impl #impl_generics crate::serialization::JsonSafeFields for #name #ty_generics #where_clause {}
                    }
                } else {
                    let assertions = generate_nested_assertions(&nested_types);
                    quote! {
                        #s

                        impl #impl_generics crate::serialization::JsonSafeFields for #name #ty_generics #where_clause {}

                        #[cfg(feature = "json-conversion")]
                        #assertions
                    }
                };
                expanded.into()
            }
        }
        Item::Enum(mut e) => {
            let mut nested_types = Vec::new();
            if has_serde_derive(&e.attrs) {
                for variant in e.variants.iter_mut() {
                    if let Fields::Named(ref mut fields) = variant.fields {
                        nested_types.extend(annotate_fields(fields, &base_path));
                    }
                }
            }

            if is_external {
                quote! { #e }.into()
            } else {
                let name = &e.ident;
                let generics = &e.generics;
                let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

                let expanded = if nested_types.is_empty() {
                    quote! {
                        #e

                        impl #impl_generics crate::serialization::JsonSafeFields for #name #ty_generics #where_clause {}
                    }
                } else {
                    let assertions = generate_nested_assertions(&nested_types);
                    quote! {
                        #e

                        impl #impl_generics crate::serialization::JsonSafeFields for #name #ty_generics #where_clause {}

                        #[cfg(feature = "json-conversion")]
                        #assertions
                    }
                };
                expanded.into()
            }
        }
        other => {
            let err = syn::Error::new_spanned(
                quote! { #other },
                "#[json_safe_fields] can only be applied to structs or enums",
            );
            err.to_compile_error().into()
        }
    }
}

/// Generate compile-time assertions that all nested field types implement `JsonSafeFields`.
///
/// This ensures that if a struct contains a field like `config: DistributionFunction`,
/// the compiler will verify that `DistributionFunction` also has `#[json_safe_fields]`.
fn generate_nested_assertions(types: &[Type]) -> proc_macro2::TokenStream {
    if types.is_empty() {
        return quote! {};
    }

    // Deduplicate types by their string representation
    let mut seen = std::collections::HashSet::new();
    let mut assertions = Vec::new();

    for (i, ty) in types.iter().enumerate() {
        let type_str = quote! { #ty }.to_string();
        if seen.insert(type_str) {
            let fn_name = Ident::new(
                &format!("_assert_json_safe_fields_{}", i),
                Span::call_site(),
            );
            assertions.push(quote! {
                fn #fn_name<T: crate::serialization::JsonSafeFields>() {}
                let _ = #fn_name::<#ty>;
            });
        }
    }

    quote! {
        const _: () = {
            #(#assertions)*
        };
    }
}

/// Add `#[serde(with = "...")]` to u64/i64 fields in a named fields block.
/// Skips fields that already have `#[serde(with)]`.
///
/// Returns the types of fields that were NOT annotated (i.e., fields that are not
/// u64/i64 and don't already have `serde(with)`). These types should be checked
/// for `JsonSafeFields` via compile-time assertions.
fn annotate_fields(fields: &mut syn::FieldsNamed, base_path: &str) -> Vec<Type> {
    let mut unannotated_types = Vec::new();
    for field in fields.named.iter_mut() {
        if let Some(suffix) = serde_with_suffix_for_type(&field.ty) {
            let with_path = format!("{}::{}", base_path, suffix);
            let already_has_serde_with = field.attrs.iter().any(|attr| {
                if attr.path().is_ident("serde") {
                    let mut found = false;
                    let _ = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("with") {
                            found = true;
                        }
                        Ok(())
                    });
                    found
                } else {
                    false
                }
            });
            if !already_has_serde_with {
                // For Option types, also add `default` so missing fields
                // deserialize as None. Without this, serde(with) overrides serde's
                // built-in Option handling and missing fields cause errors.
                if suffix.contains("option") {
                    let already_has_serde_default = has_serde_default(&field.attrs);
                    if already_has_serde_default {
                        // Already has default, just add with
                        field.attrs.push(syn::parse_quote! {
                            #[serde(with = #with_path)]
                        });
                    } else {
                        // Combine default + with in a single attribute to avoid duplicates
                        field.attrs.push(syn::parse_quote! {
                            #[serde(default, with = #with_path)]
                        });
                    }
                } else {
                    field.attrs.push(syn::parse_quote! {
                        #[serde(with = #with_path)]
                    });
                }
            }
        } else {
            // This field is not u64/i64 — its type must implement JsonSafeFields
            // to guarantee it doesn't contain unprotected large integers.
            // Skip fields that have explicit serde handling:
            // - skip/skip_serializing/skip_deserializing: not serialized
            // - flatten: special serde handling
            // - with: developer explicitly controls serialization
            let has_serde_override = field.attrs.iter().any(|attr| {
                if attr.path().is_ident("serde") {
                    let mut found = false;
                    let _ = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("skip")
                            || meta.path.is_ident("skip_serializing")
                            || meta.path.is_ident("skip_deserializing")
                            || meta.path.is_ident("flatten")
                            || meta.path.is_ident("with")
                        {
                            found = true;
                        }
                        Ok(())
                    });
                    found
                } else {
                    false
                }
            });
            // Also check for serde(with) inside cfg_attr, e.g.
            // #[cfg_attr(feature = "json-conversion", serde(with = "..."))]
            let has_cfg_attr_serde_with = field.attrs.iter().any(|attr| {
                if attr.path().is_ident("cfg_attr") {
                    let tokens = attr.meta.to_token_stream().to_string();
                    tokens.contains("serde") && tokens.contains("with")
                } else {
                    false
                }
            });
            if !has_serde_override && !has_cfg_attr_serde_with {
                unannotated_types.push(field.ty.clone());
            }
        }
    }
    unannotated_types
}

/// Determine if a type needs a serde `with` annotation and return the module name suffix.
///
/// Matches literal `u64`, `i64`, `Option<u64>`, `Option<i64>`, and known type aliases
/// that resolve to u64/i64 (e.g. `Credits`, `TokenAmount`, `TimestampMillis`).
///
/// When adding a new `type X = u64` alias in rs-dpp, add it to the appropriate list below.
fn serde_with_suffix_for_type(ty: &Type) -> Option<&'static str> {
    match ty {
        Type::Path(type_path) => {
            let segments = &type_path.path.segments;
            // Get the last segment — handles both `u64` and `std::u64` / `crate::prelude::Credits`
            let last = segments.last()?;
            let ident = &last.ident;

            if is_u64_type(ident) {
                return Some("json_safe_u64");
            }
            if is_i64_type(ident) {
                return Some("json_safe_i64");
            }
            // Check for Option<u64/i64/alias> — handles both `Option<T>` and `std::option::Option<T>`
            if ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                    if args.args.len() == 1 {
                        if let syn::GenericArgument::Type(Type::Path(inner)) = &args.args[0] {
                            if let Some(inner_last) = inner.path.segments.last() {
                                if is_u64_type(&inner_last.ident) {
                                    return Some("json_safe_option_u64");
                                }
                                if is_i64_type(&inner_last.ident) {
                                    return Some("json_safe_option_i64");
                                }
                            }
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Known type aliases that resolve to u64.
/// Keep in sync with type aliases defined in rs-dpp.
const U64_ALIASES: &[&str] = &[
    "BlockHeight",
    "BlockHeightInterval",
    "Credits",
    "Duffs",
    "FeeMultiplier",
    "IdentityNonce",
    "ProtocolVersionVoteCount",
    "RemainingCredits",
    "Revision",
    "TimestampMillis",
    "TimestampMillisInterval",
    "TokenAmount",
    "TokenDistributionWeight",
    "WithdrawalTransactionIndex",
];

/// Known type aliases that resolve to i64.
const I64_ALIASES: &[&str] = &["SignedCredits", "SignedTokenAmount"];

/// Check if the struct has serde derives (Serialize or Deserialize) in its attributes.
///
/// NOTE: `cfg_attr` is evaluated by the compiler BEFORE attribute macros run.
/// If serde derives are behind `cfg_attr(feature = "...", derive(Serialize))` and the
/// feature is disabled, the `cfg_attr` is already stripped — so we won't see it, which
/// is the correct behavior (we shouldn't add `#[serde(with)]` when serde isn't derived).
fn has_serde_derive(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("derive") {
            if let Ok(list) = attr.meta.require_list() {
                // Parse the derive list to find exact Serialize/Deserialize idents
                // Must handle both `Serialize` and `serde::Serialize` forms
                let tokens = list.tokens.clone();
                for tt in tokens.into_iter() {
                    if let proc_macro2::TokenTree::Ident(ident) = &tt {
                        let name = ident.to_string();
                        // Exact ident match — "PlatformSerialize" is a single ident,
                        // not "Platform" + "Serialize", so this won't false-match.
                        if name == "Serialize" || name == "Deserialize" {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Check if a field already has `#[serde(default)]` or `#[serde(default = "...")]`
/// in any of its attributes, including inside `#[cfg_attr(..., serde(..., default))]`.
///
/// Note: cfg_attr on fields inside a struct body is NOT evaluated before
/// the outer attribute macro processes the struct, so we must scan tokens manually.
fn has_serde_default(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        let full_tokens = quote! { #attr }.to_string();
        // Check if this attribute contains "serde" and a "default" token
        if full_tokens.contains("serde") && full_tokens.contains("default") {
            if let Ok(list) = attr.meta.require_list() {
                let token_str = list.tokens.to_string();
                // Match "default" as standalone or "default = ..." (custom fn)
                if has_default_in_token_str(&token_str) {
                    return true;
                }
                // Also check inside nested groups (cfg_attr case)
                // e.g., cfg_attr(feature = "...", serde(rename = "...", default))
                for tt in list.tokens.clone() {
                    if let proc_macro2::TokenTree::Group(group) = tt {
                        let inner = group.stream().to_string();
                        if has_default_in_token_str(&inner) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Check if a comma-separated token string contains a "default" item,
/// either standalone (`default`) or with a custom function (`default = "..."`).
fn has_default_in_token_str(s: &str) -> bool {
    s.split(',').any(|part| {
        let trimmed = part.trim();
        trimmed == "default" || trimmed.starts_with("default =") || trimmed.starts_with("default=")
    })
}

fn is_u64_type(ident: &Ident) -> bool {
    ident == "u64" || U64_ALIASES.iter().any(|alias| ident == alias)
}

fn is_i64_type(ident: &Ident) -> bool {
    ident == "i64" || I64_ALIASES.iter().any(|alias| ident == alias)
}

/// Derive macro that generates `impl JsonConvertible for Type {}` with
/// compile-time assertions that all inner types implement `JsonSafeFields`.
///
/// Feature gates are handled externally via `cfg_attr`:
/// ```ignore
/// #[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
/// #[serde(tag = "$formatVersion")]
/// pub enum TokenConfiguration {
///     #[serde(rename = "0")]
///     V0(TokenConfigurationV0),
/// }
/// ```
#[proc_macro_derive(JsonConvertible)]
pub fn derive_json_convertible(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Collect inner types from enum variants for compile-time assertions
    let assertions = match &input.data {
        Data::Enum(data_enum) => {
            let mut assertions = Vec::new();
            for variant in &data_enum.variants {
                if let Fields::Unnamed(fields) = &variant.fields {
                    for (i, field) in fields.unnamed.iter().enumerate() {
                        let ty = &field.ty;
                        let assert_fn_name = Ident::new(
                            &format!(
                                "must_have_json_safe_fields_attribute_{}_{}",
                                variant.ident.to_string().to_lowercase(),
                                i
                            ),
                            Span::call_site(),
                        );
                        assertions.push(quote! {
                            fn #assert_fn_name<T: crate::serialization::JsonSafeFields>() {}
                            let _ = #assert_fn_name::<#ty>;
                        });
                    }
                }
            }
            assertions
        }
        Data::Struct(_) => {
            // For plain structs, assert that Self implements JsonSafeFields
            vec![quote! {
                fn must_have_json_safe_fields_attribute<T: crate::serialization::JsonSafeFields>() {}
                let _ = must_have_json_safe_fields_attribute::<#name #ty_generics>;
            }]
        }
        Data::Union(_) => {
            return syn::Error::new_spanned(name, "JsonConvertible cannot be derived for unions")
                .to_compile_error()
                .into();
        }
    };

    let json_safe_fields_impl = match &input.data {
        Data::Enum(_) => {
            // Enums get JsonSafeFields — their inner V0 types are verified below.
            quote! {
                impl #impl_generics crate::serialization::JsonSafeFields for #name #ty_generics #where_clause {}
            }
        }
        _ => {
            // Structs already have JsonSafeFields from #[json_safe_fields] — don't duplicate.
            quote! {}
        }
    };

    let expanded = quote! {
        impl #impl_generics crate::serialization::JsonConvertible for #name #ty_generics #where_clause {}

        #json_safe_fields_impl

        const _: () = {
            #(#assertions)*
        };
    };

    expanded.into()
}

/// Derive macro that generates `impl ValueConvertible for Type {}`.
///
/// `ValueConvertible` has default method implementations that use `platform_value`
/// serialization, so the generated impl is always empty.
///
/// ```ignore
/// #[derive(ValueConvertible)]
/// pub enum ExtendedBlockInfo {
///     V0(ExtendedBlockInfoV0),
/// }
/// ```
#[proc_macro_derive(ValueConvertible)]
pub fn derive_value_convertible(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    if let Data::Union(_) = &input.data {
        return syn::Error::new_spanned(name, "ValueConvertible cannot be derived for unions")
            .to_compile_error()
            .into();
    }

    let expanded = quote! {
        impl #impl_generics crate::serialization::ValueConvertible for #name #ty_generics #where_clause {}
    };

    expanded.into()
}
