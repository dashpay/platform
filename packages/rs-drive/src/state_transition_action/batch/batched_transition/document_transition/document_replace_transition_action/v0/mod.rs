pub mod transformer;

use dpp::data_contract::document_type::property_constraints::AggregateRead;
use dpp::document::{Document, DocumentV0};
use dpp::identity::TimestampMillis;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::{BlockHeight, CoreBlockHeight, Revision};
use dpp::ProtocolError;

use std::collections::{BTreeMap, BTreeSet};

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;

/// document replace transition action v0
#[derive(Debug, Clone)]
pub struct DocumentReplaceTransitionActionV0 {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction,
    /// The current revision we are setting
    pub revision: Revision,
    /// The time the document was last updated
    pub created_at: Option<TimestampMillis>,
    /// The time the document was last updated
    pub updated_at: Option<TimestampMillis>,
    /// The time the document was last transferred
    pub transferred_at: Option<TimestampMillis>,
    /// The block height at which the document was created
    pub created_at_block_height: Option<BlockHeight>,
    /// The block height at which the document was last updated
    pub updated_at_block_height: Option<BlockHeight>,
    /// The block height at which the document was last transferred
    pub transferred_at_block_height: Option<BlockHeight>,
    /// The core block height at which the document was created
    pub created_at_core_block_height: Option<CoreBlockHeight>,
    /// The core block height at which the document was last updated
    pub updated_at_core_block_height: Option<CoreBlockHeight>,
    /// The core block height at which the document was last transferred
    pub transferred_at_core_block_height: Option<CoreBlockHeight>,
    /// Document properties
    pub data: BTreeMap<String, Value>,
    /// Updated fields
    pub changed_data_fields: BTreeSet<String>,
    /// The identifier each REMOVED top-level property held in the stored
    /// document (removed properties that held anything else are absent).
    /// Read by the immutable-property check, which lets an `immutable`
    /// `deletableDocument` reference be cleared once the document it
    /// pointed at is gone: the stored value is the only record of what
    /// that was.
    pub removed_identifier_fields: BTreeMap<String, Identifier>,
    /// The stored value of each `changed_data_fields` property the stored
    /// document held (the changed and the removed ones, not the added ones).
    /// Read by the reference validation, which re-validates only the
    /// elements of a changed typed array of references that the stored list
    /// did not already hold, as an unchanged single reference is not
    /// re-validated either; and by the immutable-property check, which
    /// rebuilds the stored document's properties from `data` and these for a
    /// condition reading them through `$old.`.
    pub stored_changed_values: BTreeMap<String, Value>,
    /// Creator id
    pub creator_id: Option<Identifier>,
    /// When a moderator of the contract last wrote the fields the document type keeps for
    /// its moderators: the stored document's, carried over, or the replace's block time when
    /// its owner moderates the contract and changes one of them
    pub moderated_at: Option<TimestampMillis>,
    /// The moderator who last wrote those fields: the stored document's, or the owner's, as
    /// `moderated_at`
    pub moderated_by: Option<Identifier>,
    /// The `countOf` and `sumOf` totals the document type's `propertyConstraints` rules
    /// read, each as it will be once this write is done, read from state when the action is
    /// built; `None` when the rules judging the write read none, and boxed, since only
    /// such a write holds any and the action is one variant of a large enum.
    pub property_constraint_aggregates: Option<Box<BTreeMap<AggregateRead, i128>>>,
}

/// document replace transition action accessors v0
pub trait DocumentReplaceTransitionActionAccessorsV0 {
    /// base
    fn base(&self) -> &DocumentBaseTransitionAction;
    /// base owned
    fn base_owned(self) -> DocumentBaseTransitionAction;
    /// revision
    fn revision(&self) -> Revision;
    /// created at
    fn created_at(&self) -> Option<TimestampMillis>;
    /// updated at
    fn updated_at(&self) -> Option<TimestampMillis>;
    /// transferred at
    fn transferred_at(&self) -> Option<TimestampMillis>;
    /// Returns the block height at which the document was created.
    fn created_at_block_height(&self) -> Option<BlockHeight>;

    /// Returns the block height at which the document was last updated.
    fn updated_at_block_height(&self) -> Option<BlockHeight>;

