pub mod accessors;
mod property;
pub use property::*;
pub mod class_methods;
mod index;
pub mod methods;
pub use index::*;
mod index_level;
pub use index_level::IndexLevel;
pub use index_level::IndexLevelTypeInfo;
pub use index_level::IndexType;

#[cfg(feature = "random-documents")]
pub mod random_document;
pub mod restricted_creation;
pub mod schema;

mod token_costs;
pub mod v0;
pub mod v1;
pub mod v2;
#[cfg(feature = "validation")]
pub(crate) mod validator;

use crate::data_contract::document_type::methods::{
    DocumentTypeBasicMethods, DocumentTypeV0Methods,
};
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::document::Document;
use crate::fee::Credits;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use derive_more::From;

pub const DEFAULT_HASH_SIZE: usize = 32;
pub const DEFAULT_FLOAT_SIZE: usize = 8;
pub const EMPTY_TREE_STORAGE_SIZE: usize = 33;
pub const MAX_INDEX_SIZE: usize = 255;
pub const STORAGE_FLAGS_SIZE: usize = 2;

/// A `requiredSince` annotation may never exceed the version of the contract
/// carrying it — requiredness cannot be pre-scheduled at a future version.
/// Runs over the *parsed* properties, so annotations reached through `$ref`
/// are covered. Called wherever document types are built from a contract's
/// serialized form (creates, updates, and disk loads all pass through
/// there); a no-op for every contract predating the keyword, since their
/// properties carry no annotation.
///
/// The failure is the dedicated consensus error, because the input is
/// untrusted schema data: state-transition processing must classify it as
/// consensus-invalid (nonce bump), never as an execution error. Callers map
/// it through
/// [`class_methods::consensus_or_protocol_required_fields_error`].
pub(crate) fn validate_required_since_within_contract_version(
    document_types: &std::collections::BTreeMap<String, DocumentType>,
    contract_version: u32,
) -> Result<(), crate::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError>
{
    use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;

    for (document_type_name, document_type) in document_types {
        for (property_name, property) in document_type.as_ref().properties() {
            if let Some(required_since) = property.required_since {
                if required_since > contract_version {
                    return Err(
                        crate::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError::new(
                            document_type_name.clone(),
                            format!(
                                "property '{property_name}' carries requiredSince {required_since} which exceeds the contract version {contract_version}"
                            ),
                        ),
                    );
                }
            }
        }
    }
    Ok(())
}

pub(crate) mod property_names {
    pub const DOCUMENTS_KEEP_HISTORY: &str = "documentsKeepHistory";
    pub const KEEPS_TRANSFER_HISTORY: &str = "keepsTransferHistory";
    pub const KEEPS_PURCHASE_HISTORY: &str = "keepsPurchaseHistory";
    pub const KEEPS_PRICING_HISTORY: &str = "keepsPricingHistory";
    pub const DOCUMENTS_MUTABLE: &str = "documentsMutable";

    pub const CAN_BE_DELETED: &str = "canBeDeleted";
    pub const TRANSFERABLE: &str = "transferable";
    pub const TRADE_MODE: &str = "tradeMode";

