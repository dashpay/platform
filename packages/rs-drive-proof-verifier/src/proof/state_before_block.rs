//! Verification of the state a state transition's keys held before the block that executed it.
//!
//! A node that was asked for a state transition's proof before the transition's block was
//! finalized answers a wait with two proofs of the same keys: the response's own proof, of the
//! state the transition's block left, and `state_before_block`, of the state the block before
//! it left. Together they show what that block changed.
//!
//! They show the change of the block, not of one transition: another transition of the same
//! block that touched the same keys is part of it.

use super::{proof_query_platform_version, with_contract_lookup, FromProof};
use crate::error::MapGroveDbError;
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error};
use dapi_grpc::platform::v0::wait_for_state_transition_result_response::Version;
use dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, Proof, ResponseMetadata, WaitForStateTransitionResultResponse,
};
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::state_transition::proof_result::{
    StateTransitionProofOutcome, StateTransitionProofResult,
};
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

/// The verified state of a state transition's keys at the block before the one that executed
/// the transition.
#[derive(Debug)]
pub struct StateBeforeBlock {
    /// The state of the keys in that block.
    pub state: StateTransitionProofResult,
    /// The block the state is of: the one right before the transition's block.
    pub metadata: ResponseMetadata,
    /// The proof the state was verified with.
    pub proof: Proof,
}

/// What a wait's response says about the state before the transition's block.
#[derive(Debug)]
pub enum StateBeforeBlockVerification {
    /// The response carries no state before the block: the node was not asked for the
    /// transition's proof before its block was finalized. The response's own proof is then of
    /// whatever block the node had reached when it answered.
    NotInResponse,
    /// The response carries a proof of the state before the block, and this kind of state
    /// transition has no reader for it yet. Its proof is checked against what the transition
    /// leaves, which the state before the transition does not hold. Nothing of that proof was
    /// verified.
    NotReadable,
    /// The state before the block, verified against the block before the transition's.
    Verified(Box<StateBeforeBlock>),
}

/// The verified answer of a wait for a state transition, with the state before the
/// transition's block when the response carries it.
#[derive(Debug)]
pub struct StateTransitionResultAroundBlock {
    /// The state the response's own proof holds, as
    /// [`FromProof`] returns it for [`StateTransitionProofOutcome`]. When
    /// `state_before_block` is [`StateBeforeBlockVerification::Verified`], it is of the block
    /// that executed the transition.
    pub outcome: StateTransitionProofOutcome,
    /// The block the response's own proof is of.
    pub metadata: ResponseMetadata,
    /// The state before the transition's block.
    pub state_before_block: StateBeforeBlockVerification,
}

