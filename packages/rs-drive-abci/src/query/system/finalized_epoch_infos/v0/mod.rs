use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;

use dapi_grpc::platform::v0::get_finalized_epoch_infos_request::GetFinalizedEpochInfosRequestV0;
use dapi_grpc::platform::v0::get_finalized_epoch_infos_response::{
    get_finalized_epoch_infos_response_v0, GetFinalizedEpochInfosResponseV0,
};
use dapi_grpc::platform::v0::get_finalized_epoch_infos_response::get_finalized_epoch_infos_response_v0::{BlockProposer, FinalizedEpochInfo, FinalizedEpochInfos};
use dpp::block::finalized_epoch_info::v0::getters::FinalizedEpochInfoGettersV0;
use dpp::block::epoch::MAX_EPOCH;
use dpp::check_validation_result_with_data;
use dpp::version::PlatformVersion;
use dpp::validation::ValidationResult;
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;

fn epoch_range_cardinality(start: u32, start_included: bool, end: u32, end_included: bool) -> u64 {
    if start == end {
        return u64::from(start_included && end_included);
    }

    u64::from(start.abs_diff(end)) + 1 - u64::from(!start_included) - u64::from(!end_included)
}

impl<C> Platform<C> {
    pub(super) fn query_finalized_epoch_infos_v0(
        &self,
        GetFinalizedEpochInfosRequestV0 {
            start_epoch_index,
            start_epoch_index_included,
            end_epoch_index,
            end_epoch_index_included,
            prove,
        }: GetFinalizedEpochInfosRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetFinalizedEpochInfosResponseV0>, Error> {
        // Stored epoch keys reserve a prefix range, so the public index must
        // fit before the internal offset is added.
        if start_epoch_index > MAX_EPOCH as u32 {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "start_epoch_index {} exceeds maximum value of {}",
                    start_epoch_index, MAX_EPOCH
                )),
            ));
        }

        if end_epoch_index > MAX_EPOCH as u32 {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "end_epoch_index {} exceeds maximum value of {}",
                    end_epoch_index, MAX_EPOCH
                )),
            ));
        }

        if end_epoch_index == start_epoch_index
            && (!end_epoch_index_included || !start_epoch_index_included)
        {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(
                    format!("when start_epoch_index equals end_epoch_index ({}), both boundaries must be included to return a result", start_epoch_index),
                ),
            ));
        }

        let requested_epochs = epoch_range_cardinality(
            start_epoch_index,
            start_epoch_index_included,
            end_epoch_index,
            end_epoch_index_included,
        );
        let max_epochs = u64::from(platform_version.drive_abci.query.max_returned_elements);
        if requested_epochs > max_epochs {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "requested epoch range contains {} elements, maximum is {}",
                    requested_epochs, max_epochs
                )),
            ));
        }

        if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_finalized_epoch_infos(
                    start_epoch_index as u16,
                    start_epoch_index_included,
                    end_epoch_index as u16,
                    end_epoch_index_included,
                    None,
                    platform_version,
                )
                .map_err(QueryError::Drive));

            Ok(QueryValidationResult::new_with_data(
                GetFinalizedEpochInfosResponseV0 {
                    result: Some(get_finalized_epoch_infos_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                            .map(|(_, proof)| proof)?,
                    )),
                    metadata: Some(
                        self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                    ),
                },
            ))
        } else {
            let finalized_epoch_infos: Vec<_> = check_validation_result_with_data!(self
                .drive
                .get_finalized_epoch_infos(
                    start_epoch_index as u16,
                    start_epoch_index_included,
                    end_epoch_index as u16,
                    end_epoch_index_included,
                    None,
                    platform_version,
                )
                .map_err(QueryError::Drive));

            let finalized_epoch_infos: Vec<FinalizedEpochInfo> = finalized_epoch_infos
                .into_iter()
                .map(|(epoch_index, finalized_info)| {
                    // Convert the BTreeMap to the proto repeated field format
                    let block_proposers = finalized_info
                        .block_proposers()
                        .iter()
                        .map(|(id, count)| BlockProposer {
                            proposer_id: id.to_vec(),
                            block_count: *count as u32,
                        })
                        .collect();

                    FinalizedEpochInfo {
                        number: epoch_index as u32,
                        first_block_height: finalized_info.first_block_height(),
                        first_core_block_height: finalized_info.first_core_block_height(),
                        first_block_time: finalized_info.first_block_time(),
                        fee_multiplier: finalized_info.fee_multiplier_permille() as f64 / 1000.0,
                        protocol_version: finalized_info.protocol_version(),
                        total_blocks_in_epoch: finalized_info.total_blocks_in_epoch(),
                        next_epoch_start_core_block_height: finalized_info
                            .next_epoch_start_core_block_height(),
                        total_processing_fees: finalized_info.total_processing_fees(),
                        total_distributed_storage_fees: finalized_info
                            .total_distributed_storage_fees(),
                        total_created_storage_fees: finalized_info.total_created_storage_fees(),
                        core_block_rewards: finalized_info.core_block_rewards(),
                        block_proposers,
                    }
                })
                .collect();

            Ok(QueryValidationResult::new_with_data(
                GetFinalizedEpochInfosResponseV0 {
                    result: Some(get_finalized_epoch_infos_response_v0::Result::Epochs(
                        FinalizedEpochInfos {
                            finalized_epoch_infos,
                        },
                    )),
                    metadata: Some(
                        self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                    ),
                },
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;

    #[test]
    fn test_query_finalized_epoch_infos_start_out_of_range() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetFinalizedEpochInfosRequestV0 {
            start_epoch_index: (u16::MAX as u32) + 1,
            start_epoch_index_included: true,
            end_epoch_index: 10,
            end_epoch_index_included: true,
            prove: false,
        };

        let result = platform
            .query_finalized_epoch_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("start_epoch_index")
        ));
    }

    #[test]
    fn test_query_finalized_epoch_infos_end_out_of_range() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetFinalizedEpochInfosRequestV0 {
            start_epoch_index: 0,
            start_epoch_index_included: true,
            end_epoch_index: (u16::MAX as u32) + 1,
            end_epoch_index_included: true,
            prove: false,
        };

        let result = platform
            .query_finalized_epoch_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("end_epoch_index")
        ));
    }

    #[test]
    fn test_epoch_range_cardinality_respects_direction_and_boundaries() {
        assert_eq!(epoch_range_cardinality(0, true, 99, true), 100);
        assert_eq!(epoch_range_cardinality(99, true, 0, true), 100);
        assert_eq!(epoch_range_cardinality(0, true, 100, true), 101);
        assert_eq!(epoch_range_cardinality(0, true, 100, false), 100);
        assert_eq!(epoch_range_cardinality(0, false, 100, true), 100);
        assert_eq!(epoch_range_cardinality(0, false, 100, false), 99);
        assert_eq!(epoch_range_cardinality(7, true, 7, true), 1);
        assert_eq!(epoch_range_cardinality(7, true, 7, false), 0);
    }

    #[test]
    fn test_query_finalized_epoch_infos_rejects_ranges_above_response_limit() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let max_epochs = u32::from(version.drive_abci.query.max_returned_elements);

        for (start_epoch_index, end_epoch_index, prove) in [
            (0, max_epochs, false),
            (max_epochs, 0, false),
            (0, max_epochs, true),
        ] {
            let request = GetFinalizedEpochInfosRequestV0 {
                start_epoch_index,
                start_epoch_index_included: true,
                end_epoch_index,
                end_epoch_index_included: true,
                prove,
            };

            let result = platform
                .query_finalized_epoch_infos_v0(request, &state, version)
                .expect("expected query validation to succeed");

            assert!(matches!(
                result.errors.as_slice(),
                [QueryError::InvalidArgument(msg)] if msg.contains("requested epoch range")
            ));
            assert!(result.data.is_none());
        }
    }

    #[test]
    fn test_query_finalized_epoch_infos_accepts_range_at_response_limit() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let max_epochs = u32::from(version.drive_abci.query.max_returned_elements);

        let request = GetFinalizedEpochInfosRequestV0 {
            start_epoch_index: 0,
            start_epoch_index_included: true,
            end_epoch_index: max_epochs - 1,
            end_epoch_index_included: true,
            prove: false,
        };

        let result = platform
            .query_finalized_epoch_infos_v0(request, &state, version)
            .expect("expected bounded query to succeed");

        assert!(result.errors.is_empty());
        assert!(result.data.is_some());
    }

    #[test]
    fn test_query_finalized_epoch_infos_equal_indexes_without_both_included() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetFinalizedEpochInfosRequestV0 {
            start_epoch_index: 3,
            start_epoch_index_included: true,
            end_epoch_index: 3,
            end_epoch_index_included: false,
            prove: false,
        };

        let result = platform
            .query_finalized_epoch_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("both boundaries must be included")
        ));
    }

    #[test]
    fn test_query_empty_finalized_epoch_infos() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetFinalizedEpochInfosRequestV0 {
            start_epoch_index: 0,
            start_epoch_index_included: true,
            end_epoch_index: 5,
            end_epoch_index_included: true,
            prove: false,
        };

        let result = platform
            .query_finalized_epoch_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetFinalizedEpochInfosResponseV0 {
                result: Some(get_finalized_epoch_infos_response_v0::Result::Epochs(FinalizedEpochInfos { finalized_epoch_infos })),
                metadata: Some(_),
            }) if finalized_epoch_infos.is_empty()
        ));
    }

    #[test]
    fn test_query_empty_finalized_epoch_infos_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetFinalizedEpochInfosRequestV0 {
            start_epoch_index: 0,
            start_epoch_index_included: true,
            end_epoch_index: 5,
            end_epoch_index_included: true,
            prove: true,
        };

        let result = platform
            .query_finalized_epoch_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetFinalizedEpochInfosResponseV0 {
                result: Some(get_finalized_epoch_infos_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_finalized_epoch_infos_equal_indexes_both_included() {
        // When start == end and both boundaries are included, the range
        // validation passes; the response should be an empty epoch list
        // on an uninitialised platform with no finalized epochs yet.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetFinalizedEpochInfosRequestV0 {
            start_epoch_index: 3,
            start_epoch_index_included: true,
            end_epoch_index: 3,
            end_epoch_index_included: true,
            prove: false,
        };

        let result = platform
            .query_finalized_epoch_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetFinalizedEpochInfosResponseV0 {
                result: Some(get_finalized_epoch_infos_response_v0::Result::Epochs(FinalizedEpochInfos { finalized_epoch_infos })),
                metadata: Some(_),
            }) if finalized_epoch_infos.is_empty()
        ));
    }

    #[test]
    fn test_query_finalized_epoch_infos_start_at_max_does_not_trigger_range_guard() {
        // MAX_EPOCH is the largest public index that remains representable
        // after storage adds the reserved epoch-key offset.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetFinalizedEpochInfosRequestV0 {
            start_epoch_index: MAX_EPOCH as u32,
            start_epoch_index_included: true,
            end_epoch_index: MAX_EPOCH as u32,
            end_epoch_index_included: true,
            prove: false,
        };

        let result = platform
            .query_finalized_epoch_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        // No endpoint-range InvalidArgument should be emitted at MAX_EPOCH.
        for err in &result.errors {
            if let QueryError::InvalidArgument(msg) = err {
                assert!(
                    !msg.contains("start_epoch_index") || !msg.contains("exceeds maximum"),
                    "range guard fired at MAX_EPOCH boundary: {msg}"
                );
            }
        }
    }

    #[test]
    fn test_query_finalized_epoch_infos_rejects_index_above_max_epoch() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetFinalizedEpochInfosRequestV0 {
            start_epoch_index: u32::from(MAX_EPOCH) + 1,
            start_epoch_index_included: true,
            end_epoch_index: 0,
            end_epoch_index_included: true,
            prove: false,
        };

        let result = platform
            .query_finalized_epoch_infos_v0(request, &state, version)
            .expect("expected query validation to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("start_epoch_index")
        ));
    }
}
