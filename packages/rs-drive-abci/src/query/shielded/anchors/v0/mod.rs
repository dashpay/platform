use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_anchors_request::GetShieldedAnchorsRequestV0;
use dapi_grpc::platform::v0::get_shielded_anchors_response::get_shielded_anchors_response_v0::Anchors;
use dapi_grpc::platform::v0::get_shielded_anchors_response::{
    get_shielded_anchors_response_v0, GetShieldedAnchorsResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::shielded_credit_pool_anchors_path_vec;
use drive::grovedb::query_result_type::QueryResultType;
use drive::grovedb::{PathQuery, Query, SizedQuery};
use drive::util::grove_operations::GroveDBToUse;

/// Derive V0's complete-response envelope from the active retention policy.
///
/// The retention policy caps the retained set at
/// `max_retained_shielded_anchors()`. We allow one further pruning interval of
/// operational headroom on top of that worst case,
/// so this unpaginated legacy query never fails closed on legitimately in-policy
/// state while staying coupled to the pruning algorithm rather than re-deriving
/// the bound from the raw constants.
///
/// Returns `(safety_limit, preflight_limit)`, or `None` if the active policy
/// exceeds what this V0 response can enumerate within a `u16` query limit.
fn v0_shielded_anchor_limits(platform_version: &PlatformVersion) -> Option<(u16, u16)> {
    let event_constants = &platform_version
        .drive_abci
        .validation_and_processing
        .event_constants;
    let safety_limit = event_constants
        .max_retained_shielded_anchors()
        .and_then(|max_retained| {
            max_retained.checked_add(event_constants.shielded_anchor_pruning_interval)
        })
        .and_then(|limit| u16::try_from(limit).ok())?;
    let preflight_limit = safety_limit.checked_add(1)?;

    Some((safety_limit, preflight_limit))
}

impl<C> Platform<C> {
    pub(super) fn query_shielded_anchors_v0(
        &self,
        GetShieldedAnchorsRequestV0 { prove }: GetShieldedAnchorsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedAnchorsResponseV0>, Error> {
        let Some((safety_limit, preflight_limit)) = v0_shielded_anchor_limits(platform_version)
        else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::ResourceExhausted(
                    "active shielded anchor retention policy exceeds the V0 query envelope"
                        .to_string(),
                ),
            ));
        };

        // Read at most one entry over the budget before doing any full-range
        // proof work. This keeps V0 wire-compatible for valid retained state
        // while failing closed if configuration or state ever exceeds the
        // amount that this unpaginated response can safely return.
        let bounded_path_query = PathQuery {
            path: shielded_credit_pool_anchors_path_vec(),
            query: SizedQuery {
                query: Query::new_range_full(),
                limit: Some(preflight_limit),
                offset: None,
            },
        };
        let (bounded_results, _) = self.drive.grove_get_raw_path_query(
            &bounded_path_query,
            None,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;
        let bounded_key_elements = bounded_results.to_key_elements();
        if bounded_key_elements.len() > usize::from(safety_limit) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::ResourceExhausted(format!(
                    "retained shielded anchor set exceeds the V0 safety limit of {}",
                    safety_limit
                )),
            ));
        }

        let path_query = PathQuery {
            path: shielded_credit_pool_anchors_path_vec(),
            query: SizedQuery {
                query: Query::new_range_full(),
                limit: None,
                offset: None,
            },
        };

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.grove_get_proved_path_query(
                &path_query,
                None,
                &mut vec![],
                &platform_version.drive,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetShieldedAnchorsResponseV0 {
                result: Some(get_shielded_anchors_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            // Anchors are stored as anchor_bytes → block_height_be; extract keys
            let anchors: Vec<Vec<u8>> = bounded_key_elements
                .into_iter()
                .map(|(key, _element)| key)
                .collect();

            GetShieldedAnchorsResponseV0 {
                result: Some(get_shielded_anchors_response_v0::Result::Anchors(Anchors {
                    anchors,
                })),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;
    use drive::drive::Drive;
    use drive::grovedb::batch::QualifiedGroveDbOp;
    use drive::grovedb::Element;
    use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use drive::util::batch::GroveDbOpBatch;

    #[test]
    fn accepts_the_safety_limit_and_rejects_one_more_before_proof_generation() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let (safety_limit, _) =
            v0_shielded_anchor_limits(version).expect("versioned policy must fit V0 envelope");
        let anchors_path = shielded_credit_pool_anchors_path_vec();
        let operations = (0..safety_limit)
            .map(|index| {
                let mut anchor = [0_u8; 32];
                anchor[..2].copy_from_slice(&index.to_be_bytes());
                QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
                    anchors_path.clone(),
                    anchor.to_vec(),
                    Element::new_item(u64::from(index).to_be_bytes().to_vec()),
                )
            })
            .collect();

        platform
            .drive
            .grove_apply_batch(
                GroveDbOpBatch::from_operations(operations),
                false,
                None,
                &version.drive,
            )
            .expect("expected oversized anchor fixture to be inserted");

        let at_limit = platform
            .query_shielded_anchors_v0(
                GetShieldedAnchorsRequestV0 { prove: false },
                &state,
                version,
            )
            .expect("expected at-limit query to complete");
        let anchors = match at_limit.data.and_then(|response| response.result) {
            Some(get_shielded_anchors_response_v0::Result::Anchors(anchors)) => anchors.anchors,
            other => panic!("expected anchors response at safety limit, got {other:?}"),
        };
        assert_eq!(anchors.len(), usize::from(safety_limit));

        let mut extra_anchor = [0_u8; 32];
        extra_anchor[..2].copy_from_slice(&safety_limit.to_be_bytes());
        platform
            .drive
            .grove_apply_batch(
                GroveDbOpBatch::from_operations(vec![
                    QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
                        anchors_path,
                        extra_anchor.to_vec(),
                        Element::new_item(u64::from(safety_limit).to_be_bytes().to_vec()),
                    ),
                ]),
                false,
                None,
                &version.drive,
            )
            .expect("expected over-limit anchor to be inserted");

        for prove in [false, true] {
            let result = platform
                .query_shielded_anchors_v0(GetShieldedAnchorsRequestV0 { prove }, &state, version)
                .expect("expected bounded query to complete");

            assert!(matches!(
                result.errors.as_slice(),
                [QueryError::ResourceExhausted(message)]
                    if message.contains("V0 safety limit")
            ));
            assert!(result.data.is_none());
        }
    }

    #[test]
    fn small_anchor_set_round_trips_for_plain_and_proved_queries() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let anchors_path = shielded_credit_pool_anchors_path_vec();
        let expected: Vec<[u8; 32]> = [3_u8, 1, 2]
            .into_iter()
            .map(|suffix| {
                let mut anchor = [0_u8; 32];
                anchor[31] = suffix;
                anchor
            })
            .collect();
        let operations = expected
            .iter()
            .enumerate()
            .map(|(height, anchor)| {
                QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
                    anchors_path.clone(),
                    anchor.to_vec(),
                    Element::new_item((height as u64).to_be_bytes().to_vec()),
                )
            })
            .collect();
        platform
            .drive
            .grove_apply_batch(
                GroveDbOpBatch::from_operations(operations),
                false,
                None,
                &version.drive,
            )
            .expect("insert small anchor fixture");

        let mut expected_sorted = expected;
        expected_sorted.sort_unstable();

        let plain = platform
            .query_shielded_anchors_v0(
                GetShieldedAnchorsRequestV0 { prove: false },
                &state,
                version,
            )
            .expect("plain small-state query");
        let plain_anchors = match plain.data.and_then(|response| response.result) {
            Some(get_shielded_anchors_response_v0::Result::Anchors(anchors)) => anchors
                .anchors
                .into_iter()
                .map(|anchor| anchor.try_into().expect("32-byte anchor"))
                .collect::<Vec<[u8; 32]>>(),
            other => panic!("expected plain anchors, got {other:?}"),
        };
        assert_eq!(plain_anchors, expected_sorted);

        let proved = platform
            .query_shielded_anchors_v0(GetShieldedAnchorsRequestV0 { prove: true }, &state, version)
            .expect("proved small-state query");
        let proof = match proved.data.and_then(|response| response.result) {
            Some(get_shielded_anchors_response_v0::Result::Proof(proof)) => proof.grovedb_proof,
            other => panic!("expected anchors proof, got {other:?}"),
        };
        let (_, proved_anchors) = Drive::verify_shielded_anchors(&proof, false, version)
            .expect("verify small-state anchor proof");
        assert_eq!(proved_anchors, expected_sorted);
    }

    #[test]
    fn v0_envelope_covers_the_retention_policy_for_every_platform_version() {
        // The V0 safety envelope must cover the worst-case retained set for
        // every supported protocol version. This binds the query bound to the
        // pruning policy: a future version that tunes retention/pruning (or an
        // edit that weakens the headroom) fails here instead of silently
        // rejecting legitimately in-policy state at runtime.
        for platform_version in dpp::version::PLATFORM_VERSIONS {
            let event_constants = &platform_version
                .drive_abci
                .validation_and_processing
                .event_constants;

            let max_retained = event_constants
                .max_retained_shielded_anchors()
                .expect("retention policy must not overflow u64");
            assert_eq!(
                max_retained,
                event_constants.shielded_anchor_retention_blocks
                    + event_constants.shielded_anchor_pruning_interval,
                "protocol version {} max-retained drifted from retention + interval",
                platform_version.protocol_version
            );

            let (safety_limit, preflight_limit) = v0_shielded_anchor_limits(platform_version)
                .unwrap_or_else(|| {
                    panic!(
                        "protocol version {} retention policy overflows the V0 u16 envelope",
                        platform_version.protocol_version
                    )
                });

            assert!(
                u64::from(safety_limit) >= max_retained,
                "protocol version {} safety limit {} does not cover max retained {}",
                platform_version.protocol_version,
                safety_limit,
                max_retained
            );
            assert_eq!(
                preflight_limit,
                safety_limit + 1,
                "preflight must read exactly one past the safety limit"
            );
        }
    }
}
