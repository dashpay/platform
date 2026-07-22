use super::VerifiedShieldedEncryptedNote;
use crate::drive::shielded::paths::{
    shielded_credit_pool_path_vec, SHIELDED_NOTES_CHUNK_POWER, SHIELDED_NOTES_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{Element, GroveDb, PathQuery, Query, QueryItem, SizedQuery, SubqueryBranch};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies a `GetShieldedEncryptedNotes` proof.
    ///
    /// Returns `(root_hash, notes, total_count)` where `total_count` is the
    /// on-chain total number of notes appended to the shielded
    /// `CommitmentTree` — i.e. the denominator a wallet needs for a sync
    /// progress bar.
    ///
    /// `total_count` comes "for free" from the same proof: the note-fetch
    /// proof is a GroveDB layered proof that subqueries INTO the
    /// `CommitmentTree` element at `[ShieldedBalances, "M", [128]]`. That
    /// parent element is necessarily present in the proof (it chains to the
    /// root hash and supplies `total_count` to the inner `CommitmentTree`
    /// subquery verification). We extract it by running an additional
    /// **subset** verification against the SAME proof bytes with a
    /// single-key `PathQuery` targeting the `CommitmentTree` element itself
    /// (no subquery), then decoding `Element::CommitmentTree(total_count,
    /// ..)` — field 0. This reuses the exact `PathQuery` + decode the
    /// standalone `GetShieldedNotesCount` verifier would use, but against
    /// the note-fetch proof we already have.
    pub(super) fn verify_shielded_encrypted_notes_v0(
        proof: &[u8],
        start_index: u64,
        count: u32,
        max_elements: u32,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<VerifiedShieldedEncryptedNote>, u64), Error> {
        if max_elements == 0 {
            return Err(Error::Drive(DriveError::CorruptedElementType(
                "max_elements must be greater than zero",
            )));
        }

        // The query limit may span multiple MMR chunks, but the start only
        // needs to align to one on-chain chunk. Treating `max_elements` as
        // the alignment unit rejects valid resume points such as 2048 when
        // a request is allowed to fetch 4 × 2048 notes.
        let mmr_chunk_size = 1u64 << SHIELDED_NOTES_CHUNK_POWER;
        if !start_index.is_multiple_of(mmr_chunk_size) {
            return Err(Error::Drive(DriveError::CorruptedElementType(
                "start_index is not chunk-aligned; must be a multiple of the shielded-notes MMR chunk size",
            )));
        }

        let effective = if count == 0 || count > max_elements {
            max_elements
        } else {
            count
        };
        let limit = effective.min(u16::MAX as u32) as u16;

        // PathQuery must match the server-side proof generation exactly:
        // path = [ShieldedBalances, "M"], key = [SHIELDED_NOTES_KEY],
        // subquery = range_inclusive(start..=end)
        let end_index = start_index + limit as u64 - 1;
        let mut inner_query = Query::new();
        inner_query.insert_range_inclusive(
            start_index.to_be_bytes().to_vec()..=end_index.to_be_bytes().to_vec(),
        );

        let path_query = PathQuery {
            path: shielded_credit_pool_path_vec(),
            query: SizedQuery {
                query: Query {
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

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let mut notes = Vec::with_capacity(proved_key_values.len());
        for (_, _key, maybe_element) in proved_key_values {
            match maybe_element {
                Some(Element::Item(value, _)) => {
                    // Fixed item layout:
                    //   cmx(32) || nullifier(32) || cv_net(32) || encrypted_note(216) = 312 bytes.
                    // The `get(..)` + `try_into()` extractions reject any item whose length
                    // is not exactly that (cmx/nullifier/cv_net need >= 96 bytes; the trailing
                    // encrypted_note `try_into::<[u8; 216]>` pins the total to 312).
                    const BAD_NOTE_ITEM: &str = "shielded note item must be exactly 312 bytes (cmx 32 | nullifier 32 | cv_net 32 | encrypted_note 216)";
                    notes.push(VerifiedShieldedEncryptedNote {
                        cmx: value
                            .get(..32)
                            .and_then(|s| s.try_into().ok())
                            .ok_or_else(|| {
                                Error::Drive(DriveError::CorruptedElementType(BAD_NOTE_ITEM))
                            })?,
                        nullifier: value
                            .get(32..64)
                            .and_then(|s| s.try_into().ok())
                            .ok_or_else(|| {
                                Error::Drive(DriveError::CorruptedElementType(BAD_NOTE_ITEM))
                            })?,
                        cv_net: value
                            .get(64..96)
                            .and_then(|s| s.try_into().ok())
                            .ok_or_else(|| {
                                Error::Drive(DriveError::CorruptedElementType(BAD_NOTE_ITEM))
                            })?,
                        encrypted_note: value
                            .get(96..)
                            .and_then(|s| s.try_into().ok())
                            .ok_or_else(|| {
                                Error::Drive(DriveError::CorruptedElementType(BAD_NOTE_ITEM))
                            })?,
                    });
                }
                Some(_) => {
                    return Err(Error::Drive(DriveError::CorruptedElementType(
                        "expected Item element for encrypted note, got different element type",
                    )));
                }
                None => {
                    // Absent elements in proof results are normal (key doesn't exist)
                }
            }
        }

        // Extract the on-chain total note count from the SAME proof bytes.
        //
        // The note-fetch proof above subqueries INTO the CommitmentTree, so
        // the parent `Element::CommitmentTree(total_count, ..)` at key
        // `[SHIELDED_NOTES_KEY]` under `[ShieldedBalances, "M"]` is already
        // covered by the proof. A single-key query for that element (no
        // subquery) is a strict subset of what the note-fetch proof proves,
        // so a *subset* verification against the same bytes must succeed.
        // `verify_subset_query` is used unconditionally (not gated on
        // `verify_subset_of_proof`) because this element query is, by
        // construction, a sub-portion of the larger proof.
        let count_path_query = PathQuery {
            path: shielded_credit_pool_path_vec(),
            query: SizedQuery {
                query: Query::new_single_key(vec![SHIELDED_NOTES_KEY]),
                limit: Some(1),
                offset: None,
            },
        };

        let (count_root_hash, mut count_proved_key_values) = GroveDb::verify_subset_query(
            proof,
            &count_path_query,
            &platform_version.drive.grove_version,
        )?;

        // Both verifications run against the SAME proof bytes, so they must
        // derive the same root by construction. Make that invariant explicit:
        // a mismatch means the count sub-proof and the note-fetch proof came
        // from different state and the result cannot be trusted.
        if count_root_hash != root_hash {
            return Err(Error::Proof(ProofError::IncorrectProof(
                "shielded notes count sub-proof root mismatch".to_string(),
            )));
        }

        let total_count = match count_proved_key_values.pop() {
            Some((_, _, Some(Element::CommitmentTree(total_count, _, _)))) => total_count,
            Some((_, _, Some(_))) => {
                return Err(Error::Drive(DriveError::CorruptedElementType(
                    "expected CommitmentTree element for shielded notes count, got different element type",
                )));
            }
            // Absent parent element: an uninitialized pool would lack the
            // CommitmentTree leaf. A valid note-fetch proof against a live
            // pool always carries it; treat absence as zero notes.
            Some((_, _, None)) | None => 0,
        };

        Ok((root_hash, notes, total_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::shielded::paths::SHIELDED_NOTES_CHUNK_POWER;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use platform_version::version::PlatformVersion;

    /// Deterministic cv_net for note `i`: distinct, non-zero, and different
    /// from this note's cmx/nullifier so a round-trip mix-up is detectable.
    fn cv_net_for(i: u64) -> [u8; 32] {
        let mut cv_net = [0xEEu8; 32];
        cv_net[..8].copy_from_slice(&i.to_be_bytes());
        cv_net
    }

    /// Insert `n` distinct notes into the shielded pool's CommitmentTree.
    fn insert_notes(drive: &Drive, n: u64, platform_version: &PlatformVersion) {
        for i in 0..n {
            let mut cmx = [0u8; 32];
            cmx[..8].copy_from_slice(&i.to_be_bytes());
            let mut nullifier = [0u8; 32];
            nullifier[24..].copy_from_slice(&i.to_be_bytes());
            let nullifier = {
                // Ensure nullifiers are also distinct from cmx values.
                let mut n = nullifier;
                n[0] = 0xCC;
                n
            };

            let ops = Drive::insert_note_op(
                nullifier,
                cmx,
                cv_net_for(i),
                vec![(i % 256) as u8; 216],
                platform_version,
            )
            .expect("build note op");
            let grove_ops =
                crate::fees::op::LowLevelDriveOperation::grovedb_operations_batch_consume(ops);
            drive
                .grove_apply_batch_with_add_costs(
                    grove_ops,
                    false,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("apply note insert");
        }
    }

    /// Build the server-side `GetShieldedEncryptedNotes` PathQuery for a
    /// chunk `[start_index, start_index + limit)`. Must match
    /// `query_shielded_encrypted_notes_v0` byte-for-byte so the proof we
    /// produce is the real note-fetch proof shape.
    fn notes_fetch_path_query(start_index: u64, limit: u16) -> PathQuery {
        let end_index = start_index + limit as u64 - 1;
        let mut inner_query = Query::new();
        inner_query.insert_range_inclusive(
            start_index.to_be_bytes().to_vec()..=end_index.to_be_bytes().to_vec(),
        );

        PathQuery {
            path: shielded_credit_pool_path_vec(),
            query: SizedQuery {
                query: Query {
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
        }
    }

    /// THE make-or-break test: prove that `total_count` is extractable from
    /// the real `GetShieldedEncryptedNotes` proof.
    ///
    /// (a) insert N notes, (b) generate a real note-fetch proof via the
    /// drive prove path (`grove_get_proved_path_query_v1`, exactly what the
    /// drive-abci query handler uses), (c) run the verifier against that
    /// proof, and (d) assert the extracted `total_count == N`.
    ///
    /// If the parent CommitmentTree element were NOT in the note-fetch
    /// proof, the inner `verify_subset_query` would fail or return `None`
    /// and this would not see `total_count == N`.
    #[test]
    fn total_count_extractable_from_encrypted_notes_proof() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        const N: u64 = 5;
        insert_notes(&drive, N, platform_version);

        // Request the first chunk. max_elements mirrors the wire cap; we
        // only need a limit that covers our N notes.
        let mmr_chunk_size: u64 = 1u64 << SHIELDED_NOTES_CHUNK_POWER;
        let max_elements = mmr_chunk_size as u32;
        let start_index = 0u64;
        let limit = max_elements.min(u16::MAX as u32) as u16;

        let path_query = notes_fetch_path_query(start_index, limit);
        let proof = drive
            .grove_get_proved_path_query_v1(&path_query, &mut vec![], &platform_version.drive)
            .expect("should produce note-fetch proof");

        // verify_subset_of_proof = false: this is a self-contained proof for
        // the note-fetch PathQuery (same as the FromProof path).
        let (root_hash, notes, total_count) = Drive::verify_shielded_encrypted_notes_v0(
            proof.as_slice(),
            start_index,
            0, // count = 0 → effective = max_elements
            max_elements,
            false,
            platform_version,
        )
        .expect("should verify note-fetch proof and extract total_count");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(
            notes.len() as u64,
            N,
            "should return all {N} inserted notes"
        );
        assert_eq!(
            total_count, N,
            "extracted total_count must equal the number of notes appended on-chain"
        );

        // cv_net must round-trip end-to-end (insert → store → prove → verify).
        // Notes come back in tree order, so note at output index `i` is the
        // one inserted with `cv_net_for(i)`.
        for (i, note) in notes.iter().enumerate() {
            let expected_cv_net = cv_net_for(i as u64);
            assert_eq!(
                note.cv_net.as_slice(),
                expected_cv_net.as_slice(),
                "cv_net for note {i} must round-trip through prove→verify"
            );
            // cv_net is a distinct field, not an alias of cmx.
            assert_ne!(
                note.cv_net.as_slice(),
                note.cmx.as_slice(),
                "cv_net must be a separate field from cmx"
            );
        }
    }

    /// Same extraction works when the verifier is run in subset mode (the
    /// production `FromProof` path passes `verify_subset_of_proof = false`,
    /// but the inner count extraction always uses subset semantics — assert
    /// both outer modes yield the same `total_count`).
    #[test]
    fn total_count_stable_across_verify_modes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        const N: u64 = 3;
        insert_notes(&drive, N, platform_version);

        let mmr_chunk_size: u64 = 1u64 << SHIELDED_NOTES_CHUNK_POWER;
        let max_elements = mmr_chunk_size as u32;
        let limit = max_elements.min(u16::MAX as u32) as u16;
        let path_query = notes_fetch_path_query(0, limit);
        let proof = drive
            .grove_get_proved_path_query_v1(&path_query, &mut vec![], &platform_version.drive)
            .expect("should produce note-fetch proof");

        let (_, _, tc_full) = Drive::verify_shielded_encrypted_notes_v0(
            proof.as_slice(),
            0,
            0,
            max_elements,
            false,
            platform_version,
        )
        .expect("verify (non-subset outer)");

        let (_, _, tc_subset) = Drive::verify_shielded_encrypted_notes_v0(
            proof.as_slice(),
            0,
            0,
            max_elements,
            true,
            platform_version,
        )
        .expect("verify (subset outer)");

        assert_eq!(tc_full, N);
        assert_eq!(tc_subset, N);
    }

    /// A multi-chunk request may resume at any MMR chunk boundary, not only
    /// at a boundary of the full request size. This is the production shape
    /// when a wallet watermark in the first chunk advances to 2100: the
    /// wallet rewinds to 2048, while the query cap remains 4 × 2048.
    #[test]
    fn accepts_mmr_aligned_start_inside_multi_chunk_query_boundary() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        const N: u64 = 5;
        insert_notes(&drive, N, platform_version);

        let mmr_chunk_size: u64 = 1u64 << SHIELDED_NOTES_CHUNK_POWER;
        let max_elements = (4 * mmr_chunk_size) as u32;
        let start_index = mmr_chunk_size;
        let limit = max_elements.min(u16::MAX as u32) as u16;
        let path_query = notes_fetch_path_query(start_index, limit);
        let proof = drive
            .grove_get_proved_path_query_v1(&path_query, &mut vec![], &platform_version.drive)
            .expect("should produce note-fetch proof from the second MMR chunk");

        let (_, notes, total_count) = Drive::verify_shielded_encrypted_notes_v0(
            proof.as_slice(),
            start_index,
            0,
            max_elements,
            false,
            platform_version,
        )
        .expect("MMR-aligned resume point should verify for a multi-chunk request");

        assert!(notes.is_empty());
        assert_eq!(total_count, N);
    }
}
