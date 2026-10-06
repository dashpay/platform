pub mod accessors;
pub mod action_fees;
mod property;
pub use property::*;
pub mod class_methods;
mod index;
pub mod methods;
pub use index::*;
mod index_level;
pub mod property_constraints;
pub use index_level::IndexLevel;
pub use index_level::IndexLevelTypeInfo;
pub use index_level::IndexType;

#[cfg(feature = "random-documents")]
pub mod random_document;
pub mod restricted_creation;
pub mod schema;

mod token_costs;
mod validate_required_since_within_contract_version;
pub(crate) use validate_required_since_within_contract_version::validate_required_since_within_contract_version;
pub mod v0;
pub mod v1;
pub mod v2;
#[cfg(feature = "validation")]
pub(crate) mod validator;

use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
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
/// Worst-case byte length of the contract-version stamp written by document
/// serialization format 3: a u32 varint.
pub const CONTRACT_VERSION_STAMP_MAX_SIZE: u16 = 5;

pub(crate) mod property_names {
    pub const DOCUMENTS_KEEP_HISTORY: &str = "documentsKeepHistory";
    pub const KEEPS_TRANSFER_HISTORY: &str = "keepsTransferHistory";
    pub const KEEPS_PURCHASE_HISTORY: &str = "keepsPurchaseHistory";
    pub const KEEPS_PRICING_HISTORY: &str = "keepsPricingHistory";
    pub const DOCUMENTS_MUTABLE: &str = "documentsMutable";

