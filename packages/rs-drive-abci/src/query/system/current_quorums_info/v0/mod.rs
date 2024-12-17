use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_current_quorums_info_request::GetCurrentQuorumsInfoRequestV0;
use dapi_grpc::platform::v0::get_current_quorums_info_response::{
    GetCurrentQuorumsInfoResponseV0, ValidatorSetV0, ValidatorV0,
};
use dpp::dashcore::hashes::Hash;

impl<C> Platform<C> {
    pub(super) fn query_current_quorums_info_v0(
        &self,
        GetCurrentQuorumsInfoRequestV0 {}: GetCurrentQuorumsInfoRequestV0,
        platform_state: &PlatformState,
    ) -> Result<QueryValidationResult<GetCurrentQuorumsInfoResponseV0>, Error> {
        // Get all validator sets sorted by core height
        let validator_sets_sorted =
            platform_state.validator_sets_sorted_by_core_height_by_most_recent();

        // Collect the validator sets in the desired format
        let validator_sets: Vec<ValidatorSetV0> = validator_sets_sorted
            .iter()
            .map(|validator_set| {
                // Map each ProTxHash to a ValidatorV0 object
                let members: Vec<ValidatorV0> = validator_set
                    .members()
                    .iter()
                    .map(|(pro_tx_hash, validator)| ValidatorV0 {
                        pro_tx_hash: pro_tx_hash.as_byte_array().to_vec(),
                        node_ip: validator.node_ip.clone(),
                        is_banned: validator.is_banned,
                    })
                    .collect();

                // Construct a ValidatorSetV0 object
                ValidatorSetV0 {
                    quorum_hash: validator_set.quorum_hash().as_byte_array().to_vec(),
                    core_height: validator_set.core_height(),
                    members,
                    threshold_public_key: validator_set
                        .threshold_public_key()
                        .0
                        .to_compressed()
                        .to_vec(),
                }
            })
            .collect();

        // Get the current proposer index and current quorum index, if applicable
        let current_quorum_index = platform_state.current_validator_set_quorum_hash();
        let last_committed_block_proposer_pro_tx_hash =
            platform_state.last_committed_block_proposer_pro_tx_hash();

        // Construct the response
        let response = GetCurrentQuorumsInfoResponseV0 {
            quorum_hashes: validator_sets_sorted
                .iter()
                .map(|validator_set| validator_set.quorum_hash().as_byte_array().to_vec())
                .collect(),
            validator_sets,
            last_block_proposer: last_committed_block_proposer_pro_tx_hash.to_vec(),
            current_quorum_hash: current_quorum_index.as_byte_array().to_vec(),
            metadata: Some(self.response_metadata_v0(platform_state)),
        };

        // Return the response wrapped in a QueryValidationResult
        Ok(QueryValidationResult::new_with_data(response))
    }
}
