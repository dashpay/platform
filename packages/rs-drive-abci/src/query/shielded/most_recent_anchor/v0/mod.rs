use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_most_recent_shielded_anchor_request::GetMostRecentShieldedAnchorRequestV0;
use dapi_grpc::platform::v0::get_most_recent_shielded_anchor_response::{
    get_most_recent_shielded_anchor_response_v0, GetMostRecentShieldedAnchorResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::shielded_latest_recorded_anchor_path_query;
use drive::error::drive::DriveError;
use drive::grovedb::query_result_type::QueryResultType;
use drive::grovedb::Element;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Answer `getMostRecentShieldedAnchor` by reading the latest
    /// entry from the anchors-by-height index
    /// (`[..., "s", [96]]`) — a `limit 1` reverse query. The anchor
    /// is the value at the highest block-height key.
    ///
    /// Returns `[0; 32]` (mapped to `None` by the response decoder
    /// downstream) when the index is empty — the pool has never
    /// recorded an anchor yet on this chain.
    pub(super) fn query_most_recent_shielded_anchor_v0(
        &self,
        GetMostRecentShieldedAnchorRequestV0 { prove }: GetMostRecentShieldedAnchorRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetMostRecentShieldedAnchorResponseV0>, Error> {
        let path_query = shielded_latest_recorded_anchor_path_query();

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.grove_get_proved_path_query(
                &path_query,
                None,
                &mut vec![],
                &platform_version.drive,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetMostRecentShieldedAnchorResponseV0 {
                result: Some(get_most_recent_shielded_anchor_response_v0::Result::Proof(
                    proof,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let (results, _) = self.drive.grove_get_raw_path_query(
                &path_query,
                None,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )?;

            let entries = results.to_key_elements();
            // EMPTY-INDEX CONVENTION (see also
            // `Drive::verify_most_recent_shielded_anchor_v0`):
            //
            // The `Anchor`/`Proof` `oneof` in the gRPC response has
            // no third "absent" variant, so the proven and non-proven
            // branches encode "no anchor recorded yet" differently:
            //
            //   * Non-proven (this branch) — return
            //     `Anchor(vec![0u8; 32])`. Callers treat the all-zero
            //     anchor as "absent." Carries over from the
            //     pre-`[7]`-removal behaviour where the slot was
            //     initialised to `[0; 32]`; kept for wire-format
            //     compatibility.
            //   * Proven — return the GroveDB proof. The SDK runs
            //     `verify_most_recent_shielded_anchor_v0` against
            //     the same `limit 1` reverse `PathQuery`, which
            //     returns `Ok(None)` when the index is empty.
            //
            // If a future protocol revision extends the response
            // with an explicit absent variant, drop the
            // `vec![0u8; 32]` sentinel here in lock-step with both
            // the verifier and the SDK decoder.
            // Split the three states explicitly. A non-`Item`
            // element under `SHIELDED_ANCHORS_BY_HEIGHT_KEY` is a
            // state-corruption signal — the prove-mode verifier
            // (`verify_most_recent_shielded_anchor_v0`) and the
            // raw-read helper
            // (`Drive::read_latest_recorded_shielded_anchor_v0`)
            // both surface this as `CorruptedProof` /
            // `CorruptedElementType`, so the non-prove path must
            // not silently fold it into the empty-index sentinel.
            let anchor_bytes = match entries.into_iter().next() {
                Some((_height_key, Element::Item(bytes, _))) => bytes,
                Some(_) => {
                    return Err(Error::Drive(drive::error::Error::Drive(
                        DriveError::CorruptedElementType("anchors-by-height entry is not an Item"),
                    )));
                }
                None => vec![0u8; 32],
            };

            GetMostRecentShieldedAnchorResponseV0 {
                result: Some(get_most_recent_shielded_anchor_response_v0::Result::Anchor(
                    anchor_bytes,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
