mod v0;
mod v1;

use crate::drive::Drive;
use crate::error::Error;
use std::borrow::Cow;

use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;

use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::prelude::{BlockHeight, CoreBlockHeight, TimestampMillis};

use derive_more::From;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, From)]
pub(in crate::drive::document::index_uniqueness) enum UniquenessOfDataRequest<'a, 'b> {
    V0(UniquenessOfDataRequestV0<'a>),
    V1(UniquenessOfDataRequestV1<'a, 'b>),
}

/// Represents a request to determine the uniqueness of data.
/// This structure is defined to handle index uniqueness within a document.
/// The purpose is to encapsulate all the required parameters to determine
/// if a particular data is unique or not.
///
/// **Note**: Modifications to this structure are discouraged due to its close coupling
/// with index uniqueness methods. Any change here might necessitate changes across
/// all those methods. Given the likely infrequent need for changes, this design choice
/// is deemed acceptable.
#[derive(Debug)]
pub(in crate::drive::document::index_uniqueness) struct UniquenessOfDataRequestV0<'a> {
    /// Reference to the associated data contract.
    pub contract: &'a DataContract,
    /// Reference of the document type.
    pub document_type: DocumentTypeRef<'a>,
    /// The ID representing the owner.
    pub owner_id: Identifier,
    /// The ID of the document in question.
    pub document_id: Identifier,
    /// A flag indicating if the original (existing) document is considered permissible.
    pub allow_original: bool,
    /// Optional timestamp indicating when the document was created.
    pub created_at: Option<TimestampMillis>,
    /// Optional timestamp indicating the last time the document was updated.
    pub updated_at: Option<TimestampMillis>,
    /// Optional timestamp indicating the last time the document was transferred.
    pub transferred_at: Option<TimestampMillis>,
    /// Optional timestamp indicating the block height at which the document was created.
    pub created_at_block_height: Option<BlockHeight>,
    /// Optional timestamp indicating the last block height the document was updated.
    pub updated_at_block_height: Option<BlockHeight>,
    /// Optional timestamp indicating the last block height the document was transferred.
    pub transferred_at_block_height: Option<BlockHeight>,
    /// Optional timestamp indicating the core height at which the document was created.
    pub created_at_core_block_height: Option<CoreBlockHeight>,
    /// Optional timestamp indicating the last core block height the document was updated.
    pub updated_at_core_block_height: Option<CoreBlockHeight>,
    /// Optional timestamp indicating the last core block height the document was transferred.
    pub transferred_at_core_block_height: Option<CoreBlockHeight>,
    /// The actual data to be checked for uniqueness, represented as a mapping.
    pub data: &'a BTreeMap<String, Value>,
}

/// A request object used when verifying the uniqueness of document data
/// within the platform. It bundles together references to the data contract,
/// document type, identifying fields, timestamps, and the actual data map
/// being checked for uniqueness constraints.
///
/// This structure is versioned (`V1`) to allow evolution of the
/// uniqueness-checking logic without breaking existing code.
#[derive(Debug)]
pub(in crate::drive::document::index_uniqueness) struct UniquenessOfDataRequestV1<'a, 'b> {
    /// Reference to the data contract that defines the schema
    /// and uniqueness rules for this document.
    pub contract: &'a DataContract,

    /// Reference to the document type within the contract
    /// that this uniqueness check applies to.
    pub document_type: DocumentTypeRef<'a>,

    /// Identifier of the current owner of the document.
    pub owner_id: Identifier,

    /// Identifier of the original creator of the document.
    /// This may differ from `owner_id` if ownership has changed.
    pub creator_id: Option<Identifier>,

    /// Identifier of the document whose data is being checked.
    pub document_id: Identifier,

    /// Timestamp (in milliseconds) when the document was first created.
    pub created_at: Option<TimestampMillis>,

    /// Timestamp (in milliseconds) when the document was last updated.
    pub updated_at: Option<TimestampMillis>,

    /// Timestamp (in milliseconds) when the document was last transferred
    /// (ownership change event).
    pub transferred_at: Option<TimestampMillis>,

    /// Block height at which the document was originally created.
    pub created_at_block_height: Option<BlockHeight>,

    /// Block height at which the document was last updated.
    pub updated_at_block_height: Option<BlockHeight>,

    /// Block height at which the document was last transferred.
    pub transferred_at_block_height: Option<BlockHeight>,

    /// Core chain block height at which the document was created.
    pub created_at_core_block_height: Option<CoreBlockHeight>,

    /// Core chain block height at which the document was last updated.
    pub updated_at_core_block_height: Option<CoreBlockHeight>,

    /// Core chain block height at which the document was last transferred.
    pub transferred_at_core_block_height: Option<CoreBlockHeight>,

    /// The map of field names to values representing the document’s data.
    /// This is the primary content to be checked against uniqueness indexes.
    pub data: &'a BTreeMap<String, Value>,

    /// The type of uniqueness of data request
    pub update_type: UniquenessOfDataRequestUpdateType<'b>,
}

#[derive(Debug)]
pub enum UniquenessOfDataRequestUpdateType<'a> {
    /// It's a new document, all unique index couples should not yet exist in the state
    NewDocument,
    /// It's a changed document, unique index couples should already exist in the state for series
    /// of values that have not changed.
    /// For example if you have the owner_id and a value like car_type that has a unique index, and
    /// you have another unique index owner_id and updated_at. The document changes a value of car
    /// color and updated at. Since the car type does not change, we expect that in the state we
    /// will still have the couple <owner_id, car_type> however we should not have
    /// <owner_id, new updated_at>. Knowing what changed allows us to know in advance if there
    /// will be an issue with the unique indexes.
    ChangedDocument {
        /// Indicates whether the `owner_id` field was modified
        /// in this update operation.
        changed_owner_id: bool,
        /// Indicates whether the `updated_at` field was modified.
        changed_updated_at: bool,
        /// Indicates whether the `transferred_at` field was modified.
        changed_transferred_at: bool,
        /// Indicates whether the `updated_at_block_height` field was modified.
        changed_updated_at_block_height: bool,
        /// Indicates whether the `transferred_at_block_height` field was modified.
        changed_transferred_at_block_height: bool,
        /// Indicates whether the `updated_at_core_block_height` field was modified.
        changed_updated_at_core_block_height: bool,
        /// Indicates whether the `transferred_at_core_block_height` field was modified.
        changed_transferred_at_core_block_height: bool,
        /// A list of keys in the `data` map whose values have changed,
        /// used to limit uniqueness checks only to modified fields.
        changed_data_values: Cow<'a, BTreeSet<String>>,
    },
}

impl Drive {
    /// Internal method validating uniqueness
    ///
    /// # Arguments
    ///
    /// * `request` - A `UniquenessOfDataRequest` object representing the request.
    /// * `transaction` - A `TransactionArg` object representing the transaction.
    /// * `drive_version` - A `DriveVersion` object representing the version of the Drive.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - If successful, returns a `SimpleConsensusValidationResult` object representing the result of the validation.
    ///   If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of the Drive is unknown.
    pub(in crate::drive::document::index_uniqueness) fn validate_uniqueness_of_data(
        &self,
        request: UniquenessOfDataRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match request {
            UniquenessOfDataRequest::V0(v0) => {
                self.validate_uniqueness_of_data_v0(v0, transaction, platform_version)
            }
            UniquenessOfDataRequest::V1(v1) => {
                self.validate_uniqueness_of_data_v1(v1, transaction, platform_version)
            }
        }
    }
}