    pub const CAN_BE_DELETED: &str = "canBeDeleted";
    /// The `canBeDeleted` value saying a document leaves state only when a
    /// `refersTo` with `consume` deletes it, never by its owner's delete.
    /// Meta-schema v3+ (protocol version 14).
    pub const CAN_BE_DELETED_ONLY_WHEN_CONSUMED: &str = "onlyWhenConsumed";
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
    /// Doctype-level array of the top-level properties of a **mutable**
    /// document type a replace may not change, add or remove: a property
    /// name, frozen at creation, or `{ "property": ..., "when": ... }`
    /// ([`immutable_entry`]), frozen while its condition holds. Meta-schema
    /// v3+ (protocol version 14). See `apply_immutable_fields` in
    /// `try_from_schema::common` for the structural rules.
    pub const IMMUTABLE: &str = "immutable";
    /// Doctype-level condition, in the grammar of an [`IMMUTABLE`] entry's
    /// `when`, under which a replace writes a retracted document: the one
    /// replace a banned or suspended owner may still make on a moderated
    /// contract. Meta-schema v3+ (protocol version 14). See
    /// `apply_retracted_when` in `try_from_schema::common`.
    pub const RETRACTED_WHEN: &str = "retractedWhen";
    /// Doctype-level object declaring a fixed fee in credits for actions on documents of
    /// the type, split between the contract's owner pot and its moderators pot. Meta-schema
    /// v3+ (protocol version 14). See `parse_action_fees_keyword` in
    /// `try_from_schema::common`.
    pub const ACTION_FEES: &str = "actionFees";
    /// Doctype-level object of token costs, one per document action (`create`,
    /// `replace`, `delete`, `transfer`, `update_price`, `purchase`). Meta-schema
    /// v0+ (protocol version 9). See `parse_token_costs` in `try_from_schema::common`.
    pub const TOKEN_COST: &str = "tokenCost";
    /// The keyword that listed the [`IMMUTABLE`] properties a replace could
    /// still set while absent, before a conditional `immutable` entry said it
    /// (`{ "property": "p", "when": { "present": "$old.p" } }`). Refused on
    /// every parse, naming its replacement, so that no contract written with
    /// it loads with another meaning.
    pub const IMMUTABLE_ALLOW_SETTING: &str = "immutableAllowSetting";
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
    pub const ITEMS: &str = "items";
    pub const UNIQUE_ITEMS: &str = "uniqueItems";
    pub const MIN_PROPERTIES: &str = "minProperties";
    pub const MAX_PROPERTIES: &str = "maxProperties";
    pub const CONTAINS: &str = "contains";
    pub const MIN_LENGTH: &str = "minLength";
    pub const MAX_LENGTH: &str = "maxLength";
    pub const BYTE_ARRAY: &str = "byteArray";
    pub const CONTENT_MEDIA_TYPE: &str = "contentMediaType";
    pub const ENCRYPTION_KEY_REQUIREMENTS: &str = "encryptionKeyReqs";
    pub const DECRYPTION_KEY_REQUIREMENTS: &str = "decryptionKeyReqs";
    pub const REFERS_TO: &str = "refersTo";
    /// Doctype-level `refersTo` declaration whose value is the document's
    /// `$ownerId`, the writer, rather than a property's value, only on a type
    /// whose documents cannot change owner. Meta-schema v3+ (protocol version
    /// 14). See `parse_doctype_reference` in `try_from_schema`.
    pub const OWNER_REFERS_TO: &str = "ownerRefersTo";
    /// Doctype-level `refersTo` declaration whose value is the document's
    /// `$creatorId`, its creator, only on a type that records creator ids (a
    /// transferable or tradeable one). Meta-schema v3+ (protocol version 14).
    /// See `parse_doctype_reference` in `try_from_schema`.
    pub const CREATOR_REFERS_TO: &str = "creatorRefersTo";
    /// Doctype-level object of named rules, each a condition on the document's
    /// properties (a comparison of two integer expressions, which may read a
    /// `countOf` or `sumOf` total of a type of the contract, of a string or an
    /// identifier property with constants or with another property of its
    /// kind, an `in` or `notIn` list of values, a `startsWith` or `endsWith`,
    /// a `contains`, a `present` or `absent` test, or an `anyOf`, `allOf`,
    /// `not`, `ifThen` or `ifThenElse` of conditions) that every created or replaced
    /// document must meet. Meta-schema v3+ (protocol version 14). See
    /// `parse_property_constraints` in `property_constraints`.
    pub const PROPERTY_CONSTRAINTS: &str = "propertyConstraints";
    pub const DISTINCT_FROM: &str = "distinctFrom";
    pub const CONTRACT_ID: &str = "contractId";
    pub const DOCUMENT_TYPE: &str = "documentType";
    pub const KEY_ID_PROPERTY: &str = "keyIdProperty";
    /// `identityPublicKey` reference on the key id property itself: whose key
    /// the value names (`"$ownerId"`, the writer). Takes the place of
    /// [`KEY_ID_PROPERTY`]; a declaration carries one or the other.
    pub const IDENTITY_PROPERTY: &str = "identityProperty";
    /// `refersTo` on a document reference: how the referenced document is
    /// found when the value is not its id, properties of the referenced
    /// document type (exactly those of one of its unique indexes) mapped to
    /// where each value comes from on the referring side. Meta-schema v3+
    /// (protocol version 14).
    pub const FIND_BY: &str = "findBy";
    /// `refersTo` on a document reference: what the referenced document must
    /// hold once found, properties of the referenced document type mapped to
    /// the referring-side value each must equal. Meta-schema v3+ (protocol
    /// version 14).
    pub const WHERE: &str = "where";
    /// `refersTo` beside a `findBy` with a computed key: how many blocks
    /// before the referring document's create the document the key finds
    /// must have been created.
    pub const MINIMUM_AGE_BLOCKS: &str = "minimumAgeBlocks";
    /// `refersTo` beside a `findBy` with a computed key: whether the create
    /// deletes the document the key finds.
    pub const CONSUME: &str = "consume";
    /// `refersTo: permanentDocument` found by its `$id` from a property: the
    /// typed array of identifiers, on the referenced document, the value must
    /// be an element of. Meta-schema v3+ (protocol version 14).
    pub const IN_LIST: &str = "inList";
    /// Keywords a protocol version 14 beta spelled a document reference with,
    /// refused on every parse so a contract written with them never loads with
    /// another meaning: `lookup` ([`FIND_BY`]), `propertyAgreement`
    /// ([`WHERE`], keyed the other way) and the `listElement` type (a
    /// `permanentDocument` with [`IN_LIST`]).
    pub const REPLACED_LOOKUP: &str = "lookup";
    pub const REPLACED_PROPERTY_AGREEMENT: &str = "propertyAgreement";
    pub const REPLACED_LIST_ELEMENT: &str = "listElement";
    pub const CONTRACT_REQUIREMENTS: &str = "contractRequirements";
    pub const MODERATION: &str = "moderation";
    pub const MINIMUM_AGE_SECONDS: &str = "minimumAgeSeconds";
    pub const MINIMUM_SECONDS_SINCE_UPDATE: &str = "minimumSecondsSinceUpdate";
    pub const OWNER: &str = "owner";
    pub const READONLY: &str = "readonly";
    pub const KEEPS_HISTORY: &str = "keepsHistory";
    pub const OWNER_PROTECTED: &str = "ownerProtected";
    /// Property-level object on a byte array declaring how its ciphertext was
    /// produced: the [`RECIPIENT`], the [`RECIPIENT_KEY`] and [`SENDER_KEY`]
    /// properties carrying the key ids, and the [`SCHEME`]. Meta-schema v3+
    /// (protocol version 14). See `apply_encrypted_for` in `try_from_schema`.
    pub const ENCRYPTED_FOR: &str = "encryptedFor";
    /// `encryptedFor`: the identifier property naming the recipient identity,
    /// or `$ownerId` for the writer's own.
    pub const RECIPIENT: &str = "recipient";
    /// `encryptedFor`: the integer property carrying the recipient's key id.
    pub const RECIPIENT_KEY: &str = "recipientKey";
    /// `encryptedFor`: the integer property carrying the sender's key id.
    pub const SENDER_KEY: &str = "senderKey";
    /// `encryptedFor`: the scheme name, one of `EncryptionScheme::ALL`.
    pub const SCHEME: &str = "scheme";
    /// Property-level integer on a string property, or on the `items` of a
    /// typed array of strings: the most UTF-8 bytes a value may hold.
    /// Meta-schema v3+ (protocol version 14). See `apply_max_bytes` in
    /// `try_from_schema`.
    pub const MAX_BYTES: &str = "maxBytes";
    /// Property-level object on a string property: the [`FUNCTION`] the platform
    /// generates the value with and its [`PARAMS`], other properties of the same
    /// document. Meta-schema v3+ (protocol version 14). See
    /// `apply_generated_from` in `try_from_schema`.
    pub const GENERATED_FROM: &str = "generatedFrom";
    /// `generatedFrom`: the function name, one of `SystemFunction::ALL`.
    pub const FUNCTION: &str = "function";
    /// `generatedFrom`: the parameters, dotted paths of properties of the same
    /// document type.
    pub const PARAMS: &str = "params";
    pub const KEY_REQUIREMENTS: &str = "keyRequirements";
    pub const PURPOSE: &str = "purpose";
    pub const BOUND_TO: &str = "boundTo";
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
    /// Doctype-level flag declaring an **indexOnly** document type: documents
    /// are never written to primary storage — the index entries are the rows,
    /// each terminating in an `Item` keyed by the index's `terminal` property
    /// instead of a `Reference` keyed by the document id. Only what is in the
    /// indexes exists and is recoverable. Meta-schema v3+ (protocol version
    /// 14). See `apply_index_only` in `try_from_schema::common` for the
    /// structural constraints the flag imposes.
    pub const INDEX_ONLY: &str = "indexOnly";
    /// Doctype-level list, on an `indexOnly` type, of the top-level properties
    /// stored in every entry's value (after the row commitment) instead of in
    /// a key: the type's value slot. Listed properties must be required, must
    /// not appear in any index (as a property or a terminal component) and
    /// must be bounded; they are recovered by decoding the proved element.
    /// Meta-schema v3+ (protocol version 14). See `apply_index_only` in
    /// `try_from_schema::common`.
    pub const ENTRY_PAYLOAD: &str = "entryPayload";
    /// Doctype-level object saying what the contract's moderators (its owner and the
    /// identities its moderation config appoints, or its seated team) may do to documents of
    /// this type with a `ContractUserModeration` transition, whatever `canBeDeleted` and
    /// `documentsMutable` say about the documents' own owners: delete them (`delete`, within
    /// `deleteWithin` seconds of their last modification when given, leaving a removal record
    /// unless `deleteKeepsRecord` is false, keeping in it the values `deleteKeepsFields` lists,
    /// and refunding the owner when `deleteRefundsOwner` is true), and write the fields
    /// `changeFields` lists, which nobody else writes. Meta-schema v3+ (protocol version
    /// 14). See [`moderator_abilities`] for its keys, and `apply_moderator_abilities` in
    /// `try_from_schema::common` for what each requires of the type and of the contract.
    pub const MODERATOR_ABILITIES: &str = "moderatorAbilities";

