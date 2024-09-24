use dapi_grpc::platform::v0::get_current_quorums_info_request::GetCurrentQuorumsInfoRequestV0;
use dapi_grpc::platform::v0::get_current_quorums_info_response::GetCurrentQuorumsInfoResponseV0;
use dpp::dashcore::hashes::Hash;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;

impl<C> Platform<C> {
    pub(super) fn query_current_quorums_info_v0(
        &self,
        GetCurrentQuorumsInfoRequestV0 {
        }: GetCurrentQuorumsInfoRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetCurrentQuorumsInfoResponseV0>, Error> {
        let validator_sets = platform_state.validator_sets_sorted_by_core_height_by_most_recent();
        platform_state.current_validator_set_quorum_hash()

        let response = GetCurrentQuorumsInfoResponseV0 {
            quorum_hashes: validator_sets.iter().map(|validator_set| validator_set.quorum_hash().as_byte_array().to_vec()).collect(),
            current_quorum_index: 0,
            current_quorum_members: vec![],
            current_proposer_index: 0,
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;

    #[test]
    fn test_query_empty_epoch_infos() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetEpochsInfoRequestV0 {
            start_epoch: None, // 0
            count: 5,
            ascending: true,
            prove: false,
        };

        let result = platform
            .query_epoch_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetEpochsInfoResponseV0 {
                result: Some(get_epochs_info_response_v0::Result::Epochs(EpochInfos { epoch_infos })),
                metadata: Some(_),
            }) if epoch_infos.is_empty()
        ));
    }

    #[test]
    fn test_query_empty_epoch_infos_descending() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetEpochsInfoRequestV0 {
            start_epoch: None, // 0
            count: 5,
            ascending: false,
            prove: false,
        };

        let validation_result = platform
            .query_epoch_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            validation_result.data,
            Some(GetEpochsInfoResponseV0 {
                result: Some(get_epochs_info_response_v0::Result::Epochs(EpochInfos { epoch_infos })),
                metadata: Some(_),
            }) if epoch_infos.is_empty()
        ));
    }
}