/// Verifies a wait's response as [`FromProof`] does for [`StateTransitionProofOutcome`], and
/// the state before the transition's block when the response carries it.
///
/// Both proofs are verified against the quorum signature of their own block, and the two
/// blocks must follow one another, so a verified state before the block is the state the
/// block before the transition's left.
///
/// The state before the block is read for the kinds of state transition whose proof is a
/// snapshot of the keys they affect (credit transfers, top-ups and withdrawals, address funds
/// movements, shields, token operations without history). For the others the answer is
/// [`StateBeforeBlockVerification::NotReadable`].
pub fn verify_state_transition_result_around_block<I, O>(
    request: I,
    response: O,
    network: Network,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<StateTransitionResultAroundBlock, Error>
where
    I: Into<BroadcastStateTransitionRequest>,
    O: Into<WaitForStateTransitionResultResponse>,
{
    let request: BroadcastStateTransitionRequest = request.into();
    let response: WaitForStateTransitionResultResponse = response.into();

    // What can be refused without verifying a proof is refused first
    let before = match &response.version {
        Some(Version::V0(v0)) => v0.state_before_block.clone(),
        None => return Err(Error::EmptyVersion),
    };
    let before = match before {
        Some(before) => {
            let proof = before.proof.ok_or(Error::NoProofInResult)?;
            let metadata = before.metadata.ok_or(Error::EmptyResponseMetadata)?;
            let block_metadata = response_metadata(&response)?;
            if metadata.height.checked_add(1) != Some(block_metadata.height) {
                return Err(Error::ResponseDecodeError {
                    error: format!(
                        "the state before the block is of block {}, which is not the block \
                         before block {}, the one the response's proof is of",
                        metadata.height, block_metadata.height
                    ),
                });
            }
            Some((proof, metadata))
        }
        None => None,
    };

    let state_transition = StateTransition::deserialize_from_bytes_untrusted(
        &request.state_transition,
    )
    .map_err(|e| Error::ProtocolError {
        error: e.to_string(),
    })?;

    let (outcome, metadata, _) = <StateTransitionProofOutcome as FromProof<
        BroadcastStateTransitionRequest,
    >>::maybe_from_proof_with_metadata(
        request, response, network, platform_version, provider
    )?;
    let outcome = outcome.ok_or(Error::NotFound)?;

    let state_before_block = match before {
        None => StateBeforeBlockVerification::NotInResponse,
        Some((proof, before_metadata)) => verify_state_before_block(
            &state_transition,
            proof,
            before_metadata,
            platform_version,
            provider,
        )?,
    };

    Ok(StateTransitionResultAroundBlock {
        outcome,
        metadata,
        state_before_block,
    })
}

/// The metadata of the response's own proof.
fn response_metadata(
    response: &WaitForStateTransitionResultResponse,
) -> Result<&ResponseMetadata, Error> {
    match &response.version {
        Some(Version::V0(v0)) => v0.metadata.as_ref().ok_or(Error::EmptyResponseMetadata),
        None => Err(Error::EmptyVersion),
    }
}

/// Verifies the proof of the state before the transition's block against the signature of the
/// block it names. The caller has checked that block is the one before the transition's.
fn verify_state_before_block(
    state_transition: &StateTransition,
    proof: Proof,
    metadata: ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<StateBeforeBlockVerification, Error> {
    let binds_execution =
        with_contract_lookup(provider, platform_version, |contracts_provider_fn| {
            Drive::state_transition_proof_binds_execution(
                state_transition,
                contracts_provider_fn,
                platform_version,
            )
            .map_drive_error(&proof, &metadata)
        })?;
    if binds_execution {
        return Ok(StateBeforeBlockVerification::NotReadable);
    }

    let block_info = BlockInfo {
        time_ms: metadata.time_ms,
        height: metadata.height,
        core_height: metadata.core_chain_locked_height,
        // As for the response's own proof: the epoch is not part of the signed state ID
        epoch: Default::default(),
    };
    let query_platform_version = proof_query_platform_version(&metadata)?;

    let (root_hash, outcome) =
        with_contract_lookup(provider, platform_version, |contracts_provider_fn| {
            Drive::verify_state_transition_was_executed_with_proof_for_query_version(
                state_transition,
                &block_info,
                supported_grovedb_proof_bytes(&proof)?,
                contracts_provider_fn,
                query_platform_version,
                platform_version,
            )
            .map_drive_error(&proof, &metadata)
        })?;

    verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

    Ok(StateBeforeBlockVerification::Verified(Box::new(
        StateBeforeBlock {
            state: outcome.into_result(),
            metadata,
            proof,
        },
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result as V0Result;
    use dapi_grpc::platform::v0::wait_for_state_transition_result_response::{
        StateBeforeBlock as StateBeforeBlockInResponse, WaitForStateTransitionResultResponseV0,
    };
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::DataContract;
    use dpp::prelude::{CoreBlockHeight, Identifier};
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use std::sync::Arc;

    /// A provider no test here may reach: each one is refused, or answered, before a proof is
    /// verified.
    struct UnreachableContextProvider;

    impl ContextProvider for UnreachableContextProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            panic!("the context provider must not be reached")
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<dpp::data_contract::TokenConfiguration>, ContextProviderError> {
            panic!("the context provider must not be reached")
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            panic!("the context provider must not be reached")
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            panic!("the context provider must not be reached")
        }
    }

    fn credit_transfer() -> BroadcastStateTransitionRequest {
        let state_transition: StateTransition =
            dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition::V0(
                IdentityCreditTransferTransitionV0::default(),
            )
            .into();
        BroadcastStateTransitionRequest {
            state_transition: state_transition
                .serialize_to_bytes()
                .expect("expected to serialize"),
        }
    }

    fn metadata_at(height: u64) -> ResponseMetadata {
        ResponseMetadata {
            height,
            protocol_version: PlatformVersion::latest().protocol_version,
            ..Default::default()
        }
    }

    fn response(
        height: u64,
        state_before_block: Option<StateBeforeBlockInResponse>,
    ) -> WaitForStateTransitionResultResponse {
        WaitForStateTransitionResultResponse {
            version: Some(Version::V0(WaitForStateTransitionResultResponseV0 {
                result: Some(V0Result::Proof(Proof::default())),
                metadata: Some(metadata_at(height)),
                state_before_block,
            })),
        }
    }

    fn state_before(height: u64) -> StateBeforeBlockInResponse {
        StateBeforeBlockInResponse {
            proof: Some(Proof::default()),
            metadata: Some(metadata_at(height)),
        }
    }

    fn verify(
        request: BroadcastStateTransitionRequest,
        response: WaitForStateTransitionResultResponse,
    ) -> Result<StateTransitionResultAroundBlock, Error> {
        verify_state_transition_result_around_block(
            request,
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableContextProvider,
        )
    }

    #[test]
    fn should_refuse_a_state_before_block_without_a_proof_or_metadata() {
        let mut no_proof = state_before(9);
        no_proof.proof = None;
        let mut no_metadata = state_before(9);
        no_metadata.metadata = None;

        assert!(matches!(
            verify(credit_transfer(), response(10, Some(no_proof))),
            Err(Error::NoProofInResult)
        ));
        assert!(matches!(
            verify(credit_transfer(), response(10, Some(no_metadata))),
            Err(Error::EmptyResponseMetadata)
        ));
    }

    #[test]
    fn should_refuse_a_state_before_block_that_is_not_of_the_block_before() {
        // The same block, the block two before, a later block, and a height that has no next
        for before_height in [10, 8, 11, u64::MAX] {
            let result = verify(
                credit_transfer(),
                response(10, Some(state_before(before_height))),
            );

            assert!(
                matches!(result, Err(Error::ResponseDecodeError { .. })),
                "block {before_height}: {result:?}"
            );
        }
    }

    #[test]
    fn should_refuse_a_response_without_metadata_when_it_carries_a_state_before_block() {
        let mut response = response(10, Some(state_before(9)));
        let Some(Version::V0(v0)) = &mut response.version else {
            unreachable!()
        };
        v0.metadata = None;

        assert!(matches!(
            verify(credit_transfer(), response),
            Err(Error::EmptyResponseMetadata)
        ));
    }

    #[test]
    fn should_not_read_the_state_before_block_of_a_transition_whose_proof_binds_its_execution() {
        // An identity create's proof is checked against the identity it creates, which the
        // state before its block does not hold
        let state_transition: StateTransition =
            dpp::state_transition::identity_create_transition::IdentityCreateTransition::V0(
                IdentityCreateTransitionV0 {
                    identity_id: Identifier::from([7; 32]),
                    ..Default::default()
                },
            )
            .into();

        let verification = verify_state_before_block(
            &state_transition,
            Proof::default(),
            metadata_at(9),
            PlatformVersion::latest(),
            &UnreachableContextProvider,
        )
        .expect("expected an answer without verifying a proof");

        assert!(matches!(
            verification,
            StateBeforeBlockVerification::NotReadable
        ));
    }
}
