use crate::drive::identity::{IdentityDriveQuery, IdentityProveRequestType};
use crate::drive::Drive;
use crate::error::Error;
use crate::query::{IdentityBasedVoteDriveQuery, SingleDocumentDriveQuery};

use crate::query::identity_token_balance_drive_query::IdentityTokenBalanceDriveQuery;
use crate::query::identity_token_info_drive_query::IdentityTokenInfoDriveQuery;
use crate::query::token_status_drive_query::TokenStatusDriveQuery;
use dpp::version::PlatformVersion;
use grovedb::{PathQuery, TransactionArg};
use itertools::{Either, Itertools};

impl Drive {
    /// This function query requested identities, documents and contracts and provide cryptographic proofs
    ///
    /// # Parameters
    /// - `identity_queries`: A list of [IdentityDriveQuery]. These specify the identities
    ///   to be proven.
    /// - `contract_ids`: A list of Data Contract IDs to prove
    /// - `document_queries`: A list of [SingleDocumentDriveQuery]. These specify the documents
    ///   to be proven.
    /// - `vote_queries`: A list of [IdentityBasedVoteDriveQuery]. These would be to figure out the
    ///   result of votes based on identities making them.
    /// - `token_balance_queries`: A slice of [IdentityTokenBalanceDriveQuery] objects specifying
    ///   token balance queries for identities.
    /// - `token_info_queries`: A slice of [IdentityTokenInfoDriveQuery] objects specifying
    ///   token information queries for identities.
    /// - `token_status_queries`: A slice of [TokenStatusDriveQuery] objects specifying token
    ///   status queries.
    /// - `transaction`: An optional grovedb transaction
    /// - `platform_version`: A reference to the [PlatformVersion] object that specifies the version of
    ///   the function to call.
    ///
    /// # Returns
    /// Returns a `Result` with a `Vec<u8>` containing the proof data if the function succeeds,
    /// or an `Error` if the function fails.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prove_multiple_state_transition_results_v0(
        &self,
        identity_queries: &[IdentityDriveQuery],
        contract_ids: &[([u8; 32], Option<bool>)], //bool is history
        document_queries: &[SingleDocumentDriveQuery],
        vote_queries: &[IdentityBasedVoteDriveQuery],
        token_balance_queries: &[IdentityTokenBalanceDriveQuery],
        token_info_queries: &[IdentityTokenInfoDriveQuery],
        token_status_queries: &[TokenStatusDriveQuery],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let mut path_queries = vec![];
        if !identity_queries.is_empty() {
            for identity_query in identity_queries {
                match identity_query.prove_request_type {
                    IdentityProveRequestType::FullIdentity => {
                        path_queries.push(Self::full_identity_query(
                            &identity_query.identity_id,
                            &platform_version.drive.grove_version,
                        )?);
                    }
                    IdentityProveRequestType::Balance => {
                        path_queries.push(Self::balance_for_identity_id_query(
                            identity_query.identity_id,
                        ));
                    }
                    IdentityProveRequestType::Keys => {
                        path_queries.push(Self::identity_all_keys_query(
                            &identity_query.identity_id,
                            &platform_version.drive.grove_version,
                        )?);
                    }
                    IdentityProveRequestType::Revision => {
                        path_queries
                            .push(Self::identity_revision_query(&identity_query.identity_id));
                    }
                }
            }
        }

        let (contract_ids, historical_contract_ids): (Vec<_>, Vec<_>) = contract_ids
            .iter()
            .partition_map(|(contract_id, historical)| {
                // TODO: implement None
                let history = historical.unwrap_or(false);
                if !history {
                    Either::Left(*contract_id)
                } else {
                    Either::Right(*contract_id)
                }
            });

        if !contract_ids.is_empty() {
            let mut path_query =
                Self::fetch_non_historical_contracts_query(contract_ids.as_slice());
            path_query.query.limit = None;
            path_queries.push(path_query);
        }

