pub use fields::{property_names, IDENTIFIER_FIELDS};

mod accessors;
#[cfg(feature = "client")]
mod document_facade;
#[cfg(feature = "factories")]
pub mod document_factory;
pub mod document_methods;
mod document_patch;
pub mod errors;
#[cfg(feature = "extended-document")]
pub mod extended_document;
mod fields;
pub mod generate_document_id;
pub mod serialization_traits;
#[cfg(feature = "factories")]
pub mod specialized_document_factory;
mod v0;

pub use accessors::*;
pub use v0::*;

#[cfg(feature = "extended-document")]
pub use extended_document::property_names as extended_document_property_names;
#[cfg(feature = "extended-document")]
pub use extended_document::ExtendedDocument;
#[cfg(feature = "extended-document")]
pub use extended_document::IDENTIFIER_FIELDS as EXTENDED_DOCUMENT_IDENTIFIER_FIELDS;

/// the initial revision of newly created document
pub const INITIAL_REVISION: u64 = 1;

use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::DataContract;
use crate::document::document_methods::{
    DocumentGetRawForContractV0, DocumentGetRawForDocumentTypeV0, DocumentHashV0Method,
    DocumentMethodsV0,
};
use crate::document::errors::DocumentError;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use derive_more::From;

#[cfg(feature = "document-serde-conversion")]
use serde::{Deserialize, Serialize};

use std::fmt;
use std::fmt::Formatter;

#[derive(Clone, Debug, PartialEq, From)]
#[cfg_attr(
    feature = "document-serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$version")
)]
pub enum Document {
    #[cfg_attr(feature = "document-serde-conversion", serde(rename = "0"))]
    V0(DocumentV0),
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Document::V0(v0) => {
                write!(f, "v0 : {} ", v0)?;
            }
        }
        Ok(())
    }
}

