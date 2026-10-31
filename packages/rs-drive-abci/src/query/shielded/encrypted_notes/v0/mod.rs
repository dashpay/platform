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
use crate::query::shielded::ShieldedPoolSelector;
use drive::drive::shielded::paths::{SHIELDED_NOTES_CHUNK_POWER, SHIELDED_NOTES_KEY};
use drive::grovedb::{PathQuery, Query, QueryItem, SizedQuery, SubqueryBranch};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_shielded_encrypted_notes_v0(
        &self,
        GetShieldedEncryptedNotesRequestV0 {
            start_index,
            count,
            prove,
            token_id,
        }: GetShieldedEncryptedNotesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedEncryptedNotesResponseV0>, Error> {
        // Protocol versions 1 through 13 select this generation as well, and the selector
        // leaves a request any of them can make untouched: `token_id` is absent there,
        // `from_request` maps that to the credit pool without consulting the version, the credit
        // pool's path is the same one this handler used to build inline, and
        // `validate_pool_exists` is a no-op for it. The query, the proof and the response
        // therefore all stay as they were. A `token_id` is refused outright below the version
        // that admits token pools.
        let pool = match ShieldedPoolSelector::from_request(token_id, platform_version) {
            Ok(pool) => pool,
            Err(error) => return Ok(QueryValidationResult::new_with_error(error)),
        };
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
                path: pool.pool_path_vec(),
                query: SizedQuery {
                    query: Query {
                        read_mode: None,
                        limit: None,
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
            check_validation_result_with_data!(
                pool.validate_pool_exists(&self.drive, platform_version)?
            );

            // Non-proved: one chunk-aligned range read. Each compacted chunk
            // the page overlaps is read and deserialized once, where reading
            // position by position would deserialize the whole chunk blob
            // again for every note in it.
            let pool_path = pool.pool_path_vec();
            let page = self
                .drive
                .grove
                .commitment_tree_get_range(
                    pool_path.as_slice(),
                    &[SHIELDED_NOTES_KEY],
                    start_index,
                    limit,
                    None,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .map_err(|e| Error::Drive(drive::error::Error::GroveDB(Box::new(e))))?;

            // The page is contiguous from `start_index` and already stops at
            // the end of the tree.
            let mut entries = Vec::with_capacity(page.entries.len());
            for (_, value) in page.entries {
                // Stored value = cmx (32) || rho (32) || cv_net (32) || encrypted_note (rest)
                if value.len() <= 96 {
                    break;
                }
                entries.push(EncryptedNote {
                    cmx: value[..32].to_vec(),
                    nullifier: value[32..64].to_vec(),
                    cv_net: value[64..96].to_vec(),
                    encrypted_note: value[96..].to_vec(),
                });
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
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::dashcore::Network;
    use drive::drive::shielded::paths::shielded_credit_pool_path;
    use grovedb_commitment_tree::{DashMemo, NoteBytesData, TransmittedNoteCiphertext};

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
            token_id: None,
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
            token_id: None,
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
            token_id: None,
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
            token_id: None,
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
            token_id: None,
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
            token_id: None,
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
            token_id: None,
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
            token_id: None,
        };

        let result = platform
            .query_shielded_encrypted_notes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }

    /// 32 bytes naming a note position and one of its fields, so a page that
    /// returns the wrong position or misplaces a field cannot compare equal.
    /// Small enough to be a valid Pallas base element when used as a cmx.
    fn position_tag(pos: u64, field: u64) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&(pos + 1).to_le_bytes());
        bytes[8..16].copy_from_slice(&field.to_le_bytes());
        bytes
    }

    /// Appends `count` notes whose cmx, rho, cv_net and ciphertext all carry
    /// the note's `position_tag`.
    fn insert_position_tagged_notes(
        platform: &TempPlatform<MockCoreRPCLike>,
        count: u64,
        version: &PlatformVersion,
    ) {
        let pool_path = shielded_credit_pool_path();
        let transaction = platform.drive.grove.start_transaction();
        for pos in 0..count {
            let ciphertext: TransmittedNoteCiphertext<DashMemo> =
                TransmittedNoteCiphertext::from_parts(
                    position_tag(pos, 4),
                    NoteBytesData([(pos % 251) as u8; 104]),
                    [(pos % 241) as u8; 80],
                );
            platform
                .drive
                .grove
                .commitment_tree_insert(
                    &pool_path,
                    &[SHIELDED_NOTES_KEY],
                    position_tag(pos, 1),
                    position_tag(pos, 2),
                    position_tag(pos, 3),
                    ciphertext,
                    Some(&transaction),
                    &version.drive.grove_version,
                )
                .unwrap()
                .expect("should insert note");
        }
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("should commit notes");
    }

    /// The non-proved read as it was before the range read: one
    /// `commitment_tree_get_value` per position, stopping at the first
    /// position that holds no note.
    fn notes_read_one_position_at_a_time(
        platform: &TempPlatform<MockCoreRPCLike>,
        start_index: u64,
        limit: u16,
        version: &PlatformVersion,
    ) -> Vec<EncryptedNote> {
        let pool_path = shielded_credit_pool_path();
        let mut entries = Vec::new();
        for pos in start_index..(start_index + limit as u64) {
            let maybe_value = platform
                .drive
                .grove
                .commitment_tree_get_value(
                    &pool_path,
                    &[SHIELDED_NOTES_KEY],
                    pos,
                    None,
                    &version.drive.grove_version,
                )
                .unwrap()
                .expect("should read note");
            match maybe_value {
                Some(value) if value.len() > 96 => entries.push(EncryptedNote {
                    cmx: value[..32].to_vec(),
                    nullifier: value[32..64].to_vec(),
                    cv_net: value[64..96].to_vec(),
                    encrypted_note: value[96..].to_vec(),
                }),
                _ => break,
            }
        }
        entries
    }

    #[test]
    fn test_v0_range_read_matches_per_position_reads_across_chunk_and_buffer() {
        // One compacted chunk plus a few notes in the dense buffer, so pages
        // cover the chunk alone, the buffer alone, and a span across both.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let chunk = mmr_chunk_size();
        let buffered = 5;
        let total = chunk + buffered;
        insert_position_tagged_notes(&platform, total, version);

        // The pre-change read over the whole pool is the reference: a
        // per-position loop over any page equals the matching slice of it.
        let max = max_notes(version) as u64;
        assert!(max > total, "one page must be able to hold the whole pool");
        let reference = notes_read_one_position_at_a_time(&platform, 0, max as u16, version);
        assert_eq!(reference.len() as u64, total);
        for (pos, note) in reference.iter().enumerate() {
            let pos = pos as u64;
            assert_eq!(note.cmx, position_tag(pos, 1), "cmx at {}", pos);
            assert_eq!(note.nullifier, position_tag(pos, 2), "rho at {}", pos);
            assert_eq!(note.cv_net, position_tag(pos, 3), "cv_net at {}", pos);
        }

        let pages: [(u64, u32); 9] = [
            (0, 0),                // default: the whole pool
            (0, 1),                // first note of the chunk
            (0, chunk as u32 - 1), // chunk minus its last note
            (0, chunk as u32),     // exactly the chunk
            (0, chunk as u32 + 2), // chunk plus part of the buffer
            (0, max as u32 + 1),   // over the cap: clamped to max
            (chunk, 3),            // part of the buffer
            (chunk, 0),            // buffer to the end of the tree
            (chunk * 2, 0),        // past the end of the tree
        ];
        for (start_index, count) in pages {
            let effective = if count == 0 || count as u64 > max {
                max
            } else {
                count as u64
            };
            let first = start_index.min(total) as usize;
            let last = (start_index + effective).min(total) as usize;
            let expected = &reference[first..last];

            let result = platform
                .query_shielded_encrypted_notes_v0(
                    GetShieldedEncryptedNotesRequestV0 {
                        start_index,
                        count,
                        prove: false,
                        token_id: None,
                    },
                    &state,
                    version,
                )
                .expect("expected query to succeed");
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            match result.data.and_then(|data| data.result) {
                Some(get_shielded_encrypted_notes_response_v0::Result::EncryptedNotes(notes)) => {
                    assert_eq!(
                        notes.entries.as_slice(),
                        expected,
                        "start_index {} count {}",
                        start_index,
                        count
                    );
                }
                other => panic!("expected EncryptedNotes, got {:?}", other),
            }
        }
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
            token_id: None,
        };

        let result = platform
            .query_shielded_encrypted_notes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }
}
