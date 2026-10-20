use std::collections::BTreeMap;

use super::broadcast::{convert_proof_result, BroadcastStateTransition, WaitForOutcome};
use super::put_settings::PutSettings;
use crate::platform::Fetch;
use crate::Error;
use crate::Sdk;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::prelude::{DataContract, Identifier, Identity};
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::StateTransition;
use dpp::state_transition::StateTransitionLike;
use dpp::voting::votes::Vote;
use dpp::ProtocolError;

/// Waitable trait provides a way to wait for a response of a state transition after it has been broadcast and
/// receive altered objects.
///
/// This is a simple convenience trait wrapping the [`BroadcastStateTransition::wait_for_response`] method.
#[async_trait::async_trait]
pub trait Waitable: Sized {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>;
}
#[async_trait::async_trait]
impl Waitable for DataContract {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<DataContract, Error> {
        match &state_transition {
            // A contract update proof authenticates the current contract
            // body — a height-pinned snapshot — and cannot bind this
            // update's execution, so the snapshot wait is required.
            StateTransition::DataContractUpdate(_) => {
                state_transition
                    .wait_for_affected_state(sdk, settings)
                    .await
            }
            _ => state_transition.wait_for_response(sdk, settings).await,
        }
    }
}

#[async_trait::async_trait]
impl Waitable for Document {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error> {
        let doc_id = single_document_id(&state_transition)?;
        let mut documents: BTreeMap<Identifier, Option<Document>> =
            state_transition.wait_for_response(sdk, settings).await?;
        take_document(&mut documents, doc_id)
    }
}

/// Waits for the proof of a document batch and returns the document it left
/// together with the credit balance of the batch's owner after it: from
/// protocol version 14 the proof carries both, read from one state, and the
/// verified outcome hands the balance out; a proof made at an earlier version
/// carries only the document and the balance is `None`. The balance is a
/// snapshot at the proof's block, so it may already include later transitions
/// of the same identity.
///
/// Unlike [`Waitable::wait_for_response`] for a document, this accepts an
/// affected-state outcome too: an indexOnly document type's proof can only
/// attest the resulting entry, not this transition's execution, and its
/// document is then a snapshot rebuilt from the transition.
pub async fn wait_for_document_and_owner_balance(
    sdk: &Sdk,
    state_transition: StateTransition,
    settings: Option<PutSettings>,
) -> Result<(Document, Option<Credits>), Error> {
    let doc_id = single_document_id(&state_transition)?;

    let (outcome, _metadata) = state_transition
        .wait_for_outcome_with_metadata(sdk, settings)
        .await?;
    let owner_balance = outcome.owner_balance();
    let mut documents: BTreeMap<Identifier, Option<Document>> =
        convert_proof_result(outcome.into_result())?;
    take_document(&mut documents, doc_id).map(|document| (document, owner_balance))
}

/// The id of the one document a batch transition modifies.
fn single_document_id(state_transition: &StateTransition) -> Result<Identifier, Error> {
    if let StateTransition::Batch(transition) = state_transition {
        let ids = transition.modified_data_ids();
        if ids.len() != 1 {
            return Err(Error::Protocol(
                dpp::ProtocolError::InvalidStateTransitionType(format!(
                    "expected state transition with exactly one document, got {}",
                    ids.into_iter()
                        .map(|id| id
                            .to_string(dpp::platform_value::string_encoding::Encoding::Base58))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            ));
        }
        Ok(ids[0])
    } else {
        Err(Error::Protocol(ProtocolError::InvalidStateTransitionType(
            format!(
                "expected state transition to be a DocumentsBatchTransition, got {}",
                state_transition.name()
            ),
        )))
    }
}

/// The proved document out of a verified documents result.
fn take_document(
    documents: &mut BTreeMap<Identifier, Option<Document>>,
    doc_id: Identifier,
) -> Result<Document, Error> {
    documents
        .remove(&doc_id)
        .ok_or(Error::InvalidProvedResponse(
            "did not prove the sent document".to_string(),
        ))?
        .ok_or(Error::InvalidProvedResponse(
            "expected there to actually be a document".to_string(),
        ))
}

#[async_trait::async_trait]
impl Waitable for Identity {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error> {
        let result: Result<Self, Error> = state_transition.wait_for_response(sdk, settings).await;

        match result {
            Ok(identity) => Ok(identity),
            // TODO: We need to refactor sdk Error to be able to retrieve gRPC error code and identify conflicts
            Err(Error::AlreadyExists(_)) => {
                let identity_id = if let StateTransition::IdentityCreate(st) = state_transition {
                    st.identity_id()
                } else {
                    return Err(Error::Generic(format!(
                        "expected identity create state transition, got {:?}",
                        state_transition.name()
                    )));
                };

                tracing::debug!(
                    ?identity_id,
                    "attempt to create identity that already exists"
                );
                let identity = Identity::fetch(sdk, identity_id).await?;
                identity.ok_or(Error::Generic(
                    "identity was proved to not exist but was said to exist".to_string(),
                ))
            }
            Err(e) => Err(e),
        }
    }
}

#[async_trait::async_trait]
impl Waitable for Vote {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error> {
        state_transition.wait_for_response(sdk, settings).await
    }
}