impl DocumentMethodsV0 for Document {
    /// Return a value given the path to its key and the document type for a contract.
    fn get_raw_for_contract(
        &self,
        key: &str,
        document_type_name: &str,
        contract: &DataContract,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        match self {
            Document::V0(document_v0) => {
                match platform_version
                    .dpp
                    .document_versions
                    .document_method_versions
                    .get_raw_for_contract
                {
                    0 => document_v0.get_raw_for_contract_v0(
                        key,
                        document_type_name,
                        contract,
                        owner_id,
                        platform_version,
                    ),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "DocumentMethodV0::get_raw_for_contract".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }

    /// Return a value given the path to its key for a document type.
    fn get_raw_for_document_type(
        &self,
        key_path: &str,
        document_type: DocumentTypeRef,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        match self {
            Document::V0(document_v0) => {
                match platform_version
                    .dpp
                    .document_versions
                    .document_method_versions
                    .get_raw_for_document_type
                {
                    0 => document_v0.get_raw_for_document_type_v0(
                        key_path,
                        document_type,
                        owner_id,
                        platform_version,
                    ),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "DocumentMethodV0::get_raw_for_document_type".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }

    fn hash(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Document::V0(document_v0) => {
                match platform_version
                    .dpp
                    .document_versions
                    .document_method_versions
                    .hash
                {
                    0 => document_v0.hash_v0(contract, document_type, platform_version),
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "DocumentMethodV0::hash".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }

    fn increment_revision(&mut self) -> Result<(), ProtocolError> {
        let Some(revision) = self.revision() else {
            return Err(ProtocolError::Document(Box::new(
                DocumentError::DocumentNoRevisionError {
                    document: Box::new(self.clone()),
                },
            )));
        };

        let new_revision = revision
            .checked_add(1)
            .ok_or(ProtocolError::Overflow("overflow when adding 1"))?;

        self.set_revision(Some(new_revision));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::serialization::serialization_traits::PlatformDeserializable;
    use crate::state_transition::StateTransition;
    use crate::tests::json_document::json_document_to_contract;
    use regex::Regex;

    #[test]
    fn test_document_display() {
        let platform_version = PlatformVersion::first();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let st1 = StateTransition::deserialize_from_bytes(&[
            2, 0, 116, 26, 160, 119, 54, 238, 148, 17, 43, 60, 246, 186, 64, 187, 11, 223, 76, 127,
            6, 139, 49, 21, 160, 79, 181, 213, 252, 63, 154, 51, 205, 98, 1, 0, 0, 0, 202, 62, 64,
            7, 54, 157, 104, 42, 69, 190, 237, 62, 72, 5, 125, 75, 136, 215, 102, 13, 12, 244, 172,
            203, 2, 118, 125, 129, 33, 170, 221, 227, 6, 100, 111, 109, 97, 105, 110, 230, 104,
            198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126, 10, 29, 113, 42, 9,
            196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85, 221, 180, 146, 64, 45, 97, 135, 206,
            56, 109, 118, 191, 112, 130, 250, 204, 152, 107, 137, 165, 241, 142, 216, 225, 88, 200,
            155, 182, 25, 208, 244, 202, 0, 0, 6, 5, 108, 97, 98, 101, 108, 18, 6, 115, 105, 109,
            111, 110, 49, 15, 110, 111, 114, 109, 97, 108, 105, 122, 101, 100, 76, 97, 98, 101,
            108, 18, 6, 115, 105, 109, 111, 110, 49, 26, 110, 111, 114, 109, 97, 108, 105, 122,
            101, 100, 80, 97, 114, 101, 110, 116, 68, 111, 109, 97, 105, 110, 78, 97, 109, 101, 18,
            4, 100, 97, 115, 104, 12, 112, 114, 101, 111, 114, 100, 101, 114, 83, 97, 108, 116, 10,
            32, 226, 102, 94, 244, 196, 126, 137, 57, 70, 134, 35, 46, 251, 203, 158, 239, 213, 16,
            232, 246, 157, 52, 0, 43, 24, 241, 231, 139, 48, 247, 160, 176, 7, 114, 101, 99, 111,
            114, 100, 115, 22, 1, 18, 20, 100, 97, 115, 104, 85, 110, 105, 113, 117, 101, 73, 100,
            101, 110, 116, 105, 116, 121, 73, 100, 16, 116, 26, 160, 119, 54, 238, 148, 17, 43, 60,
            246, 186, 64, 187, 11, 223, 76, 127, 6, 139, 49, 21, 160, 79, 181, 213, 252, 63, 154,
            51, 205, 98, 14, 115, 117, 98, 100, 111, 109, 97, 105, 110, 82, 117, 108, 101, 115, 22,
            1, 18, 15, 97, 108, 108, 111, 119, 83, 117, 98, 100, 111, 109, 97, 105, 110, 115, 19,
            0, 1, 65, 32, 207, 131, 24, 137, 73, 192, 80, 153, 63, 33, 57, 168, 224, 140, 60, 41,
            89, 217, 57, 197, 108, 34, 170, 123, 47, 131, 238, 143, 106, 232, 122, 112, 96, 118,
            104, 117, 42, 190, 58, 149, 98, 154, 69, 195, 238, 83, 107, 174, 105, 63, 29, 241, 60,
            255, 196, 31, 131, 251, 161, 225, 47, 213, 72, 210,
        ])
        .unwrap();

        let st2 = StateTransition::deserialize_from_bytes(&[
            2, 0, 116, 26, 160, 119, 54, 238, 148, 17, 43, 60, 246, 186, 64, 187, 11, 223, 76, 127,
            6, 139, 49, 21, 160, 79, 181, 213, 252, 63, 154, 51, 205, 98, 1, 0, 0, 0, 1, 153, 5,
            130, 110, 79, 3, 81, 34, 138, 137, 180, 115, 125, 180, 229, 115, 195, 166, 69, 70, 167,
            127, 211, 236, 134, 169, 23, 122, 51, 44, 16, 6, 100, 111, 109, 97, 105, 110, 230, 104,
            198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126, 10, 29, 113, 42, 9,
            196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85, 120, 65, 73, 102, 70, 206, 114, 28,
            241, 152, 52, 143, 56, 186, 10, 61, 124, 162, 108, 89, 121, 235, 242, 72, 143, 45, 72,
            114, 114, 37, 87, 142, 0, 0, 6, 5, 108, 97, 98, 101, 108, 18, 6, 115, 105, 109, 111,
            110, 49, 15, 110, 111, 114, 109, 97, 108, 105, 122, 101, 100, 76, 97, 98, 101, 108, 18,
            6, 115, 105, 109, 111, 110, 49, 26, 110, 111, 114, 109, 97, 108, 105, 122, 101, 100,
            80, 97, 114, 101, 110, 116, 68, 111, 109, 97, 105, 110, 78, 97, 109, 101, 18, 4, 100,
            97, 115, 104, 12, 112, 114, 101, 111, 114, 100, 101, 114, 83, 97, 108, 116, 10, 32, 74,
            178, 241, 94, 14, 85, 101, 211, 33, 7, 180, 185, 26, 205, 144, 29, 11, 11, 15, 130,
            127, 205, 119, 109, 232, 20, 26, 116, 101, 249, 132, 169, 7, 114, 101, 99, 111, 114,
            100, 115, 22, 1, 18, 20, 100, 97, 115, 104, 85, 110, 105, 113, 117, 101, 73, 100, 101,
            110, 116, 105, 116, 121, 73, 100, 16, 116, 26, 160, 119, 54, 238, 148, 17, 43, 60, 246,
            186, 64, 187, 11, 223, 76, 127, 6, 139, 49, 21, 160, 79, 181, 213, 252, 63, 154, 51,
            205, 98, 14, 115, 117, 98, 100, 111, 109, 97, 105, 110, 82, 117, 108, 101, 115, 22, 1,
            18, 15, 97, 108, 108, 111, 119, 83, 117, 98, 100, 111, 109, 97, 105, 110, 115, 19, 0,
            1, 65, 32, 216, 75, 115, 120, 91, 252, 71, 39, 126, 60, 213, 239, 183, 74, 166, 107,
            21, 255, 216, 0, 182, 203, 42, 78, 193, 78, 135, 35, 5, 143, 144, 146, 69, 108, 209,
            207, 110, 42, 154, 13, 117, 133, 222, 108, 115, 153, 152, 123, 208, 254, 4, 16, 239,
            230, 157, 172, 165, 163, 130, 197, 13, 34, 251, 74,
        ])
        .unwrap();

        // let StateTransition::DataContractCreate(st1) = st1 else { panic!() };
        //
        // let StateTransition::DataContractCreate(st2) = st2 else { panic!() };

        dbg!(st1, st2);

        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected to get profile document type");
        let document = document_type
            .random_document(Some(3333), platform_version)
            .expect("expected to get a random document");

        let document_string = format!("{}", document);

        let pattern = r#"id:45ZNwGcxeMpLpYmiVEKKBKXbZfinrhjZLkau1GWizPFX owner_id:2vq574DjKi7ZD8kJ6dMHxT5wu6ZKD2bW5xKAyKAGW7qZ created_at:(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) updated_at:(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) avatarUrl:string y8RD1DbW18RuyblDX7hx\[...\(670\)\] displayName:string SvAQrzsslj0ESc15GQB publicMessage:string ccpKt9ckWftHIEKdBlas\[...\(36\)\] .*"#;
        let re = Regex::new(pattern).unwrap();
        assert!(re.is_match(document_string.as_str()));
    }
}