    /// The keys of the `moderatorAbilities` object.
    /// The keys of a conditional [`IMMUTABLE`] entry.
    pub mod immutable_entry {
        /// The top-level property the entry freezes.
        pub const PROPERTY: &str = "property";
        /// The condition under which it is frozen, in the grammar of a
        /// `propertyConstraints` rule, judged on the document a replace
        /// writes, with the stored one read through `$old.`.
        pub const WHEN: &str = "when";
    }

    pub mod moderator_abilities {
        /// When true, the moderators may delete documents of the type, leaving a removal
        /// record under the contract.
        pub const DELETE: &str = "delete";
        /// For how many seconds after a document's last modification (`$updatedAt`, or
        /// `$createdAt` on a type whose documents never change and carry no `$updatedAt`) the
        /// moderators may still delete it. Past that the document is settled and no moderator
        /// can remove it; a replace moves `$updatedAt` and opens the window again. Absent
        /// means no limit. Needs `delete: true`.
        pub const DELETE_WITHIN: &str = "deleteWithin";
        /// Whether a moderator's deletion leaves a removal record under the contract, which is
        /// what a restore brings the document back from. Default `true`. Needs `delete: true`.
        pub const DELETE_KEEPS_RECORD: &str = "deleteKeepsRecord";
        /// Whether the owner of a document a moderator deletes is refunded its storage, as an
        /// owner deleting it themselves is. Default `false`: the owner forfeits it. Needs
        /// `delete: true`.
        pub const DELETE_REFUNDS_OWNER: &str = "deleteRefundsOwner";
        /// Who must approve a moderator's deletion of a document once it is settled, past
        /// `deleteWithin`: an object with `leader` (whether the seated team's leader must be
        /// among the approvals, default `false`), `approvals` (how many members of the team
        /// must approve, the leader counted, default 1) and `approversPredateDocument` (whether
        /// a member the leader added approves only when added before the document was created,
        /// default `true` when more than one approval is needed). See [`delete_settled`] for
        /// the keys. Needs `deleteWithin` and a contract whose moderators are an elected team.
        pub const DELETE_SETTLED: &str = "deleteSettled";
        /// The property paths whose values a moderator's removal record keeps, copied from
        /// the document as it was deleted: what of it stays public once it is gone (the
        /// hashtag of a removed post, say). Any declared, stored property at any depth, and
        /// the timestamps and block heights the type requires. Needs `delete: true` and a
        /// record (`deleteKeepsRecord` not false).
        pub const DELETE_KEEPS_FIELDS: &str = "deleteKeepsFields";
        /// The top-level properties only the moderators write: a document's owner can
        /// neither set them when creating it nor change them when replacing it, unless the
        /// owner moderates the contract.
        pub const CHANGE_FIELDS: &str = "changeFields";

