use std::collections::BTreeMap;

use super::broadcast::{
    convert_proof_result, require_execution_proved, BroadcastStateTransition, WaitForOutcome,
};
use super::put_settings::PutSettings;
use crate::platform::Fetch;
use crate::Error;
use crate::Sdk;
use dapi_grpc::platform::v0::ResponseMetadata;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::prelude::{DataContract, Identifier, Identity};
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::proof_result::StateTransitionProofOutcome;
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
    wait_for_document_and_owner_balance_with_metadata(sdk, state_transition, settings, false)
        .await
        .map(|(document, owner_balance, _metadata)| (document, owner_balance))
}

/// [`wait_for_document_and_owner_balance`] that also returns the verified
/// metadata of the response: its height is the block the balance is a
/// snapshot at, so a caller that keeps the balance can order it against
/// balances it read elsewhere. With `execution_proof_required` an
/// affected-state outcome is refused, as [`Waitable::wait_for_response`]
/// refuses it for a document.
pub(crate) async fn wait_for_document_and_owner_balance_with_metadata(
    sdk: &Sdk,
    state_transition: StateTransition,
    settings: Option<PutSettings>,
    execution_proof_required: bool,
) -> Result<(Document, Option<Credits>, ResponseMetadata), Error> {
    let doc_id = single_document_id(&state_transition)?;

    let (outcome, metadata) = state_transition
        .wait_for_outcome_with_metadata(sdk, settings)
        .await?;
    document_and_owner_balance(outcome, doc_id, execution_proof_required)
        .map(|(document, owner_balance)| (document, owner_balance, metadata))
}

/// The document a verified batch outcome proves, with the owner's balance the
/// outcome carries. The balance is read before the guarantee is checked:
/// [`require_execution_proved`] keeps only the result.
fn document_and_owner_balance(
    outcome: StateTransitionProofOutcome,
    doc_id: Identifier,
    execution_proof_required: bool,
) -> Result<(Document, Option<Credits>), Error> {
    let owner_balance = outcome.owner_balance();
    let result = if execution_proof_required {
        require_execution_proved(outcome)?
    } else {
        outcome.into_result()
    };
    let mut documents: BTreeMap<Identifier, Option<Document>> = convert_proof_result(result)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::document::DocumentV0;
    use dpp::state_transition::proof_result::StateTransitionProofResult;

    const OWNER_BALANCE: Credits = 2_743_797_100;

    fn document_id() -> Identifier {
        Identifier::from([7u8; 32])
    }

    fn document() -> Document {
        Document::V0(DocumentV0 {
            id: document_id(),
            owner_id: Identifier::from([2u8; 32]),
            ..Default::default()
        })
    }

    fn proved_document(document: Option<Document>) -> StateTransitionProofResult {
        StateTransitionProofResult::VerifiedDocuments(BTreeMap::from([(document_id(), document)]))
    }

    #[test]
    fn should_return_document_and_owner_balance_from_an_execution_proof() {
        let outcome =
            StateTransitionProofOutcome::execution_proved(proved_document(Some(document())))
                .with_owner_balance(Some(OWNER_BALANCE));

        let (proved, owner_balance) = document_and_owner_balance(outcome, document_id(), true)
            .expect("an execution proof of the sent document must be accepted");

        assert_eq!(proved, document());
        assert_eq!(owner_balance, Some(OWNER_BALANCE));
    }

    #[test]
    fn should_return_no_owner_balance_for_a_proof_made_before_protocol_version_14() {
        let outcome =
            StateTransitionProofOutcome::execution_proved(proved_document(Some(document())));

        let (proved, owner_balance) = document_and_owner_balance(outcome, document_id(), true)
            .expect("a document-only proof must still give the document");

        assert_eq!(proved, document());
        assert_eq!(owner_balance, None);
    }

    #[test]
    fn should_reject_an_affected_state_outcome_when_execution_proof_is_required() {
        let outcome =
            StateTransitionProofOutcome::affected_state(proved_document(Some(document())))
                .with_owner_balance(Some(OWNER_BALANCE));

        let error = document_and_owner_balance(outcome, document_id(), true)
            .expect_err("a snapshot must not pass as execution evidence");

        assert!(matches!(error, Error::ExecutionNotProved(_)));
    }

    #[test]
    fn should_accept_an_affected_state_outcome_with_its_owner_balance_when_not_required() {
        let outcome =
            StateTransitionProofOutcome::affected_state(proved_document(Some(document())))
                .with_owner_balance(Some(OWNER_BALANCE));

        let (proved, owner_balance) = document_and_owner_balance(outcome, document_id(), false)
            .expect("an indexOnly document's snapshot must be accepted");

        assert_eq!(proved, document());
        assert_eq!(owner_balance, Some(OWNER_BALANCE));
    }

    #[test]
    fn should_reject_an_outcome_that_does_not_prove_the_sent_document() {
        for result in [
            StateTransitionProofResult::VerifiedDocuments(BTreeMap::new()),
            proved_document(None),
        ] {
            let outcome = StateTransitionProofOutcome::execution_proved(result)
                .with_owner_balance(Some(OWNER_BALANCE));

            let error = document_and_owner_balance(outcome, document_id(), true)
                .expect_err("an outcome without the sent document must be rejected");

            assert!(matches!(error, Error::InvalidProvedResponse(_)));
        }
    }

    #[test]
    fn should_reject_an_outcome_that_is_not_a_documents_result() {
        let outcome = StateTransitionProofOutcome::execution_proved(
            StateTransitionProofResult::VerifiedTokenBalanceAbsence(document_id()),
        );

        let error = document_and_owner_balance(outcome, document_id(), true)
            .expect_err("a non-document result must be rejected");

        assert!(matches!(error, Error::InvalidProvedResponse(_)));
    }
}
