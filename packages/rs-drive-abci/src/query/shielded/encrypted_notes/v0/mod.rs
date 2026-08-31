use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_encrypted_notes_request::GetShieldedEncryptedNotesRequestV0;
use dapi_grpc::platform::v0::get_shielded_encrypted_notes_response::get_shielded_encrypted_notes_response_v0::{
    EncryptedNote, EncryptedNotes,
};
use dapi_grpc::platform::v0::get_shielded_encrypted_notes_response::{
    get_shielded_encrypted_notes_response_v0, GetShieldedEncryptedNotesResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_credit_pool_path, shielded_credit_pool_path_vec, SHIELDED_NOTES_CHUNK_POWER,
    SHIELDED_NOTES_KEY,
};
use drive::grovedb::{PathQuery, Query, QueryItem, SizedQuery, SubqueryBranch};
use drive::grovedb_path::SubtreePath;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_shielded_encrypted_notes_v0(
        &self,
        GetShieldedEncryptedNotesRequestV0 {
            start_index,
            count,
            prove,
        }: GetShieldedEncryptedNotesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedEncryptedNotesResponseV0>, Error> {
        // Two distinct quantities:
        //   * `mmr_chunk_size` — the on-chain MMR chunk size
        //     (`1 << SHIELDED_NOTES_CHUNK_POWER` = 2048 today). This is the
        //     alignment unit: `start_index` MUST be a multiple of this so
        //     every query begins at an MMR chunk boundary.
        //   * `max_query_chunks` — the per-query CAP, expressed in chunks.
        //     One query may span up to this many adjacent MMR chunks, so
        //     the wire-level note limit is `max_query_chunks × mmr_chunk_size`.
        //     Decoupling the cap from the chunk size is what lets us bump
        //     throughput without touching the on-chain tree shape.
        let mmr_chunk_size: u64 = 1u64 << SHIELDED_NOTES_CHUNK_POWER;
        let max_query_chunks = platform_version
            .drive_abci
            .query
            .shielded_queries
            .max_query_chunks as u32;
        // `saturating_mul` on u32 already caps at u32::MAX — no extra
        // clamp needed.
        let max_notes = max_query_chunks.saturating_mul(mmr_chunk_size as u32);

        if start_index % mmr_chunk_size != 0 {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "start_index {} is not chunk-aligned; must be a multiple of {}",
                    start_index, mmr_chunk_size
                )),
            ));
        }

        let effective = if count == 0 || count > max_notes {
            max_notes
        } else {
            count
        };
        let limit = effective.min(u16::MAX as u32) as u16;

        let response = if prove {
            // V1 proof: PathQuery with subquery targeting positions in the CommitmentTree
            let end_index = start_index + limit as u64 - 1;
            let mut inner_query = Query::new();
            inner_query.insert_range_inclusive(
                start_index.to_be_bytes().to_vec()..=end_index.to_be_bytes().to_vec(),
            );

            let path_query = PathQuery {
                path: shielded_credit_pool_path_vec(),
                query: SizedQuery {
                    query: Query {
                        read_mode: None,
                        items: vec![QueryItem::Key(vec![SHIELDED_NOTES_KEY])],
                        default_subquery_branch: SubqueryBranch {
                            subquery_path: None,
                            subquery: Some(inner_query.into()),
                        },
                        left_to_right: true,
                        conditional_subquery_branches: None,
                        add_parent_tree_on_subquery: false,
                    },
                    limit: None,
                    offset: None,
                },
            };

            let proof =
                check_validation_result_with_data!(self.drive.grove_get_proved_path_query_v1(
                    &path_query,
                    &mut vec![],
                    &platform_version.drive,
                ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetShieldedEncryptedNotesResponseV0 {
                result: Some(get_shielded_encrypted_notes_response_v0::Result::Proof(
                    proof,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            // Non-proved: loop over commitment_tree_get_value for each position
            let pool_path = shielded_credit_pool_path();
            let pool_subtree: SubtreePath<&[u8]> = (&pool_path).into();
            let notes_key: &[u8] = &[SHIELDED_NOTES_KEY];

            let mut entries = Vec::with_capacity(limit as usize);
            for pos in start_index..(start_index + limit as u64) {
                let maybe_value = self
                    .drive
                    .grove
                    .commitment_tree_get_value(
                        pool_subtree.clone(),
                        notes_key,
                        pos,
                        None,
                        &platform_version.drive.grove_version,
                    )
                    .unwrap()
                    .map_err(|e| Error::Drive(drive::error::Error::GroveDB(Box::new(e))))?;

                match maybe_value {
                    // Stored value = cmx (32) || rho (32) || cv_net (32) || encrypted_note (rest)
                    Some(value) if value.len() > 96 => {
                        entries.push(EncryptedNote {
                            cmx: value[..32].to_vec(),
                            nullifier: value[32..64].to_vec(),
                            cv_net: value[64..96].to_vec(),
                            encrypted_note: value[96..].to_vec(),
                        });
                    }
                    _ => break, // past end of tree
                }
            }

            GetShieldedEncryptedNotesResponseV0 {
                result: Some(
                    get_shielded_encrypted_notes_response_v0::Result::EncryptedNotes(
                        EncryptedNotes { entries },
                    ),
                ),
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

    /// MMR chunk size used for alignment. Derived from
    /// `SHIELDED_NOTES_CHUNK_POWER`; independent of `max_query_chunks`.
    fn mmr_chunk_size() -> u64 {
        1u64 << SHIELDED_NOTES_CHUNK_POWER
    }

    /// Per-query cap on returned notes: `max_query_chunks × mmr_chunk_size`.
    fn max_notes(version: &PlatformVersion) -> u32 {
        let chunks = version.drive_abci.query.shielded_queries.max_query_chunks as u32;
        chunks.saturating_mul(mmr_chunk_size() as u32)
    }

    #[test]
    fn test_v0_non_aligned_start_index_errors() {
        // Non-aligned start_index branch: returns InvalidArgument directly.
        // Derive the unaligned value from the versioned chunk size so this
        // test never degrades into a vacuous check if the constant is later
        // tuned to 1 or 5.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let chunk = mmr_chunk_size();
        assert!(
            chunk > 1,
            "test requires a chunk size > 1 so an unaligned start_index exists"
        );

        let request = GetShieldedEncryptedNotesRequestV0 {
            start_index: chunk - 1, // not aligned to chunk size
            count: 10,
            prove: false,
        };

        let result = platform
            .query_shielded_encrypted_notes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("not chunk-aligned")
        ));
    }

    #[test]
    fn test_v0_non_aligned_large_start_index_errors() {
        // An almost-aligned value (chunk_size + 1) must still be rejected.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let chunk = mmr_chunk_size();

        let request = GetShieldedEncryptedNotesRequestV0 {
            start_index: chunk + 1,
            count: 10,
            prove: false,
        };

        let result = platform
            .query_shielded_encrypted_notes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("not chunk-aligned")
        ));
    }

    #[test]
    fn test_v0_aligned_start_at_chunk_size_boundary_ok() {
        // An aligned start_index equal to exactly chunk_size should succeed
        // (fresh pool → empty result set).
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let chunk = mmr_chunk_size();

        let request = GetShieldedEncryptedNotesRequestV0 {
            start_index: chunk,
            count: 1,
            prove: false,
        };

        let result = platform
            .query_shielded_encrypted_notes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let data = result.data.unwrap();
        match data.result {
            Some(get_shielded_encrypted_notes_response_v0::Result::EncryptedNotes(notes)) => {
                assert!(notes.entries.is_empty());
            }
            other => panic!("expected EncryptedNotes, got {:?}", other),
        }
    }

    #[test]
    fn test_v0_aligned_start_at_multiple_of_chunk_size_ok() {
        // start_index = 2 * chunk_size must also be accepted.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let chunk = mmr_chunk_size();

        let request = GetShieldedEncryptedNotesRequestV0 {
            start_index: chunk * 2,
            count: 1,
            prove: false,
        };

        let result = platform
            .query_shielded_encrypted_notes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }

    #[test]
    fn test_v0_count_one_yields_limit_one() {
        // count=1 bypasses the "0 or > max" branch and sets effective=1.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedEncryptedNotesRequestV0 {
            start_index: 0,
            count: 1,
            prove: false,
        };

        let result = platform
            .query_shielded_encrypted_notes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let data = result.data.unwrap();
        match data.result {
            Some(get_shielded_encrypted_notes_response_v0::Result::EncryptedNotes(notes)) => {
                // Empty state → empty entries even with count=1.
                assert!(notes.entries.is_empty());
            }
            other => panic!("expected EncryptedNotes, got {:?}", other),
        }
    }

    #[test]
    fn test_v0_prove_path_aligned_start() {
        // Prove path on empty state with aligned start_index should return a
        // Proof variant.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedEncryptedNotesRequestV0 {
            start_index: 0,
            count: 16,
            prove: true,
        };

        let result = platform
            .query_shielded_encrypted_notes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert!(matches!(
            result.data,
            Some(GetShieldedEncryptedNotesResponseV0 {
                result: Some(get_shielded_encrypted_notes_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_v0_prove_path_rejects_unaligned_start() {
        // Non-aligned start_index is rejected even with prove=true — the
        // alignment check is *before* the prove branch.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedEncryptedNotesRequestV0 {
            start_index: 3,
            count: 4,
            prove: true,
        };

        let result = platform
            .query_shielded_encrypted_notes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("not chunk-aligned")
        ));
    }

    #[test]
    fn test_v0_count_exactly_max_is_accepted() {
        // count == max is neither `0` nor `> max`, so it falls through the
        // inner `else` that keeps count as-is. Covers that fallthrough.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let max = max_notes(version);

        let request = GetShieldedEncryptedNotesRequestV0 {
            start_index: 0,
            count: max,
            prove: false,
        };

        let result = platform
            .query_shielded_encrypted_notes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }

    #[test]
    fn test_v0_start_index_zero_is_always_aligned() {
        // start_index = 0 is always aligned (any X % chunk_size for 0 is 0).
        // Exercises the `start_index % chunk_size == 0` short-path.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedEncryptedNotesRequestV0 {
            start_index: 0,
            count: 8,
            prove: false,
        };

        let result = platform
            .query_shielded_encrypted_notes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }
}