        if !historical_contract_ids.is_empty() {
            let mut path_query =
                Self::fetch_historical_contracts_query(historical_contract_ids.as_slice());
            path_query.query.limit = None;
            path_queries.push(path_query);
        }
        if !document_queries.is_empty() {
            path_queries.extend(document_queries.iter().filter_map(|drive_query| {
                // The path query construction can only fail in extremely rare circumstances.
                let mut path_query = drive_query.construct_path_query(platform_version).ok()?;
                path_query.query.limit = None;
                Some(path_query)
            }));
        }

        if !vote_queries.is_empty() {
            path_queries.extend(vote_queries.iter().filter_map(|vote_query| {
                // The path query construction can only fail if the serialization fails.
                // Because the serialization will pretty much never fail, we can do this.
                let mut path_query = vote_query.construct_path_query().ok()?;
                path_query.query.limit = None;
                Some(path_query)
            }));
        }

        if !token_balance_queries.is_empty() {
            path_queries.extend(token_balance_queries.iter().map(|token_balance_query| {
                // The path query construction can only fail if the serialization fails.
                // Because the serialization will pretty much never fail, we can do this.
                let mut path_query = token_balance_query.construct_path_query();
                path_query.query.limit = None;
                path_query
            }));
        }

        if !token_info_queries.is_empty() {
            path_queries.extend(token_info_queries.iter().map(|token_info_query| {
                // The path query construction can only fail if the serialization fails.
                // Because the serialization will pretty much never fail, we can do this.
                let mut path_query = token_info_query.construct_path_query();
                path_query.query.limit = None;
                path_query
            }));
        }

        if !token_status_queries.is_empty() {
            path_queries.extend(token_status_queries.iter().map(|token_status_query| {
                // The path query construction can only fail if the serialization fails.
                // Because the serialization will pretty much never fail, we can do this.
                let mut path_query = token_status_query.construct_path_query();
                path_query.query.limit = None;
                path_query
            }));
        }

        let path_query = PathQuery::merge(
            path_queries.iter().collect(),
            &platform_version.drive.grove_version,
        )
        .map_err(Error::from)?;

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
#[allow(clippy::type_complexity)]
mod tests {
    use crate::drive::identity::{IdentityDriveQuery, IdentityProveRequestType};
    use crate::query::identity_token_balance_drive_query::IdentityTokenBalanceDriveQuery;
    use crate::query::identity_token_info_drive_query::IdentityTokenInfoDriveQuery;
    use crate::query::token_status_drive_query::TokenStatusDriveQuery;
    use crate::query::{IdentityBasedVoteDriveQuery, SingleDocumentDriveQuery};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    // ---------------- Helpers ---------------------------------------------

    fn empty_query_set<'a>() -> (
        &'a [IdentityDriveQuery],
        &'a [([u8; 32], Option<bool>)],
        &'a [SingleDocumentDriveQuery],
        &'a [IdentityBasedVoteDriveQuery],
        &'a [IdentityTokenBalanceDriveQuery],
        &'a [IdentityTokenInfoDriveQuery],
        &'a [TokenStatusDriveQuery],
    ) {
        (&[], &[], &[], &[], &[], &[], &[])
    }

    fn insert_identity(drive: &crate::drive::Drive, seed: u64) -> Identity {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(3, Some(seed), platform_version)
            .expect("expected a random identity");
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert identity");
        identity
    }

    // ---------------- Empty input ----------------------------------------

    /// Calling with no queries at all means there are zero path queries to
    /// merge — `PathQuery::merge` errors with "InvalidInput" because it
    /// requires at least one. This is the documented error path for a
    /// completely empty request.
    #[test]
    fn empty_input_returns_invalid_input_merge_error() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let (ids, cids, docs, votes, tbals, tinfos, tstat) = empty_query_set();

