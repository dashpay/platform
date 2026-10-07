use crate::drive::document::derived_index_values::set_derived_index_values;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::util::batch::DriveOperation::{
    DocumentOperation, IdentityOperation, PrefundedSpecializedBalanceOperation,
};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DocumentTypeInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::epoch::Epoch;

use dpp::document::Document;
use dpp::prelude::Identifier;
use std::borrow::Cow;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0, DocumentFromCreateTransitionAction};
use dpp::version::PlatformVersion;
use crate::util::batch::drive_op_batch::{PrefundedSpecializedBalanceOperationType};
use crate::util::object_size_info::DataContractInfo::DataContractFetchInfo;
use crate::error::drive::DriveError;

impl DriveHighLevelBatchOperationConverter for DocumentCreateTransitionAction {
    fn into_high_level_batch_drive_operations<'b>(
        mut self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .document_create_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();

                let contract_fetch_info = self.base().data_contract_fetch_info();

                let document_type_name = self.base().document_type_name().clone();

                let identity_contract_nonce = self.base().identity_contract_nonce();

                let maybe_prefunded_voting_balance = self.take_prefunded_voting_balance();

                let also_insert_vote_poll_stored_info = self.take_should_store_contest_info();

                let document_creation_token_cost_operations =
                    self.base().token_cost_operations(owner_id);

                // In place in generation 0, which every table selects: only the protocol
                // version 14 batch state validation sets consumed documents, and only for a
                // create revealing a commitment through a `refersTo` lookup with a computed key
                // declaring `consume`, which only that version's parser produces. Every earlier
                // create has none and converts to exactly the operations it always did
                let consumed_documents = self.consumed_documents().to_vec();

                // The values of the type's derived index properties (protocol version 14), which
                // the reference validation took from the documents it fetched: Drive keys the
                // document by them instead of reading those documents again. None on every
                // create of an earlier version.
                let derived_index_values = self.derived_index_values().cloned();

                let mut document = Document::try_from_owned_create_transition_action(
                    self,
                    owner_id,
                    platform_version,
                )?;
                if let Some(values) = derived_index_values {
                    set_derived_index_values(&mut document, values);
                }

                let storage_flags =
                    StorageFlags::new_single_epoch(epoch.index, Some(owner_id.to_buffer()));

                let mut ops = vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )];

                ops.extend(document_creation_token_cost_operations);

                if let Some((contested_document_resource_vote_poll, credits)) =
                    maybe_prefunded_voting_balance
                {
                    let prefunded_specialized_balance_id =
                        contested_document_resource_vote_poll.specialized_balance_id()?;
                    // We are in the situation of a contested document
                    // We prefund the voting balances first
                    ops.push(PrefundedSpecializedBalanceOperation(
                        PrefundedSpecializedBalanceOperationType::CreateNewPrefundedBalance {
                            prefunded_specialized_balance_id,
                            add_balance: credits,
                        },
                    ));

                    // We remove from the identity balance an equal amount
                    ops.push(IdentityOperation(
                        IdentityOperationType::RemoveFromIdentityBalance {
                            identity_id: owner_id.into_buffer(),
                            balance_to_remove: credits,
                        },
                    ));

                    // We add the contested document
                    // The contested document resides in a special location in grovedb until a time where the
                    // resolution expires, at that point it either will be moved to
                    ops.push(DocumentOperation(
                        DocumentOperationType::AddContestedDocument {
                            owned_document_info: OwnedDocumentInfo {
                                document_info: DocumentOwnedInfo((
                                    document,
                                    Some(Cow::Owned(storage_flags)),
                                )),
                                owner_id: Some(owner_id.into_buffer()),
                            },
                            contested_document_resource_vote_poll,
                            contract_info: DataContractFetchInfo(contract_fetch_info.clone()),
                            document_type_info: DocumentTypeInfo::DocumentTypeName(
                                document_type_name,
                            ),
                            insert_without_check: false, //todo: consider setting to true
                            also_insert_vote_poll_stored_info,
                        },
                    ));
                    // The commitments the create revealed and consumes, each in the create's
                    // own contract. A contested document sits in its contest's own trees, so
                    // no tree its insert writes into is shared with these deletes. They are
                    // deleted without their owner's `canBeDeleted` guard: the consume rules
                    // admitted them, and a type whose documents only a consume deletes
                    // (`canBeDeleted: "onlyWhenConsumed"`) refuses its owner's delete
                    for consumed in consumed_documents {
                        ops.push(DocumentOperation(
                            DocumentOperationType::ForceDeleteDocument {
                                document_id: consumed.document_id,
                                contract_info: DataContractFetchInfo(contract_fetch_info.clone()),
                                document_type_info: DocumentTypeInfo::DocumentTypeName(
                                    consumed.document_type_name,
                                ),
                            },
                        ));
                    }
                } else if consumed_documents.is_empty() {
                    // Just add the document
                    ops.push(DocumentOperation(DocumentOperationType::AddDocument {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentOwnedInfo((
                                document,
                                Some(Cow::Owned(storage_flags)),
                            )),
                            owner_id: Some(owner_id.into_buffer()),
                        },
                        contract_info: DataContractFetchInfo(contract_fetch_info.clone()),
                        document_type_info: DocumentTypeInfo::DocumentTypeName(document_type_name),
                        override_document: false,
                    }));
                } else {
                    // The document and the commitments the create revealed and consumes, each
                    // in the create's own contract, converted as one so the deletes see the
                    // insert: a consumed document may share an index bucket or an
                    // expirations tree with the new one
                    ops.push(DocumentOperation(
                        DocumentOperationType::AddDocumentAndDeleteConsumed {
                            owned_document_info: OwnedDocumentInfo {
                                document_info: DocumentOwnedInfo((
                                    document,
                                    Some(Cow::Owned(storage_flags)),
                                )),
                                owner_id: Some(owner_id.into_buffer()),
                            },
                            contract_info: DataContractFetchInfo(contract_fetch_info.clone()),
                            document_type_info: DocumentTypeInfo::DocumentTypeName(
                                document_type_name,
                            ),
                            consumed_documents: consumed_documents
                                .into_iter()
                                .map(|consumed| (consumed.document_id, consumed.document_type_name))
                                .collect(),
                        },
                    ));
                }

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DocumentCreateTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