        /// The keys of the `deleteSettled` object.
        pub mod delete_settled {
            /// Whether the seated team's leader must be among the approvals.
            pub const LEADER: &str = "leader";
            /// How many members of the seated team must approve, the leader counted.
            pub const APPROVALS: &str = "approvals";
            /// Whether a member the leader added approves only when its addition was made
            /// before the document was created. Default `true` when `approvals` is above 1,
            /// `false` otherwise; needs `$createdAt` in the type's `required` at registration
            /// while on.
            pub const APPROVERS_PREDATE_DOCUMENT: &str = "approversPredateDocument";
        }
    }
    /// Doctype-level time to live, in seconds: the platform deletes each document of the
    /// type once `$createdAt` plus this many seconds has passed, whoever owns it and
    /// whatever `canBeDeleted` says; from then on it can no longer be changed or restored by a
    /// moderator. Its documents are stored without storage flags, pay
    /// for the time they live instead of perpetual storage, and refund nothing. Requires
    /// `$createdAt` in `required`; refused with `documentsKeepHistory`, `indexOnly` and a
    /// contested index, and fixed when the document type is created. Meta-schema v3+
    /// (protocol version 14). See `apply_documents_ttl` in `try_from_schema::common`.
    pub const TTL: &str = "ttl";
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

impl DocumentTypeBasicMethods for DocumentType {
    fn has_moderator_changeable_fields(&self) -> bool {
        !self.moderator_changeable_fields().is_empty()
    }

    fn derived_index_property_type(&self, name: &str) -> Option<&DocumentPropertyType> {
        match self {
            DocumentType::V0(_) | DocumentType::V1(_) => None,
            DocumentType::V2(v2) => v2.derived_index_property_type(name),
        }
    }
}

impl DocumentTypeBasicMethods for DocumentTypeRef<'_> {
    fn has_moderator_changeable_fields(&self) -> bool {
        !self.moderator_changeable_fields().is_empty()
    }

    fn derived_index_property_type(&self, name: &str) -> Option<&DocumentPropertyType> {
        match self {
            DocumentTypeRef::V0(_) | DocumentTypeRef::V1(_) => None,
            DocumentTypeRef::V2(v2) => v2.derived_index_property_type(name),
        }
    }
}

impl DocumentTypeV0Methods for DocumentType {}

impl DocumentTypeV0Methods for DocumentTypeRef<'_> {}
