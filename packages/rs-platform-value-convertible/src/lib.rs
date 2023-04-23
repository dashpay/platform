extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::spanned::Spanned;
use syn::Error;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Meta, NestedMeta};

#[proc_macro_derive(
    PlatformValueConvert,
    attributes(platform_error_type, platform_convert_into, platform_value)
)]
pub fn derive_platform_convert(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Extract the error type from the attribute.
    let error_type = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("platform_error_type") {
                Some(attr.parse_args::<syn::Path>().unwrap())
            } else {
                None
            }
        })
        .expect("Missing platform_error_type attribute");

    let platform_convert_into: Option<syn::Path> = input.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("platform_convert_into") {
            Some(attr.parse_args::<syn::Path>().unwrap())
        } else {
            None
        }
    });

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    let mut filtered_fields = vec![];
    for field in fields {
        if !field.attrs.iter().any(|attr| {
            attr.path.is_ident("platform_value") && attr.parse_args::<Meta>().ok().map_or(false, |meta| {
                matches!(meta, Meta::List(list) if list.nested.iter().any(|n| matches!(n, NestedMeta::Meta(Meta::Path(p)) if p.is_ident("skip"))))
            })
        }) {
            filtered_fields.push(field);
        }
    }

    let field_idents: Vec<_> = filtered_fields.iter().map(|f| &f.ident).collect();

    // Extract the generics.
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let convert_from_value = match &platform_convert_into {
        Some(inner) => quote! {
            #inner::from_object(value).map(Into::into)
        },
        None => quote! {
            Self::from_object(value)
        },
    };

    let convert_from_value_ref = match &platform_convert_into {
        Some(inner) => quote! {
            #inner::from_object_ref(value).map(Into::into)
        },
        None => quote! {
            Self::from_object_ref(value)
        },
    };

    let convert_into_value = match &platform_convert_into {
        Some(inner) => quote! {
            let inner: #inner = self.clone().into();
            inner.into_object()
        },
        None => quote! {
            self.into_object()
        },
    };

    let convert_to_object = match &platform_convert_into {
        Some(inner) => quote! {
            let inner: #inner = self.clone().into();
            inner.to_object()
        },
        None => quote! {
                       platform_value::to_value_with_filter(self, |field_name| {
                let field_names = vec![#(#field_idents),*];
                field_names.contains(&field_name)
            }).map_err(#error_type::ValueError)
        },
    };

    let expanded = quote! {
        impl #impl_generics ValueConvertible for #name #ty_generics #where_clause
        {
            fn to_object(&self) -> Result<Value, ProtocolError> {
                #convert_to_object
            }

            fn into_object(self) -> Result<Value, ProtocolError> {
                #convert_into_value
            }

            fn from_object(value: Value) -> Result<Self, ProtocolError> {
                #convert_from_value
            }

            fn from_object_ref(value: &Value) -> Result<Self, ProtocolError> {
                #convert_from_value_ref
            }
        }
    };

    TokenStream::from(expanded)
}
