use crate::attribute::ContainerAttributes;
use crate::{derive_bincode_enum, derive_bincode_struct};
use proc_macro::TokenStream;
use virtue::prelude::*;

pub(crate) fn derive_encode_inner(input: TokenStream) -> Result<TokenStream> {
    let parse = Parse::new(input)?;
    let (mut generator, attributes, body) = parse.into_generator();
    let attributes = attributes
        .get_attribute::<ContainerAttributes>()?
        .unwrap_or_default();

    match body {
        Body::Struct(body) => {
            derive_bincode_struct::DeriveStruct {
                fields: body.fields,
                attributes,
            }
            .generate_encode(&mut generator)?;
        }
        Body::Enum(body) => {
            derive_bincode_enum::DeriveEnum {
                variants: body.variants,
                attributes,
            }
            .generate_encode(&mut generator)?;
        }
    }

    generator.export_to_file("bincode", "Encode");
    generator.finish()
}

pub(crate) fn derive_decode_inner(input: TokenStream) -> Result<TokenStream> {
    let parse = Parse::new(input)?;
    let (mut generator, attributes, body) = parse.into_generator();
    let attributes = attributes
        .get_attribute::<ContainerAttributes>()?
        .unwrap_or_default();

    match body {
        Body::Struct(body) => {
            derive_bincode_struct::DeriveStruct {
                fields: body.fields,
                attributes,
            }
            .generate_decode(&mut generator)?;
        }
        Body::Enum(body) => {
            derive_bincode_enum::DeriveEnum {
                variants: body.variants,
                attributes,
            }
            .generate_decode(&mut generator)?;
        }
    }

    generator.export_to_file("bincode", "Decode");
    generator.finish()
}

pub(crate) fn derive_borrow_decode_inner(input: TokenStream) -> Result<TokenStream> {
    let parse = Parse::new(input)?;
    let (mut generator, attributes, body) = parse.into_generator();
    let attributes = attributes
        .get_attribute::<ContainerAttributes>()?
        .unwrap_or_default();

    match body {
        Body::Struct(body) => {
            derive_bincode_struct::DeriveStruct {
                fields: body.fields,
                attributes,
            }
            .generate_borrow_decode(&mut generator)?;
        }
        Body::Enum(body) => {
            derive_bincode_enum::DeriveEnum {
                variants: body.variants,
                attributes,
            }
            .generate_borrow_decode(&mut generator)?;
        }
    }

    generator.export_to_file("bincode", "BorrowDecode");
    generator.finish()
}