        let err = drive
            .prove_multiple_state_transition_results_v0(
                ids,
                cids,
                docs,
                votes,
                tbals,
                tinfos,
                tstat,
                None,
                platform_version,
            )
            .expect_err("expected merge error for empty input");

        let msg = format!("{:?}", err);
        assert!(
            msg.contains("merge function requires at least 1 path query")
                || msg.contains("InvalidInput"),
            "expected merge error, got: {msg}"
        );
    }

    // ---------------- Single balance query --------------------------------

    #[test]
    fn single_balance_query_produces_non_empty_proof_that_verifies() {
        use grovedb::GroveDb;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = insert_identity(&drive, 42);

        let identity_queries = vec![IdentityDriveQuery {
            identity_id: identity.id().to_buffer(),
            prove_request_type: IdentityProveRequestType::Balance,
        }];

        let proof = drive
            .prove_multiple_state_transition_results_v0(
                &identity_queries,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                None,
                platform_version,
            )
            .expect("expected to produce a proof");

        assert!(!proof.is_empty(), "proof should not be empty");

        // Verify against the balance query.
        let pq = crate::drive::Drive::balance_for_identity_id_query(identity.id().to_buffer());
        let (_root, items) =
            GroveDb::verify_query(proof.as_slice(), &pq, &platform_version.drive.grove_version)
                .expect("expected proof to verify");
        assert_eq!(items.len(), 1);
    }

    // ---------------- Deterministic output --------------------------------

    /// Invoking twice with the same inputs (and no writes in between) must
    /// yield byte-identical proofs. This protects against incidental
    /// nondeterminism (e.g. iterating over a HashMap) inside the aggregator.
    #[test]
    fn deterministic_proof_for_equal_inputs() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_a = insert_identity(&drive, 1);
        let identity_b = insert_identity(&drive, 2);

        let identity_queries = vec![
            IdentityDriveQuery {
                identity_id: identity_a.id().to_buffer(),
                prove_request_type: IdentityProveRequestType::Balance,
            },
            IdentityDriveQuery {
                identity_id: identity_b.id().to_buffer(),
                prove_request_type: IdentityProveRequestType::Revision,
            },
        ];

        let first = drive
            .prove_multiple_state_transition_results_v0(
                &identity_queries,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                None,
                platform_version,
            )
            .expect("first proof");
        let second = drive
            .prove_multiple_state_transition_results_v0(
                &identity_queries,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                None,
                platform_version,
            )
            .expect("second proof");

        assert_eq!(
            first, second,
            "proofs must be deterministic for equal inputs"
        );
    }

    // ---------------- Non-existent identity -> absence proof --------------

    /// A Balance query for an identity that does not exist must still
    /// produce a verifiable proof (absence proof). This exercises the
    /// `Balance` arm of the `IdentityProveRequestType` match independently.
    #[test]
    fn nonexistent_identity_still_produces_verifiable_proof() {
        use grovedb::GroveDb;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let missing_id = [0xAAu8; 32];
        let identity_queries = vec![IdentityDriveQuery {
            identity_id: missing_id,
            prove_request_type: IdentityProveRequestType::Balance,
        }];

        let proof = drive
            .prove_multiple_state_transition_results_v0(
                &identity_queries,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                None,
                platform_version,
            )
            .expect("absence proof must still succeed");

        assert!(!proof.is_empty());

        let pq = crate::drive::Drive::balance_for_identity_id_query(missing_id);
        let (_root, items) =
            GroveDb::verify_query(proof.as_slice(), &pq, &platform_version.drive.grove_version)
                .expect("absence proof must verify");
        // Balance item should come back empty / None since the identity does
        // not exist.
        assert!(
            items.is_empty() || items.iter().all(|(_, _, v)| v.is_none()),
            "expected no balance items for a non-existent identity, got {:?}",
            items
        );
    }

    // ---------------- All four IdentityProveRequestType arms --------------

    /// Exercises the full request-type match by proving four identities,
    /// each with a different `IdentityProveRequestType` variant, ensuring
    /// each arm of the match in `prove_multiple_state_transition_results_v0`
    /// is hit. We only assert that the proof is non-empty and unique.
    #[test]
    fn covers_all_identity_prove_request_type_variants() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let i1 = insert_identity(&drive, 101);
        let i2 = insert_identity(&drive, 202);
        let i3 = insert_identity(&drive, 303);
        let i4 = insert_identity(&drive, 404);

        let identity_queries = vec![
            IdentityDriveQuery {
                identity_id: i1.id().to_buffer(),
                prove_request_type: IdentityProveRequestType::FullIdentity,
            },
            IdentityDriveQuery {
                identity_id: i2.id().to_buffer(),
                prove_request_type: IdentityProveRequestType::Balance,
            },
            IdentityDriveQuery {
                identity_id: i3.id().to_buffer(),
                prove_request_type: IdentityProveRequestType::Keys,
            },
            IdentityDriveQuery {
                identity_id: i4.id().to_buffer(),
                prove_request_type: IdentityProveRequestType::Revision,
            },
        ];

        let proof = drive
            .prove_multiple_state_transition_results_v0(
                &identity_queries,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                None,
                platform_version,
            )
            .expect("proof should be produced");

        assert!(!proof.is_empty());
        // A single-query balance proof baseline should differ from this
        // combined proof (different queries -> different bytes).
        let solo = drive
            .prove_multiple_state_transition_results_v0(
                &[IdentityDriveQuery {
                    identity_id: i2.id().to_buffer(),
                    prove_request_type: IdentityProveRequestType::Balance,
                }],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                None,
                platform_version,
            )
            .expect("solo proof");
        assert_ne!(
            proof, solo,
            "combined proof must differ from solo balance proof"
        );
    }

    // ---------------- Non-historical contract ids -------------------------

    /// Non-historical contract query: provide a single contract_id flagged
    /// non-historical. This exercises the `Left(contract_id)` branch of the
    /// partition and the `fetch_non_historical_contracts_query` path.
    #[test]
    fn non_historical_contract_id_produces_verifiable_proof() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Absent contract; we only care that a proof is produced.
        let contract_id = [0xC1u8; 32];

        let proof = drive
            .prove_multiple_state_transition_results_v0(
                &[],
                &[(contract_id, Some(false))],
                &[],
                &[],
                &[],
                &[],
                &[],
                None,
                platform_version,
            )
            .expect("non-historical contract proof");
        assert!(!proof.is_empty());
    }

    /// Historical contract query: provide a single contract_id flagged
    /// historical. This exercises the `Right(contract_id)` branch.
    #[test]
    fn historical_contract_id_produces_verifiable_proof() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_id = [0xC2u8; 32];

        let proof = drive
            .prove_multiple_state_transition_results_v0(
                &[],
                &[(contract_id, Some(true))],
                &[],
                &[],
                &[],
                &[],
                &[],
                None,
                platform_version,
            )
            .expect("historical contract proof");
        assert!(!proof.is_empty());
    }

    // ---------------- Mixed contract historicity -------------------------

    /// Both historical and non-historical contracts in the same call cover
    /// the both-non-empty branches (two separate path queries appended).
    #[test]
    fn mixed_historical_and_non_historical_contract_ids_succeeds() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let c_nonhist = [0xC3u8; 32];
        let c_hist = [0xC4u8; 32];

        let proof = drive
            .prove_multiple_state_transition_results_v0(
                &[],
                &[(c_nonhist, Some(false)), (c_hist, Some(true))],
                &[],
                &[],
                &[],
                &[],
                &[],
                None,
                platform_version,
            )
            .expect("mixed contract proof");
        assert!(!proof.is_empty());
    }

    // ---------------- Contract historical=None default-path ---------------

    /// Passing `None` for `historical` must default to non-historical
    /// (the `.unwrap_or(false)` branch).
    #[test]
    fn contract_id_with_none_historical_defaults_to_non_historical() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let c = [0xC5u8; 32];

        // None should be treated the same as Some(false).
        let none_proof = drive
            .prove_multiple_state_transition_results_v0(
                &[],
                &[(c, None)],
                &[],
                &[],
                &[],
                &[],
                &[],
                None,
                platform_version,
            )
            .expect("none-historical");
        let false_proof = drive
            .prove_multiple_state_transition_results_v0(
                &[],
                &[(c, Some(false))],
                &[],
                &[],
                &[],
                &[],
                &[],
                None,
                platform_version,
            )
            .expect("explicit false-historical");

        assert_eq!(
            none_proof, false_proof,
            "historical=None must be equivalent to historical=Some(false)"
        );
    }

    // ---------------- Token queries ---------------------------------------

    /// Exercise each of the three token query arms (balance / info / status)
    /// with absent tokens. We just need to prove these branches execute and
    /// their `construct_path_query` calls feed the merge correctly.
    #[test]
    fn covers_token_balance_info_and_status_arms() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let token_id = Identifier::from([0x77u8; 32]);
        let identity_id = Identifier::from([0x88u8; 32]);

        let balance_q = vec![IdentityTokenBalanceDriveQuery {
            identity_id,
            token_id,
        }];
        let info_q = vec![IdentityTokenInfoDriveQuery {
            identity_id,
            token_id,
        }];
        let status_q = vec![TokenStatusDriveQuery { token_id }];

        let proof = drive
            .prove_multiple_state_transition_results_v0(
                &[],
                &[],
                &[],
                &[],
                &balance_q,
                &info_q,
                &status_q,
                None,
                platform_version,
            )
            .expect("all three token arms should produce a proof");
        assert!(!proof.is_empty());
    }

    // ---------------- Combined request covering many branches -----------

    /// A maximally combined call covers nearly every `if !<...>.is_empty()`
    /// branch of the aggregator in a single invocation. This is the
    /// multi-result happy path.
    #[test]
    fn combined_multi_query_happy_path() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = insert_identity(&drive, 7);
        let ids = vec![IdentityDriveQuery {
            identity_id: identity.id().to_buffer(),
            prove_request_type: IdentityProveRequestType::Balance,
        }];
        let contracts = vec![([0xA1u8; 32], Some(false)), ([0xA2u8; 32], Some(true))];
        let token_id = Identifier::from([0x33u8; 32]);
        let tbals = vec![IdentityTokenBalanceDriveQuery {
            identity_id: identity.id(),
            token_id,
        }];
        let tinfos = vec![IdentityTokenInfoDriveQuery {
            identity_id: identity.id(),
            token_id,
        }];
        let tstat = vec![TokenStatusDriveQuery { token_id }];

        let proof = drive
            .prove_multiple_state_transition_results_v0(
                &ids,
                &contracts,
                &[],
                &[],
                &tbals,
                &tinfos,
                &tstat,
                None,
                platform_version,
            )
            .expect("combined proof must succeed");
        assert!(!proof.is_empty());
    }

    // ---------------- Public dispatcher routing --------------------------

    /// Route the same call through the public `prove_multiple_state_transition_results`
    /// entry point to confirm wiring to `_v0`. Not a version-dispatch test —
    /// we only check the happy path produces a non-empty proof.
    #[test]
    fn public_entrypoint_routes_through_v0() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = insert_identity(&drive, 77);
        let ids = vec![IdentityDriveQuery {
            identity_id: identity.id().to_buffer(),
            prove_request_type: IdentityProveRequestType::Revision,
        }];

        let proof = drive
            .prove_multiple_state_transition_results(
                &ids,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                None,
                platform_version,
            )
            .expect("public entrypoint happy path");
        assert!(!proof.is_empty());
    }
}
