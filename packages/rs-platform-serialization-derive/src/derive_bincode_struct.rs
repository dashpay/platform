use crate::attribute::{ContainerAttributes, FieldAttributes};
use proc_macro::Delimiter;
use virtue::generate::Generator;
use virtue::parse::Fields;
use virtue::prelude::*;

pub(crate) struct DeriveStruct {
    pub fields: Option<Fields>,
    pub attributes: ContainerAttributes,
}

impl DeriveStruct {
    pub fn generate_encode(self, generator: &mut Generator) -> Result<()> {
        let crate_name = &self.attributes.crate_name;
        generator
            .impl_for(format!("{}::PlatformVersionEncode", crate_name))
            .modify_generic_constraints(|generics, where_constraints| {
                if let Some((bounds, lit)) =
                    (self.attributes.encode_bounds.as_ref()).or(self.attributes.bounds.as_ref())
                {
                    where_constraints.clear();
                    where_constraints
                        .push_parsed_constraint(bounds)
                        .map_err(|e| e.with_span(lit.span()))?;
                } else {
                    for g in generics.iter_generics() {
                        where_constraints
                            .push_constraint(g, format!("{}::PlatformVersionEncode", crate_name))
                            .unwrap();
                    }
                }
                Ok(())
            })?
            .generate_fn("platform_encode")
            .with_generic_deps("__E", ["bincode::enc::Encoder"])
            .with_self_arg(virtue::generate::FnSelfArg::RefSelf)
            .with_arg("encoder", "&mut __E")
            .with_arg("platform_version", "&platform_version::version::PlatformVersion")
            .with_return_type("core::result::Result<(), bincode::error::EncodeError>".to_string())
            .body(|fn_body| {
                if let Some(fields) = self.fields.as_ref() {
                    for field in fields.names() {
                        let attributes = field
                            .attributes()
                            .get_attribute::<FieldAttributes>()?
                            .unwrap_or_default();
                        if attributes.with_serde {
                            fn_body.push_parsed(format!(
                                "{0}::Encode::encode(&bincode::serde::Compat(&self.{1}), encoder)?;",
                                crate_name, field
                            ))?;
                        } else if attributes.with_platform_version {
                            fn_body.push_parsed(format!(
                                "{}::PlatformVersionEncode::platform_encode(&self.{}, encoder, platform_version)?;",
                                crate_name, field
                            ))?;
                        } else {
                            fn_body.push_parsed(format!(
                                "{}::Encode::encode(&self.{}, encoder)?;",
                                crate_name, field
                            ))?;
                        }
                    }
                }
                fn_body.push_parsed("Ok(())")?;
                Ok(())
            })?;
        Ok(())
    }

    pub fn generate_decode(self, generator: &mut Generator) -> Result<()> {
        // Remember to keep this mostly in sync with generate_borrow_decode
        let crate_name = &self.attributes.crate_name;

        generator
            .impl_for(format!("{}::PlatformVersionedDecode", crate_name))
            .modify_generic_constraints(|generics, where_constraints| {
                if let Some((bounds, lit)) = (self.attributes.decode_bounds.as_ref()).or(self.attributes.bounds.as_ref()) {
                    where_constraints.clear();
                    where_constraints.push_parsed_constraint(bounds).map_err(|e| e.with_span(lit.span()))?;
                } else {
                    for g in generics.iter_generics() {
                        where_constraints.push_constraint(g, format!("{}::PlatformVersionedDecode", crate_name)).unwrap();
                    }
                }
                Ok(())
            })?
            .generate_fn("platform_versioned_decode")
            .with_generic_deps("__D", [format!("{}::de::Decoder", crate_name)])
            .with_arg("decoder", "&mut __D")
            .with_arg("platform_version", "&platform_version::version::PlatformVersion")
            .with_return_type(format!("core::result::Result<Self, {}::error::DecodeError>", crate_name))
            .body(|fn_body| {
                // Ok(Self {
                fn_body.ident_str("Ok");
                fn_body.group(Delimiter::Parenthesis, |ok_group| {
                    ok_group.ident_str("Self");
                    ok_group.group(Delimiter::Brace, |struct_body| {
                        // Fields
                        // {
                        //      a: bincode::Decode::decode(decoder)?,
                        //      b: bincode::Decode::decode(decoder)?,
                        //      ...
                        // }
                        if let Some(fields) = self.fields.as_ref() {
                            for field in fields.names() {
                                let attributes = field.attributes().get_attribute::<FieldAttributes>()?.unwrap_or_default();
                                if attributes.with_serde {
                                    struct_body
                                        .push_parsed(format!(
                                            "{1}: (<bincode::serde::Compat<_> as {0}::Decode>::decode(decoder)?).0,",
                                            crate_name,
                                            field
                                        ))?;
                                } else if attributes.with_platform_version {
                                    struct_body
                                        .push_parsed(format!(
                                            "{1}: (<bincode::serde::Compat<_> as {0}::PlatformVersionedDecode>::platform_versioned_decode(decoder, platform_version)?),",
                                            crate_name,
                                            field
                                        ))?;
                                } else {
                                    struct_body
                                        .push_parsed(format!(
                                            "{1}: {0}::Decode::decode(decoder)?,",
                                            crate_name,
                                            field
                                        ))?;
                                }
                            }
                        }
                        Ok(())
                    })?;
                    Ok(())
                })?;
                Ok(())
            })?;
        self.generate_borrow_decode(generator)?;
        Ok(())
    }

