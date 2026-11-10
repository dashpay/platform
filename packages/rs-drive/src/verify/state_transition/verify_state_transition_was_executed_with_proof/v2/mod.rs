use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::ContractLookupFn;
use crate::verify::RootHash;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::proof_result::StateTransitionProofOutcome;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

impl Drive {
    /// Generation 2 of `verify_state_transition_was_executed_with_proof`: generations 0 and
    /// 1, and the execution proof of a delta contract update (version 1 of the update
    /// transition, from protocol version 15) is verified too.
    ///
    /// The proof of a contract update holds the contract as it is after the update, which
    /// an update that embeds its contract is compared with. A delta embeds only what it
    /// changes, so the contract it asks for exists only once it is merged onto the contract
    /// it was built on. The verifier takes that contract from `known_contracts_provider_fn`,
    /// as document verification takes a document's contract, merges the delta onto it with
    /// the merge block execution runs ([`DataContract::apply_update`]), and verifies the
    /// update embedding the result exactly as generations 0 and 1 verify one: the whole
    /// proved contract must be the merged one, with the guarantee and the owner balance of
    /// any contract update.
    ///
    /// - A lookup that does not know the contract, or knows it at another version than the
    ///   one before the delta's, is [`ProofError::MissingContextRequirement`]: the caller
    ///   must provide the contract the delta was built on.
    /// - A delta that names an entry in two sections that cannot both apply, which a node
    ///   refuses, or that does not merge onto that contract, is
    ///   [`ProofError::InvalidTransition`].
    /// - A proved contract that is not the merged one is [`ProofError::IncorrectProof`], as
    ///   for an update that embeds its contract.
    ///
    /// [`DataContract::apply_update`]: dpp::data_contract::DataContract::apply_update
    pub(super) fn verify_state_transition_was_executed_with_proof_v2(
        state_transition: &StateTransition,
        block_info: &BlockInfo,
        proof: &[u8],
        known_contracts_provider_fn: &ContractLookupFn,
        carries_owner_balance: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StateTransitionProofOutcome), Error> {
        let StateTransition::DataContractUpdate(DataContractUpdateTransition::V1(delta)) =
            state_transition
        else {
            return Drive::verify_state_transition_was_executed_with_proof_v0(
                state_transition,
                block_info,
                proof,
                known_contracts_provider_fn,
                carries_owner_balance,
                platform_version,
            );
        };

        if let Some(overlap) = delta.overlapping_entry() {
            return Err(Error::Proof(ProofError::InvalidTransition(
                overlap.to_string(),
            )));
        }

        let contract_id = delta.data_contract_id;
        let known_contract = known_contracts_provider_fn(&contract_id)?.ok_or_else(|| {
            Error::Proof(ProofError::MissingContextRequirement(format!(
                "the contract {} as it was before the update is needed to verify the proof of a delta contract update",
                contract_id
            )))
        })?;
        if known_contract.version().checked_add(1) != Some(delta.version) {
            return Err(Error::Proof(ProofError::MissingContextRequirement(format!(
                "the contract {} is known at version {}, and the delta contract update that produces version {} is verified against the version before it",
                contract_id,
                known_contract.version(),
                delta.version
            ))));
        }

        let merged = delta.to_full_contract_update(&known_contract, platform_version)?;
        if let Some(error) = merged.errors.first() {
            return Err(Error::Proof(ProofError::InvalidTransition(format!(
                "the delta contract update does not apply to contract {} at version {}: {}",
                contract_id,
                known_contract.version(),
                error
            ))));
        }
        let full_contract_update: StateTransition = merged.into_data()?.into();

        Drive::verify_state_transition_was_executed_with_proof_v0(
            &full_contract_update,
            block_info,
            proof,
            known_contracts_provider_fn,
            carries_owner_balance,
            platform_version,
        )
    }
}