    /// Returns the block height at which the document was last transferred.
    fn transferred_at_block_height(&self) -> Option<BlockHeight>;

    /// Returns the core block height at which the document was created.
    fn created_at_core_block_height(&self) -> Option<CoreBlockHeight>;

    /// Returns the core block height at which the document was last updated.
    fn updated_at_core_block_height(&self) -> Option<CoreBlockHeight>;

    /// Returns the core block height at which the document was last transferred.
    fn transferred_at_core_block_height(&self) -> Option<CoreBlockHeight>;

    /// data
    fn data(&self) -> &BTreeMap<String, Value>;

    /// The fields that have changed
    fn changed_data_fields(&self) -> &BTreeSet<String>;
    /// The identifier each removed top-level property held in the stored
    /// document
    fn removed_identifier_fields(&self) -> &BTreeMap<String, Identifier>;
    /// The stored value of each changed property the stored document held
    fn stored_changed_values(&self) -> &BTreeMap<String, Value>;
    /// data owned
    fn data_owned(self) -> BTreeMap<String, Value>;

    /// creator id
    fn creator_id(&self) -> Option<Identifier>;

    /// When a moderator last wrote the document's moderator fields
    fn moderated_at(&self) -> Option<TimestampMillis>;

    /// The moderator who last wrote the document's moderator fields
    fn moderated_by(&self) -> Option<Identifier>;

    /// Stamps the document as written at `moderated_at` by `moderator`, a moderator of the
    /// contract whose replace changes fields only moderators write
    fn set_moderated(&mut self, moderated_at: TimestampMillis, moderator: Identifier);

    /// The `countOf` and `sumOf` totals the rules judging this write read, each as it will
    /// be once the write is done
    fn property_constraint_aggregates(&self) -> &BTreeMap<AggregateRead, i128>;

    /// Sets the totals the rules judging this write read, once they are read from state
    fn set_property_constraint_aggregates(&mut self, aggregates: BTreeMap<AggregateRead, i128>);
}

/// document from replace transition v0
pub trait DocumentFromReplaceTransitionActionV0 {
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransitionAction` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentReplaceTransitionAction` containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_replace_transition_action_v0(
        value: &DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransitionAction` instance and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A `DocumentReplaceTransitionAction` instance containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_owned_replace_transition_action_v0(
        value: DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromReplaceTransitionActionV0 for Document {
    fn try_from_replace_transition_action_v0(
        value: &DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionActionV0 {
            base,
            revision,
            created_at,
            updated_at,
            transferred_at,
            created_at_block_height,
            updated_at_block_height,
            transferred_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
            transferred_at_core_block_height,
            data,
            creator_id,
            moderated_at,
            moderated_by,
            ..
        } = value;

        let id = base.id();

        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0 {
                contract_version: None,
                id,
                owner_id,
                properties: data.clone(),
                revision: Some(*revision),
                created_at: *created_at,
                updated_at: *updated_at,
                transferred_at: *transferred_at,
                created_at_block_height: *created_at_block_height,
                updated_at_block_height: *updated_at_block_height,
                transferred_at_block_height: *transferred_at_block_height,
                created_at_core_block_height: *created_at_core_block_height,
                updated_at_core_block_height: *updated_at_core_block_height,
                transferred_at_core_block_height: *transferred_at_core_block_height,
                creator_id: *creator_id,
                moderated_at: *moderated_at,
                moderated_by: *moderated_by,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::try_from_replace_transition".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn try_from_owned_replace_transition_action_v0(
        value: DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionActionV0 {
            base,
            revision,
            created_at,
            updated_at,
            transferred_at,
            created_at_block_height,
            updated_at_block_height,
            transferred_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
            transferred_at_core_block_height,
            data,
            creator_id,
            moderated_at,
            moderated_by,
            ..
        } = value;

        let id = base.id();

        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0 {
                contract_version: None,
                id,
                owner_id,
                properties: data,
                revision: Some(revision),
                created_at,
                updated_at,
                transferred_at,
                created_at_block_height,
                updated_at_block_height,
                transferred_at_block_height,
                created_at_core_block_height,
                updated_at_core_block_height,
                transferred_at_core_block_height,
                creator_id,
                moderated_at,
                moderated_by,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::try_from_replace_transition".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
