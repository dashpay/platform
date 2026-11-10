use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::prove::prove_state_transition::ProofCreationResult;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Version 2: version 1, and a delta contract update (version 1 of the update
    /// transition, from protocol version 15) is proved too. Its proof is the one of an
    /// update that embeds its contract, the contract and its owner's credit balance; as a
    /// delta embeds no contract, whether the contract keeps history, which selects where
    /// the contract is proved, is read from the stored contract. Every other transition
    /// proves as in version 1, in the same layout.
    pub(super) fn prove_state_transition_v2(
        &self,
        state_transition: &StateTransition,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ProofCreationResult<Vec<u8>>, Error> {
        let delta_contract_keeps_history = match state_transition {
            StateTransition::DataContractUpdate(update) if update.data_contract().is_none() => {
                let contract_id = update.data_contract_id();
                let Some(contract_fetch_info) = self.get_contract_with_fetch_info(
                    contract_id.to_buffer(),
                    false,
                    transaction,
                    platform_version,
                )?
                else {
                    return Ok(ProofCreationResult::new_with_error(
                        ProofError::UnknownContract(format!(
                            "unknown contract with id {} in the proof of a delta contract update",
                            contract_id
                        )),
                    ));
                };
                Some(contract_fetch_info.contract.config().keeps_history())
            }
            _ => None,
        };

        self.prove_state_transition_internal(
            state_transition,
            transaction,
            true,
            delta_contract_keeps_history,
            platform_version,
        )
    }
}
