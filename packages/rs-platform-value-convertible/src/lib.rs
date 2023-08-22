extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(
    PlatformValueConvert,
    attributes(platform_error_type, platform_convert_into)
)]
pub fn derive_platform_convert(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Extract the error type from the attribute.
    let _error_type = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("platform_error_type") {
                Some(attr.parse_args::<syn::Path>().unwrap())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            syn::parse_str::<syn::Path>("ProtocolError")
                .expect("Failed to parse default error type")
        });

    let platform_convert_into: Option<syn::Path> = input.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("platform_convert_into") {
            Some(attr.parse_args::<syn::Path>().unwrap())
        } else {
            None
        }
    });

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
            self.to_object()
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

        impl #impl_generics TryFrom<&Value> for #name #ty_generics #where_clause {
            type Error = ProtocolError;

            fn try_from(value: &Value) -> Result<Self, Self::Error> {
                Self::from_object_ref(value)
            }
        }

        impl #impl_generics TryFrom<Value> for #name #ty_generics #where_clause {
            type Error = ProtocolError;

            fn try_from(value: Value) -> Result<Self, Self::Error> {
                Self::from_object(value)
            }
        }

        impl #impl_generics TryFrom<#name> for Value #ty_generics #where_clause {
            type Error = ProtocolError;

            fn try_from(value: #name) -> Result<Self, Self::Error> {
                value.into_object()
            }
        }

        impl #impl_generics TryFrom<&#name> for Value #ty_generics #where_clause {
            type Error = ProtocolError;

            fn try_from(value: &#name) -> Result<Self, Self::Error> {
                value.to_object()
            }
        }
    };

    TokenStream::from(expanded)
}