    pub const CREATION_RESTRICTION_MODE: &str = "creationRestrictionMode";
    pub const SECURITY_LEVEL_REQUIREMENT: &str = "signatureSecurityLevelRequirement";
    pub const REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY: &str =
        "requiresIdentityEncryptionBoundedKey";
    pub const REQUIRES_IDENTITY_DECRYPTION_BOUNDED_KEY: &str =
        "requiresIdentityDecryptionBoundedKey";
    pub const INDICES: &str = "indices";
    pub const NULL_SEARCHABLE: &str = "nullSearchable";
    pub const PROPERTIES: &str = "properties";
    pub const POSITION: &str = "position";
    pub const REQUIRED: &str = "required";
    pub const REQUIRED_SINCE: &str = "requiredSince";
    pub const TRANSIENT: &str = "transient";
    pub const TYPE: &str = "type";
    pub const REF: &str = "$ref";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
    pub const TRANSFERRED_AT: &str = "$transferredAt";
    pub const MINIMUM: &str = "minimum";
    pub const ENUM: &str = "enum";
    pub const MAXIMUM: &str = "maximum";
    pub const MIN_ITEMS: &str = "minItems";
    pub const MAX_ITEMS: &str = "maxItems";
    pub const MIN_LENGTH: &str = "minLength";
    pub const MAX_LENGTH: &str = "maxLength";
    pub const BYTE_ARRAY: &str = "byteArray";
    pub const CONTENT_MEDIA_TYPE: &str = "contentMediaType";
    pub const ENCRYPTION_KEY_REQUIREMENTS: &str = "encryptionKeyReqs";
    pub const DECRYPTION_KEY_REQUIREMENTS: &str = "decryptionKeyReqs";
    pub const REFERS_TO: &str = "refersTo";
    pub const CONTRACT_ID: &str = "contractId";
    pub const DOCUMENT_TYPE: &str = "documentType";
    pub const KEY_ID_PROPERTY: &str = "keyIdProperty";
    pub const DOCUMENTS_COUNTABLE: &str = "documentsCountable";
    pub const RANGE_COUNTABLE: &str = "rangeCountable";
    /// Doctype-level flag naming the property whose values are summed into
    /// the primary-key tree's running aggregate. When set, the primary-key
    /// tree is a `SumTree` (or `ProvableSumTree` if [`RANGE_SUMMABLE`] is
    /// also set), enabling O(1) `sum(named_property)` for the whole
    /// document type. See `book/src/drive/document-sum-trees.md`.
    pub const DOCUMENTS_SUMMABLE: &str = "documentsSummable";
    /// Doctype-level flag upgrading the primary-key sum tree to its
    /// provable variant (per-node aggregated sums committed to each
    /// merk-internal node's hash), so range queries on the primary key
    /// can be answered with an `AggregateSumOnRange` O(log n) proof.
    /// Requires [`DOCUMENTS_SUMMABLE`] to be set.
    pub const RANGE_SUMMABLE: &str = "rangeSummable";
    /// Doctype-level syntactic sugar for the combination of
    /// `documentsCountable: true` + [`DOCUMENTS_SUMMABLE`]`: "<prop>"`.
    /// Average queries return `(count, sum)` pairs the client divides
    /// — same on-disk layout as setting both flags directly. Authors
    /// who think in terms of averages get a single flag; the parser
    /// in `try_from_schema/v2` desugars it into the underlying
    /// count + sum flags so all downstream code paths (insert, query,
    /// estimation) stay unchanged.
    pub const DOCUMENTS_AVERAGEABLE: &str = "documentsAverageable";
    /// Doctype-level syntactic sugar for [`RANGE_COUNTABLE`]`: true` +
    /// [`RANGE_SUMMABLE`]`: true`. Requires [`DOCUMENTS_AVERAGEABLE`]
    /// to be set (parallels the count/sum-individually rules: range
    /// axes require the corresponding base flag).
    pub const RANGE_AVERAGEABLE: &str = "rangeAverageable";
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DocumentTypeRef<'a> {
    V0(&'a DocumentTypeV0),
    V1(&'a DocumentTypeV1),
    V2(&'a DocumentTypeV2),
}

#[derive(Debug)]
pub enum DocumentTypeMutRef<'a> {
    V0(&'a mut DocumentTypeV0),
    V1(&'a mut DocumentTypeV1),
    V2(&'a mut DocumentTypeV2),
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, From)]
pub enum DocumentType {
    V0(DocumentTypeV0),
    V1(DocumentTypeV1),
    V2(DocumentTypeV2),
}

impl DocumentType {
    pub const fn as_ref(&self) -> DocumentTypeRef<'_> {
        match self {
            DocumentType::V0(v0) => DocumentTypeRef::V0(v0),
            DocumentType::V1(v1) => DocumentTypeRef::V1(v1),
            DocumentType::V2(v2) => DocumentTypeRef::V2(v2),
        }
    }

    pub fn as_mut_ref(&mut self) -> DocumentTypeMutRef<'_> {
        match self {
            DocumentType::V0(v0) => DocumentTypeMutRef::V0(v0),
            DocumentType::V1(v1) => DocumentTypeMutRef::V1(v1),
            DocumentType::V2(v2) => DocumentTypeMutRef::V2(v2),
        }
    }

    pub fn prefunded_voting_balances_for_document(
        &self,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(String, Credits)>, ProtocolError> {
        match self {
            DocumentType::V0(v0) => {
                v0.prefunded_voting_balance_for_document(document, platform_version)
            }
            DocumentType::V1(v1) => {
                v1.prefunded_voting_balance_for_document(document, platform_version)
            }
            DocumentType::V2(v2) => {
                v2.prefunded_voting_balance_for_document(document, platform_version)
            }
        }
    }
}

impl DocumentTypeRef<'_> {
    pub fn to_owned_document_type(&self) -> DocumentType {
        match self {
            DocumentTypeRef::V0(v0) => DocumentType::V0((*v0).to_owned()),
            DocumentTypeRef::V1(v1) => DocumentType::V1((*v1).to_owned()),
            DocumentTypeRef::V2(v2) => DocumentType::V2((*v2).to_owned()),
        }
    }
}

impl DocumentTypeBasicMethods for DocumentType {}

impl DocumentTypeBasicMethods for DocumentTypeRef<'_> {}

impl DocumentTypeV0Methods for DocumentType {}

impl DocumentTypeV0Methods for DocumentTypeRef<'_> {}
