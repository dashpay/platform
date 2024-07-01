use anyhow::Context;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentType, DocumentTypeRef};
use dpp::document::property_names::{
    CREATED_AT, FEATURE_VERSION, ID, OWNER_ID, REVISION, UPDATED_AT,
};
use dpp::document::serialization_traits::DocumentPlatformValueMethodsV0;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use platform_value::{Bytes32, Identifier, Value};

pub trait SchemaAwareRandomDocument {
    fn valid_random_documents(
        &self,
        owner_id: Identifier,
        entropy: &Bytes32,
        count: u32,
        platform_version: &PlatformVersion,
        substitutions: &BTreeMap<&str, Value>,
    ) -> Result<Vec<Document>, ProtocolError>;
}

impl<'a> SchemaAwareRandomDocument for DocumentTypeRef<'a> {
    fn valid_random_documents(
        &self,
        owner_id: Identifier,
        entropy: &Bytes32,
        count: u32,
        platform_version: &PlatformVersion,
        substitutions: &BTreeMap<&str, Value>,
    ) -> Result<Vec<Document>, ProtocolError> {
        valid_random_documents(
            self,
            owner_id,
            entropy,
            count,
            platform_version,
            substitutions,
        )
    }
}

impl SchemaAwareRandomDocument for DocumentType {
    fn valid_random_documents(
        &self,
        owner_id: Identifier,
        entropy: &Bytes32,
        count: u32,
        platform_version: &PlatformVersion,
        substitutions: &BTreeMap<&str, Value>,
    ) -> Result<Vec<Document>, ProtocolError> {
        valid_random_documents(
            &self.as_ref(),
            owner_id,
            entropy,
            count,
            platform_version,
            substitutions,
        )
    }
}

/// Create random documents using json-schema-faker-rs
fn valid_random_documents(
    document_type: &DocumentTypeRef,
    owner_id: Identifier,
    entropy: &Bytes32,
    count: u32,
    platform_version: &PlatformVersion,
    substitutions: &BTreeMap<&str, Value>,
) -> Result<Vec<Document>, ProtocolError> {
    let json_schema = document_type.schema().try_to_validating_json()?;
    let json_documents = json_schema_faker::generate(&json_schema, count as u16)
        .context("cannot generate a random document with json-schema-faker-rs")?;

    let fix_document = |mut document: Value| {
        let id = Document::generate_document_id_v0(
            &document_type.data_contract_id(),
            &owner_id,
            document_type.name().as_str(),
            entropy.as_slice(),
        );
        let now = SystemTime::now();
        let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let time_ms = duration_since_epoch.as_millis() as u64;

        if document_type.documents_mutable() {
            document.as_map_mut().into_iter().for_each(|d| {
                d.push((REVISION.into(), 1.into()));
            });
        }
        document.as_map_mut().into_iter().for_each(|d| {
            d.push((ID.into(), id.into()));
        });
        document.as_map_mut().into_iter().for_each(|d| {
            d.push((OWNER_ID.into(), owner_id.into()));
        });
        if document_type.required_fields().contains(FEATURE_VERSION) {
            document.as_map_mut().into_iter().for_each(|d| {
                d.push((FEATURE_VERSION.into(), "0".into()));
            });
        }
        if document_type.required_fields().contains(CREATED_AT) {
            document.as_map_mut().into_iter().for_each(|d| {
                d.push((CREATED_AT.into(), time_ms.into()));
            });
        }
        if document_type.required_fields().contains(UPDATED_AT) {
            document.as_map_mut().into_iter().for_each(|d| {
                d.push((UPDATED_AT.into(), time_ms.into()));
            });
        }

        document
    };

    json_documents
        .into_iter()
        .map(|d| {
            let p_value: Value = d.into();
            let fixed_value = fix_document(p_value);

            // TODO: tl;dr use PlatformDeserialize instead of Deserialize for Documents
            //
            // `properties` is a `BTreeMap` with `platform_value::Value` as values, since
            // `Document::from_platform_value` does deserialization through Serde's data model
            // it losts some information like distinction between `Value::Bytes` and `Value::Bytes32`;
            // The solution here is to let deserialize a `Document`, but put `properties` unprocessed
            // since they were `platform_value::Value` and will be the same type again and no deserialization
            // is needed, especially that lossy kind.
            let mut properties = fixed_value
                .to_map_ref()
                .ok()
                .and_then(|m| Value::map_into_btree_string_map(m.clone()).ok())
                .unwrap_or_default();
            let mut document = Document::from_platform_value(fixed_value, platform_version);
            if let Ok(Document::V0(d)) = document.as_mut() {
                // This moves stored properties back to the document so it could skip unnecessary
                // and wrong deserialization part
                d.properties.iter_mut().for_each(|(k, v)| {
                    substitutions
                        .get(k.as_str())
                        .cloned()
                        .or(properties.remove(k))
                        .into_iter()
                        .for_each(|prop| {
                            // TODO: schema and internal DocumentType representations are incompatible
                            // Properties are tweaked though, because the only integer type supported by
                            // DPP is i64, while `platform_value::Value` distincts them, and json schema is
                            // even more permissive; however, we want our proofs to work and proofs use the
                            // DPP model.
                            *v = match prop {
                                Value::U64(x) => Value::I64(x as i64),
                                Value::U32(x) => Value::I64(x as i64),
                                Value::I32(x) => Value::I64(x as i64),
                                x => x,
                            };
                        })
                });
            }
            document
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use dpp::dashcore::secp256k1::rand::rngs::StdRng;
    use dpp::dashcore::secp256k1::rand::SeedableRng;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};

    use super::*;

    #[test]
    fn test_random_document_faker() {
        let data_contract =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest()).unwrap();
        let mut rng = StdRng::from_entropy();
        let entropy = Bytes32::random_with_rng(&mut rng);

        let _random_documents = data_contract
            .document_types()
            .iter()
            .next()
            .unwrap()
            .1
            .valid_random_documents(
                Identifier::random(),
                &entropy,
                2,
                PlatformVersion::latest(),
                &Default::default(),
            );
    }
}
