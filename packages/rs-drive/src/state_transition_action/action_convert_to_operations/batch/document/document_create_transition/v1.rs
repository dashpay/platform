use crate::drive::document::derived_index_values::set_derived_index_values;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
    DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0,
    DocumentFromCreateTransitionAction,
};
use crate::util::batch::drive_op_batch::{
    PrefundedSpecializedBalanceOperationType, VotePollOperationType,
};
use crate::util::batch::DriveOperation::{
    DocumentOperation, IdentityOperation, PrefundedSpecializedBalanceOperation,
    VotePollOperation,
};
use crate::util::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::util::object_size_info::DataContractInfo::DataContractFetchInfo;
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DocumentTypeInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::epoch::Epoch;
use dpp::decision_proposal::DecisionProposalVotePoll;
use dpp::document::Document;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use std::borrow::Cow;

/// The create conversion 1 (protocol version 15): generation 0's operations, but each
/// document the create consumes is deleted from what document reference validation 1 read of
/// it (`ForceDeleteReadDocument`, `AddDocumentAndDeleteReadConsumed`), without reading it
/// again, where generation 0 deletes it by its id and reads it once more.
///
/// A decision proposal (its contract exists from protocol version 15) also opens the yes/no
/// poll it states: the owner's stated fund moves to the poll's prefunded balance, as a
/// contest's does, and the poll is opened after the document. The poll is read from the
/// proposal here, as check_tx's fee estimate converts a create no state validation has seen.
pub(super) fn into_high_level_batch_drive_operations_v1<'b>(
    mut action: DocumentCreateTransitionAction,
    epoch: &Epoch,
    owner_id: Identifier,
    platform_version: &PlatformVersion,
) -> Result<Vec<DriveOperation<'b>>, Error> {
    let data_contract_id = action.base().data_contract_id();

    let contract_fetch_info = action.base().data_contract_fetch_info();

    let document_type_name = action.base().document_type_name().clone();

    let identity_contract_nonce = action.base().identity_contract_nonce();

    let maybe_prefunded_voting_balance = action.take_prefunded_voting_balance();

    let also_insert_vote_poll_stored_info = action.take_should_store_contest_info();

    let document_creation_token_cost_operations = action.base().token_cost_operations(owner_id);

    // The commitments the create consumes, each with what document reference validation 1
    // read of it. Taken, not copied: the document built from the action below does not read
    // them
    let consumed_documents = action.take_consumed_documents();

    // The values of the type's derived index properties (protocol version 14), which
    // the reference validation took from the documents it fetched: Drive keys the
    // document by them instead of reading those documents again. None on every
    // create of an earlier version.
    let derived_index_values = action.derived_index_values().cloned();

    // The poll a decision proposal opens, and the fund it pays in: structure validation 2 held
    // the fund to the decision proposal fund and the rule to the system limits.
    let decision_proposal_opening = action
        .decision_proposal()
        .map(|decision_proposal| {
            let Some((_, fund)) = decision_proposal.stated_fund else {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a decision proposal reaches its conversion with the fund it states",
                )));
            };
            let vote_poll = DecisionProposalVotePoll::from_proposal(
                action.base().id(),
                action.data(),
                platform_version,
            )?;
            if !vote_poll.is_valid() {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a decision proposal reaches its conversion with a rule structure validation accepted",
                )));
            }
            Ok((vote_poll.into_data()?, fund))
        })
        .transpose()?;

    let mut document =
        Document::try_from_owned_create_transition_action(action, owner_id, platform_version)?;
    if let Some(values) = derived_index_values {
        set_derived_index_values(&mut document, values);
    }

    let storage_flags = StorageFlags::new_single_epoch(epoch.index, Some(owner_id.to_buffer()));

    let mut ops = vec![IdentityOperation(
        IdentityOperationType::UpdateIdentityContractNonce {
            identity_id: owner_id.into_buffer(),
            contract_id: data_contract_id.into_buffer(),
            nonce: identity_contract_nonce,
        },
    )];

    ops.extend(document_creation_token_cost_operations);

    if let Some((decision_proposal_vote_poll, fund)) = &decision_proposal_opening {
        // The poll's votes are paid from its prefunded balance, under its id
        ops.push(PrefundedSpecializedBalanceOperation(
            PrefundedSpecializedBalanceOperationType::CreateNewPrefundedBalance {
                prefunded_specialized_balance_id: decision_proposal_vote_poll
                    .vote_poll
                    .specialized_balance_id(platform_version)?,
                add_balance: *fund,
            },
        ));
        // which the proposer pays
        ops.push(IdentityOperation(
            IdentityOperationType::RemoveFromIdentityBalance {
                identity_id: owner_id.into_buffer(),
                balance_to_remove: *fund,
            },
        ));
    }

    if let Some((contested_document_resource_vote_poll, credits)) = maybe_prefunded_voting_balance {
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
                    document_info: DocumentOwnedInfo((document, Some(Cow::Owned(storage_flags)))),
                    owner_id: Some(owner_id.into_buffer()),
                },
                contested_document_resource_vote_poll,
                contract_info: DataContractFetchInfo(contract_fetch_info.clone()),
                document_type_info: DocumentTypeInfo::DocumentTypeName(document_type_name),
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
        // Each from what its lookup read, without reading it again
        for consumed_document in consumed_documents {
            ops.push(DocumentOperation(
                DocumentOperationType::ForceDeleteReadDocument {
                    consumed_document,
                    contract_info: DataContractFetchInfo(contract_fetch_info.clone()),
                },
            ));
        }
    } else if consumed_documents.is_empty() {
        // Just add the document
        ops.push(DocumentOperation(DocumentOperationType::AddDocument {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentOwnedInfo((document, Some(Cow::Owned(storage_flags)))),
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
            DocumentOperationType::AddDocumentAndDeleteReadConsumed {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentOwnedInfo((document, Some(Cow::Owned(storage_flags)))),
                    owner_id: Some(owner_id.into_buffer()),
                },
                contract_info: DataContractFetchInfo(contract_fetch_info.clone()),
                document_type_info: DocumentTypeInfo::DocumentTypeName(document_type_name),
                consumed_documents,
            },
        ));
    }

    if let Some((decision_proposal_vote_poll, _)) = decision_proposal_opening {
        // The poll, with its stored info, its vote trees and its end date entry
        ops.push(VotePollOperation(
            VotePollOperationType::OpenYesNoVotePoll {
                vote_poll: decision_proposal_vote_poll.vote_poll,
                end_date: decision_proposal_vote_poll.end_date,
            },
        ));
    }

    Ok(ops)
}