    pub fn generate_borrow_decode(self, generator: &mut Generator) -> Result<()> {
        // Remember to keep this mostly in sync with generate_decode
        //let crate_name = self.attributes.crate_name;
        let crate_name = "platform_serialization";

        generator
            .impl_for_with_lifetimes(format!("{}::PlatformVersionedBorrowDecode", crate_name), ["__de"])
            .modify_generic_constraints(|generics, where_constraints| {
                if let Some((bounds, lit)) = (self.attributes.borrow_decode_bounds.as_ref()).or(self.attributes.bounds.as_ref()) {
                    where_constraints.clear();
                    where_constraints.push_parsed_constraint(bounds).map_err(|e| e.with_span(lit.span()))?;
                } else {
                    for g in generics.iter_generics() {
                        where_constraints.push_constraint(g, format!("{}::de::BorrowDecode<'__de>", crate_name)).unwrap();
                    }
                    for lt in generics.iter_lifetimes() {
                        where_constraints.push_parsed_constraint(format!("'__de: '{}", lt.ident))?;
                    }
                }
                Ok(())
            })?
            .generate_fn("platform_versioned_borrow_decode")
            .with_generic_deps("__D", [format!("{}::de::BorrowDecoder<'__de>", crate_name)])
            .with_arg("decoder", "&mut __D")
            .with_arg("platform_version", "&platform_version::version::PlatformVersion")
            .with_return_type(format!("core::result::Result<Self, {}::error::DecodeError>", crate_name))
            .body(|fn_body| {
                // Ok(Self {
                fn_body.ident_str("Ok");
                fn_body.group(Delimiter::Parenthesis, |ok_group| {
                    ok_group.ident_str("Self");
                    ok_group.group(Delimiter::Brace, |struct_body| {
                        if let Some(fields) = self.fields.as_ref() {
                            for field in fields.names() {
                                let attributes = field.attributes().get_attribute::<FieldAttributes>()?.unwrap_or_default();
                                if attributes.with_serde {
                                    struct_body
                                        .push_parsed(format!(
                                            "{1}: (<bincode::serde::BorrowCompat<_> as {0}::BorrowDecode>::borrow_decode(decoder)?).0,",
                                            crate_name,
                                            field
                                        ))?;
                                } else if attributes.with_platform_version {
                                    struct_body
                                        .push_parsed(format!(
                                            "{1}: {0}::PlatformVersionedBorrowDecode::platform_versioned_borrow_decode(decoder, platform_version)?,",
                                            crate_name,
                                            field
                                        ))?;
                                } else {
                                    struct_body
                                        .push_parsed(format!(
                                            "{1}: {0}::BorrowDecode::borrow_decode(decoder)?,",
                                            crate_name,
                                            field
                                        ))?;
                                }
                            }
                        }
                        Ok(())
                    })?;
                    Ok(())
                })?;
                Ok(())
            })?;
        Ok(())
    }
}
