mod anchors;
mod encrypted_notes;
mod most_recent_anchor;
mod notes_count;
mod nullifiers;
mod pool_state;

use crate::error::query::QueryError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::feature_initial_protocol_versions::TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_credit_pool_anchors_path_vec, shielded_credit_pool_nullifiers_path_vec,
    shielded_credit_pool_path_vec, shielded_latest_recorded_anchor_path_query,
    token_shielded_pool_anchors_path_vec, token_shielded_pool_latest_recorded_anchor_path_query,
    token_shielded_pool_nullifiers_path_vec, token_shielded_pool_path_vec,
};
use drive::drive::tokens::paths::token_shielded_pools_root_path;
use drive::drive::Drive;
use drive::grovedb::PathQuery;
use drive::grovedb_path::SubtreePath;
use drive::util::grove_operations::DirectQueryType;

/// Which shielded pool a shielded query targets: the credit pool (no `token_id` in the request)
/// or one token's pool. Every pool has the same subtree layout, so the handlers only differ in
/// the paths the selector hands them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShieldedPoolSelector {
    /// The main credit shielded pool.
    Credit,
    /// The shielded pool of the token with this id.
    Token([u8; 32]),
}

impl ShieldedPoolSelector {
    /// Resolves the optional `token_id` a request carries. A token pool is only addressable
    /// once token shielded pools exist (protocol version 14), and the id must be 32 bytes.
    pub(super) fn from_request(
        token_id: Option<Vec<u8>>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, QueryError> {
        match token_id {
            None => Ok(ShieldedPoolSelector::Credit),
            Some(token_id) => {
                if platform_version.protocol_version < TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION
                {
                    return Err(QueryError::InvalidArgument(format!(
                        "token shielded pools are not active before protocol version {}",
                        TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION
                    )));
                }
                let token_id: [u8; 32] = token_id.try_into().map_err(|token_id: Vec<u8>| {
                    QueryError::InvalidArgument(format!(
                        "token_id must be 32 bytes, got {}",
                        token_id.len()
                    ))
                })?;
                Ok(ShieldedPoolSelector::Token(token_id))
            }
        }
    }

    /// Confirms the chain holds the pool this selector names, so that an unproved read of it has
    /// something to read.
    ///
    /// A token pool only exists once a token that opted into one was registered, and any 32 bytes
    /// an unauthenticated caller makes up name a pool that was never created. Its subtree is
    /// absent, which no read underneath can tell apart from a pool that exists and is empty: a
    /// read of the notes tree fails inside GroveDB as a missing path, and a range or membership
    /// read of the anchors, nullifier or balance keys comes back empty, which would report a
    /// nullifier in a pool that does not exist as unspent. "This pool holds no such entry" and
    /// "there is no such pool" are different facts, so the second is refused here rather than
    /// answered as the first.
    ///
    /// Only the unproved reads need this. A proof over a pool that does not exist is a proof of
    /// its absence, which is an answer the client can check against the root hash rather than has
    /// to trust, so the proved reads keep handing one out. Separating the two facts is then the
    /// verifier's job, and a proof that descends through the pool carries what it needs: the pools
    /// tree's own Merk proof for the pool's key. `Drive::verify_token_shielded_pool_nullifiers`
    /// reads that key and refuses a spend status for a pool the chain does not hold, because
    /// `is_spent: false` is the one absence here that a caller acts on. The other proved reads
    /// return `None` for a missing pool, where an empty one gives `Some(0)`, so a caller that
    /// distinguishes the two can; nothing here establishes that one does.
    pub(super) fn validate_pool_exists(
        &self,
        drive: &Drive,
        platform_version: &PlatformVersion,
    ) -> Result<Result<(), QueryError>, Error> {
        match self {
            // The credit pool is not named by the client, so there is nothing here to refuse: a
            // caller cannot ask for a credit pool that was never created the way it can ask for a
            // token's. Versions that serve these queries before the credit pool exists read it as
            // empty, which is what they did before this check was added.
            ShieldedPoolSelector::Credit => Ok(Ok(())),
            ShieldedPoolSelector::Token(token_id) => {
                let pools_root = token_shielded_pools_root_path();
                let pool_exists = drive.grove_has_raw(
                    SubtreePath::from(&pools_root),
                    token_id,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )?;

                if pool_exists {
                    Ok(Ok(()))
                } else {
                    Ok(Err(QueryError::NotFound(format!(
                        "shielded pool for token {} not found",
                        Identifier::new(*token_id)
                    ))))
                }
            }
        }
    }

    /// The pool subtree path.
    pub(super) fn pool_path_vec(&self) -> Vec<Vec<u8>> {
        match self {
            ShieldedPoolSelector::Credit => shielded_credit_pool_path_vec(),
            ShieldedPoolSelector::Token(token_id) => token_shielded_pool_path_vec(*token_id),
        }
    }

    /// The pool's nullifiers tree path.
    pub(super) fn nullifiers_path_vec(&self) -> Vec<Vec<u8>> {
        match self {
            ShieldedPoolSelector::Credit => shielded_credit_pool_nullifiers_path_vec(),
            ShieldedPoolSelector::Token(token_id) => {
                token_shielded_pool_nullifiers_path_vec(*token_id)
            }
        }
    }

    /// The pool's anchors tree path.
    pub(super) fn anchors_path_vec(&self) -> Vec<Vec<u8>> {
        match self {
            ShieldedPoolSelector::Credit => shielded_credit_pool_anchors_path_vec(),
            ShieldedPoolSelector::Token(token_id) => {
                token_shielded_pool_anchors_path_vec(*token_id)
            }
        }
    }

    /// The pool's canonical most-recent-anchor query.
    pub(super) fn latest_recorded_anchor_path_query(&self) -> PathQuery {
        match self {
            ShieldedPoolSelector::Credit => shielded_latest_recorded_anchor_path_query(),
            ShieldedPoolSelector::Token(token_id) => {
                token_shielded_pool_latest_recorded_anchor_path_query(*token_id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::platform_state::PlatformState;
    use crate::query::tests::setup_platform;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dapi_grpc::platform::v0::get_most_recent_shielded_anchor_request::{
        GetMostRecentShieldedAnchorRequestV0, Version as MostRecentAnchorRequestVersion,
    };
    use dapi_grpc::platform::v0::get_most_recent_shielded_anchor_response::{
        get_most_recent_shielded_anchor_response_v0, Version as MostRecentAnchorResponseVersion,
    };
    use dapi_grpc::platform::v0::get_shielded_anchors_request::{
        GetShieldedAnchorsRequestV0, Version as AnchorsRequestVersion,
    };
    use dapi_grpc::platform::v0::get_shielded_anchors_response::{
        get_shielded_anchors_response_v0, Version as AnchorsResponseVersion,
    };
    use dapi_grpc::platform::v0::get_shielded_encrypted_notes_request::{
        GetShieldedEncryptedNotesRequestV0, Version as EncryptedNotesRequestVersion,
    };
    use dapi_grpc::platform::v0::get_shielded_encrypted_notes_response::{
        get_shielded_encrypted_notes_response_v0, Version as EncryptedNotesResponseVersion,
    };
    use dapi_grpc::platform::v0::get_shielded_notes_count_request::{
        GetShieldedNotesCountRequestV0, Version as NotesCountRequestVersion,
    };
    use dapi_grpc::platform::v0::get_shielded_notes_count_response::{
        get_shielded_notes_count_response_v0, Version as NotesCountResponseVersion,
    };
    use dapi_grpc::platform::v0::get_shielded_nullifiers_request::{
        GetShieldedNullifiersRequestV0, Version as NullifiersRequestVersion,
    };
    use dapi_grpc::platform::v0::get_shielded_nullifiers_response::{
        get_shielded_nullifiers_response_v0, Version as NullifiersResponseVersion,
    };
    use dapi_grpc::platform::v0::get_shielded_pool_state_request::{
        GetShieldedPoolStateRequestV0, Version as PoolStateRequestVersion,
    };
    use dapi_grpc::platform::v0::get_shielded_pool_state_response::{
        get_shielded_pool_state_response_v0, Version as PoolStateResponseVersion,
    };
    use dapi_grpc::platform::v0::{
        GetMostRecentShieldedAnchorRequest, GetShieldedAnchorsRequest,
        GetShieldedEncryptedNotesRequest, GetShieldedNotesCountRequest,
        GetShieldedNullifiersRequest, GetShieldedPoolStateRequest,
    };
    use dpp::dashcore::Network;
    use std::sync::Arc;

    /// A token id the chain holds no shielded pool for. Any 32 bytes an unauthenticated caller
    /// makes up land here.
    const POOL_LESS_TOKEN_ID: [u8; 32] = [0xAB; 32];

    /// The token id of the pool the fixtures create.
    const POOLED_TOKEN_ID: [u8; 32] = [3; 32];

    /// A platform holding one token shielded pool, so that a pool the chain has can be told
    /// apart from one it does not.
    fn setup_platform_with_a_token_pool<'a>() -> (
        TempPlatform<MockCoreRPCLike>,
        Arc<PlatformState>,
        &'a PlatformVersion,
    ) {
        let (platform, state, platform_version) = setup_platform(None, Network::Testnet, None);

        let operations = platform
            .drive
            .create_token_shielded_pool_trees_operations(
                POOLED_TOKEN_ID,
                false,
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the pool tree operations");
        platform
            .drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the token shielded pool to be created");

        (platform, state, platform_version)
    }

    fn pool_not_found_message() -> String {
        format!(
            "shielded pool for token {} not found",
            Identifier::new(POOL_LESS_TOKEN_ID)
        )
    }

    fn assert_pool_not_found(endpoint: &str, errors: &[QueryError]) {
        let expected = pool_not_found_message();
        match errors {
            [QueryError::NotFound(message)] if message == &expected => {}
            other => panic!("{endpoint}: expected {expected:?}, got {other:?}"),
        }
    }

    /// Every unproved shielded read of a pool the chain does not have answers `NotFound`, the
    /// coded client answer the rest of this surface gives for a missing identity or contract.
    ///
    /// Two of these reads used to fail inside GroveDB as a missing path, which reaches the client
    /// as gRPC `Internal` and writes a `tracing::error!` per request — an unauthenticated caller
    /// choosing 32 arbitrary bytes could fill a node's log with them, and an honest client was
    /// told its own request was the node's fault.
    #[test]
    fn unproved_queries_answer_not_found_for_a_pool_the_chain_does_not_have() {
        let (platform, state, version) = setup_platform_with_a_token_pool();
        let token_id = Some(POOL_LESS_TOKEN_ID.to_vec());

        // Collected rather than asserted one at a time: the six answered a missing pool in three
        // different wrong ways, and a failure that stops at the first endpoint hides the rest.
        let outcomes: Vec<(&str, Result<Vec<QueryError>, Error>)> = vec![
            (
                "notes_count",
                platform
                    .query_shielded_notes_count(
                        GetShieldedNotesCountRequest {
                            version: Some(NotesCountRequestVersion::V0(
                                GetShieldedNotesCountRequestV0 {
                                    prove: false,
                                    token_id: token_id.clone(),
                                },
                            )),
                        },
                        &state,
                        version,
                    )
                    .map(|result| result.errors),
            ),
            (
                "encrypted_notes",
                platform
                    .query_shielded_encrypted_notes(
                        GetShieldedEncryptedNotesRequest {
                            version: Some(EncryptedNotesRequestVersion::V0(
                                GetShieldedEncryptedNotesRequestV0 {
                                    start_index: 0,
                                    count: 1,
                                    prove: false,
                                    token_id: token_id.clone(),
                                },
                            )),
                        },
                        &state,
                        version,
                    )
                    .map(|result| result.errors),
            ),
            (
                "anchors",
                platform
                    .query_shielded_anchors(
                        GetShieldedAnchorsRequest {
                            version: Some(AnchorsRequestVersion::V0(GetShieldedAnchorsRequestV0 {
                                prove: false,
                                token_id: token_id.clone(),
                            })),
                        },
                        &state,
                        version,
                    )
                    .map(|result| result.errors),
            ),
            (
                "most_recent_anchor",
                platform
                    .query_most_recent_shielded_anchor(
                        GetMostRecentShieldedAnchorRequest {
                            version: Some(MostRecentAnchorRequestVersion::V0(
                                GetMostRecentShieldedAnchorRequestV0 {
                                    prove: false,
                                    token_id: token_id.clone(),
                                },
                            )),
                        },
                        &state,
                        version,
                    )
                    .map(|result| result.errors),
            ),
            (
                "nullifiers",
                platform
                    .query_shielded_nullifiers(
                        GetShieldedNullifiersRequest {
                            version: Some(NullifiersRequestVersion::V0(
                                GetShieldedNullifiersRequestV0 {
                                    nullifiers: vec![vec![0x11; 32]],
                                    prove: false,
                                    token_id: token_id.clone(),
                                },
                            )),
                        },
                        &state,
                        version,
                    )
                    .map(|result| result.errors),
            ),
            (
                "pool_state",
                platform
                    .query_shielded_pool_state(
                        GetShieldedPoolStateRequest {
                            version: Some(PoolStateRequestVersion::V0(
                                GetShieldedPoolStateRequestV0 {
                                    prove: false,
                                    token_id,
                                },
                            )),
                        },
                        &state,
                        version,
                    )
                    .map(|result| result.errors),
            ),
        ];

        let expected = pool_not_found_message();
        let mut wrong = Vec::new();
        for (endpoint, outcome) in outcomes {
            match outcome {
                Ok(errors) => match errors.as_slice() {
                    [QueryError::NotFound(message)] if message == &expected => {}
                    other => wrong.push(format!("{endpoint}: answered with {other:?}")),
                },
                Err(error) => wrong.push(format!("{endpoint}: failed the request with {error}")),
            }
        }

        assert!(
            wrong.is_empty(),
            "every unproved shielded read must answer {expected:?}:\n{}",
            wrong.join("\n")
        );
    }

    /// A nullifier is never reported unspent for a pool the chain does not have.
    ///
    /// `is_spent: false` is the answer a wallet leans on to decide a note is still spendable.
    /// The spent set of a pool that was never created is a tree that does not exist, and the
    /// lookup reads that as "no such key" — indistinguishable from an unspent nullifier in a
    /// real pool. The caller must be told the pool is unknown instead, because "this pool holds
    /// no record of that nullifier" and "there is no such pool" are different facts.
    #[test]
    fn a_nullifier_is_never_reported_unspent_for_a_pool_the_chain_does_not_have() {
        let (platform, state, version) = setup_platform_with_a_token_pool();

        let result = platform
            .query_shielded_nullifiers(
                GetShieldedNullifiersRequest {
                    version: Some(NullifiersRequestVersion::V0(
                        GetShieldedNullifiersRequestV0 {
                            nullifiers: vec![vec![0x11; 32]],
                            prove: false,
                            token_id: Some(POOL_LESS_TOKEN_ID.to_vec()),
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");

        assert_pool_not_found("nullifiers", &result.errors);
        assert!(
            result.data.is_none(),
            "a spend status must not be handed back alongside the refusal: {:?}",
            result.data
        );
    }

    /// An empty pool and a pool that does not exist are different facts, and the unproved reads
    /// must not answer the same way for both. Every value asserted here is one that a pool the
    /// chain does not have used to return too.
    #[test]
    fn unproved_queries_tell_an_empty_pool_apart_from_a_pool_that_does_not_exist() {
        let (platform, state, version) = setup_platform_with_a_token_pool();
        let token_id = Some(POOLED_TOKEN_ID.to_vec());

        let notes_count = platform
            .query_shielded_notes_count(
                GetShieldedNotesCountRequest {
                    version: Some(NotesCountRequestVersion::V0(
                        GetShieldedNotesCountRequestV0 {
                            prove: false,
                            token_id: token_id.clone(),
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(notes_count.errors.is_empty(), "{:?}", notes_count.errors);
        match notes_count.data.and_then(|response| response.version) {
            Some(NotesCountResponseVersion::V0(v0)) => assert!(matches!(
                v0.result,
                Some(get_shielded_notes_count_response_v0::Result::TotalNotesCount(0))
            )),
            other => panic!("expected a notes count, got {other:?}"),
        }

        let encrypted_notes = platform
            .query_shielded_encrypted_notes(
                GetShieldedEncryptedNotesRequest {
                    version: Some(EncryptedNotesRequestVersion::V0(
                        GetShieldedEncryptedNotesRequestV0 {
                            start_index: 0,
                            count: 1,
                            prove: false,
                            token_id: token_id.clone(),
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(
            encrypted_notes.errors.is_empty(),
            "{:?}",
            encrypted_notes.errors
        );
        match encrypted_notes.data.and_then(|response| response.version) {
            Some(EncryptedNotesResponseVersion::V0(v0)) => match v0.result {
                Some(get_shielded_encrypted_notes_response_v0::Result::EncryptedNotes(notes)) => {
                    assert!(notes.entries.is_empty())
                }
                other => panic!("expected encrypted notes, got {other:?}"),
            },
            other => panic!("expected a v0 response, got {other:?}"),
        }

        let anchors = platform
            .query_shielded_anchors(
                GetShieldedAnchorsRequest {
                    version: Some(AnchorsRequestVersion::V0(GetShieldedAnchorsRequestV0 {
                        prove: false,
                        token_id: token_id.clone(),
                    })),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(anchors.errors.is_empty(), "{:?}", anchors.errors);
        match anchors.data.and_then(|response| response.version) {
            Some(AnchorsResponseVersion::V0(v0)) => match v0.result {
                Some(get_shielded_anchors_response_v0::Result::Anchors(anchors)) => {
                    assert!(anchors.anchors.is_empty())
                }
                other => panic!("expected anchors, got {other:?}"),
            },
            other => panic!("expected a v0 response, got {other:?}"),
        }

        let most_recent_anchor = platform
            .query_most_recent_shielded_anchor(
                GetMostRecentShieldedAnchorRequest {
                    version: Some(MostRecentAnchorRequestVersion::V0(
                        GetMostRecentShieldedAnchorRequestV0 {
                            prove: false,
                            token_id: token_id.clone(),
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(
            most_recent_anchor.errors.is_empty(),
            "{:?}",
            most_recent_anchor.errors
        );
        match most_recent_anchor
            .data
            .and_then(|response| response.version)
        {
            Some(MostRecentAnchorResponseVersion::V0(v0)) => match v0.result {
                // The empty-index sentinel this endpoint documents.
                Some(get_most_recent_shielded_anchor_response_v0::Result::Anchor(anchor)) => {
                    assert_eq!(anchor, vec![0; 32])
                }
                other => panic!("expected an anchor, got {other:?}"),
            },
            other => panic!("expected a v0 response, got {other:?}"),
        }

        let nullifiers = platform
            .query_shielded_nullifiers(
                GetShieldedNullifiersRequest {
                    version: Some(NullifiersRequestVersion::V0(
                        GetShieldedNullifiersRequestV0 {
                            nullifiers: vec![vec![0x11; 32]],
                            prove: false,
                            token_id: token_id.clone(),
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(nullifiers.errors.is_empty(), "{:?}", nullifiers.errors);
        match nullifiers.data.and_then(|response| response.version) {
            Some(NullifiersResponseVersion::V0(v0)) => match v0.result {
                Some(get_shielded_nullifiers_response_v0::Result::NullifierStatuses(statuses)) => {
                    assert!(statuses.entries.iter().all(|status| !status.is_spent))
                }
                other => panic!("expected nullifier statuses, got {other:?}"),
            },
            other => panic!("expected a v0 response, got {other:?}"),
        }

        let pool_state = platform
            .query_shielded_pool_state(
                GetShieldedPoolStateRequest {
                    version: Some(PoolStateRequestVersion::V0(GetShieldedPoolStateRequestV0 {
                        prove: false,
                        token_id,
                    })),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(pool_state.errors.is_empty(), "{:?}", pool_state.errors);
        match pool_state.data.and_then(|response| response.version) {
            Some(PoolStateResponseVersion::V0(v0)) => assert!(matches!(
                v0.result,
                Some(get_shielded_pool_state_response_v0::Result::TotalBalance(0))
            )),
            other => panic!("expected a total balance, got {other:?}"),
        }
    }

    /// The proved reads keep answering a pool the chain does not have with a proof, deliberately.
    ///
    /// GroveDB proves the pool's absence, which is an answer the client checks against the root
    /// hash rather than has to trust, so it is worth more than the `NotFound` the unproved reads
    /// give. Whoever changes this must change the verifiers with it.
    ///
    /// This test asserts only that a proof comes back; what a verifier makes of it is covered on
    /// the verifier's own side. `Drive::verify_token_shielded_pool_nullifiers` reads the pool's key
    /// out of the same proof and refuses rather than reporting every nullifier unspent;
    /// `Drive::verify_pool_notes_count_v0` and the other siblings walk across the absent pool layer
    /// and return `None`, which an empty pool's `Some(0)` is distinguishable from. No test here
    /// pins what a caller does with either.
    #[test]
    fn proved_queries_answer_a_pool_the_chain_does_not_have_with_an_absence_proof() {
        let (platform, state, version) = setup_platform_with_a_token_pool();
        let token_id = Some(POOL_LESS_TOKEN_ID.to_vec());

        let notes_count = platform
            .query_shielded_notes_count(
                GetShieldedNotesCountRequest {
                    version: Some(NotesCountRequestVersion::V0(
                        GetShieldedNotesCountRequestV0 {
                            prove: true,
                            token_id: token_id.clone(),
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(notes_count.errors.is_empty(), "{:?}", notes_count.errors);
        match notes_count.data.and_then(|response| response.version) {
            Some(NotesCountResponseVersion::V0(v0)) => assert!(matches!(
                v0.result,
                Some(get_shielded_notes_count_response_v0::Result::Proof(_))
            )),
            other => panic!("expected a proof, got {other:?}"),
        }

        let encrypted_notes = platform
            .query_shielded_encrypted_notes(
                GetShieldedEncryptedNotesRequest {
                    version: Some(EncryptedNotesRequestVersion::V0(
                        GetShieldedEncryptedNotesRequestV0 {
                            start_index: 0,
                            count: 1,
                            prove: true,
                            token_id: token_id.clone(),
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(
            encrypted_notes.errors.is_empty(),
            "{:?}",
            encrypted_notes.errors
        );
        match encrypted_notes.data.and_then(|response| response.version) {
            Some(EncryptedNotesResponseVersion::V0(v0)) => assert!(matches!(
                v0.result,
                Some(get_shielded_encrypted_notes_response_v0::Result::Proof(_))
            )),
            other => panic!("expected a proof, got {other:?}"),
        }

        let anchors = platform
            .query_shielded_anchors(
                GetShieldedAnchorsRequest {
                    version: Some(AnchorsRequestVersion::V0(GetShieldedAnchorsRequestV0 {
                        prove: true,
                        token_id: token_id.clone(),
                    })),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(anchors.errors.is_empty(), "{:?}", anchors.errors);
        match anchors.data.and_then(|response| response.version) {
            Some(AnchorsResponseVersion::V0(v0)) => assert!(matches!(
                v0.result,
                Some(get_shielded_anchors_response_v0::Result::Proof(_))
            )),
            other => panic!("expected a proof, got {other:?}"),
        }

        let most_recent_anchor = platform
            .query_most_recent_shielded_anchor(
                GetMostRecentShieldedAnchorRequest {
                    version: Some(MostRecentAnchorRequestVersion::V0(
                        GetMostRecentShieldedAnchorRequestV0 {
                            prove: true,
                            token_id: token_id.clone(),
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(
            most_recent_anchor.errors.is_empty(),
            "{:?}",
            most_recent_anchor.errors
        );
        match most_recent_anchor
            .data
            .and_then(|response| response.version)
        {
            Some(MostRecentAnchorResponseVersion::V0(v0)) => assert!(matches!(
                v0.result,
                Some(get_most_recent_shielded_anchor_response_v0::Result::Proof(
                    _
                ))
            )),
            other => panic!("expected a proof, got {other:?}"),
        }

        let nullifiers = platform
            .query_shielded_nullifiers(
                GetShieldedNullifiersRequest {
                    version: Some(NullifiersRequestVersion::V0(
                        GetShieldedNullifiersRequestV0 {
                            nullifiers: vec![vec![0x11; 32]],
                            prove: true,
                            token_id: token_id.clone(),
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(nullifiers.errors.is_empty(), "{:?}", nullifiers.errors);
        match nullifiers.data.and_then(|response| response.version) {
            Some(NullifiersResponseVersion::V0(v0)) => assert!(matches!(
                v0.result,
                Some(get_shielded_nullifiers_response_v0::Result::Proof(_))
            )),
            other => panic!("expected a proof, got {other:?}"),
        }

        let pool_state = platform
            .query_shielded_pool_state(
                GetShieldedPoolStateRequest {
                    version: Some(PoolStateRequestVersion::V0(GetShieldedPoolStateRequestV0 {
                        prove: true,
                        token_id,
                    })),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(pool_state.errors.is_empty(), "{:?}", pool_state.errors);
        match pool_state.data.and_then(|response| response.version) {
            Some(PoolStateResponseVersion::V0(v0)) => assert!(matches!(
                v0.result,
                Some(get_shielded_pool_state_response_v0::Result::Proof(_))
            )),
            other => panic!("expected a proof, got {other:?}"),
        }
    }

    /// A credit-pool read is never refused for the pool not existing. The client does not name
    /// the credit pool, so there is nothing for an existence gate to refuse, and a gate that
    /// caught it would refuse every shielded credit query on the network at once.
    ///
    /// Asserted through the six unproved reads rather than against the gate helper, because the
    /// gate is only one of the places a refusal could come from and the helper answering `Ok` is
    /// not the same fact as a client getting an answer. Each read here is checked for the value
    /// an empty pool gives, which is what a `NotFound` would have replaced.
    #[test]
    fn unproved_credit_pool_reads_are_never_refused_for_the_pool_not_existing() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let notes_count = platform
            .query_shielded_notes_count(
                GetShieldedNotesCountRequest {
                    version: Some(NotesCountRequestVersion::V0(
                        GetShieldedNotesCountRequestV0 {
                            prove: false,
                            token_id: None,
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(notes_count.errors.is_empty(), "{:?}", notes_count.errors);
        match notes_count.data.and_then(|response| response.version) {
            Some(NotesCountResponseVersion::V0(v0)) => assert!(matches!(
                v0.result,
                Some(get_shielded_notes_count_response_v0::Result::TotalNotesCount(0))
            )),
            other => panic!("expected a notes count, got {other:?}"),
        }

        let encrypted_notes = platform
            .query_shielded_encrypted_notes(
                GetShieldedEncryptedNotesRequest {
                    version: Some(EncryptedNotesRequestVersion::V0(
                        GetShieldedEncryptedNotesRequestV0 {
                            start_index: 0,
                            count: 1,
                            prove: false,
                            token_id: None,
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(
            encrypted_notes.errors.is_empty(),
            "{:?}",
            encrypted_notes.errors
        );
        match encrypted_notes.data.and_then(|response| response.version) {
            Some(EncryptedNotesResponseVersion::V0(v0)) => match v0.result {
                Some(get_shielded_encrypted_notes_response_v0::Result::EncryptedNotes(notes)) => {
                    assert!(notes.entries.is_empty())
                }
                other => panic!("expected encrypted notes, got {other:?}"),
            },
            other => panic!("expected a v0 response, got {other:?}"),
        }

        let anchors = platform
            .query_shielded_anchors(
                GetShieldedAnchorsRequest {
                    version: Some(AnchorsRequestVersion::V0(GetShieldedAnchorsRequestV0 {
                        prove: false,
                        token_id: None,
                    })),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(anchors.errors.is_empty(), "{:?}", anchors.errors);
        match anchors.data.and_then(|response| response.version) {
            Some(AnchorsResponseVersion::V0(v0)) => match v0.result {
                Some(get_shielded_anchors_response_v0::Result::Anchors(anchors)) => {
                    assert!(anchors.anchors.is_empty())
                }
                other => panic!("expected anchors, got {other:?}"),
            },
            other => panic!("expected a v0 response, got {other:?}"),
        }

        let most_recent_anchor = platform
            .query_most_recent_shielded_anchor(
                GetMostRecentShieldedAnchorRequest {
                    version: Some(MostRecentAnchorRequestVersion::V0(
                        GetMostRecentShieldedAnchorRequestV0 {
                            prove: false,
                            token_id: None,
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(
            most_recent_anchor.errors.is_empty(),
            "{:?}",
            most_recent_anchor.errors
        );
        match most_recent_anchor
            .data
            .and_then(|response| response.version)
        {
            Some(MostRecentAnchorResponseVersion::V0(v0)) => match v0.result {
                // The empty-index sentinel this endpoint documents.
                Some(get_most_recent_shielded_anchor_response_v0::Result::Anchor(anchor)) => {
                    assert_eq!(anchor, vec![0; 32])
                }
                other => panic!("expected an anchor, got {other:?}"),
            },
            other => panic!("expected a v0 response, got {other:?}"),
        }

        let nullifiers = platform
            .query_shielded_nullifiers(
                GetShieldedNullifiersRequest {
                    version: Some(NullifiersRequestVersion::V0(
                        GetShieldedNullifiersRequestV0 {
                            nullifiers: vec![vec![0x11; 32]],
                            prove: false,
                            token_id: None,
                        },
                    )),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(nullifiers.errors.is_empty(), "{:?}", nullifiers.errors);
        match nullifiers.data.and_then(|response| response.version) {
            Some(NullifiersResponseVersion::V0(v0)) => match v0.result {
                Some(get_shielded_nullifiers_response_v0::Result::NullifierStatuses(statuses)) => {
                    assert!(statuses.entries.iter().all(|status| !status.is_spent))
                }
                other => panic!("expected nullifier statuses, got {other:?}"),
            },
            other => panic!("expected a v0 response, got {other:?}"),
        }

        let pool_state = platform
            .query_shielded_pool_state(
                GetShieldedPoolStateRequest {
                    version: Some(PoolStateRequestVersion::V0(GetShieldedPoolStateRequestV0 {
                        prove: false,
                        token_id: None,
                    })),
                },
                &state,
                version,
            )
            .expect("expected the query to complete");
        assert!(pool_state.errors.is_empty(), "{:?}", pool_state.errors);
        match pool_state.data.and_then(|response| response.version) {
            Some(PoolStateResponseVersion::V0(v0)) => assert!(matches!(
                v0.result,
                Some(get_shielded_pool_state_response_v0::Result::TotalBalance(0))
            )),
            other => panic!("expected a total balance, got {other:?}"),
        }
    }

    /// A token pool the chain holds passes the existence gate, and one it does not is refused —
    /// the gate reads state rather than always answering one way.
    #[test]
    fn existence_gate_separates_a_held_token_pool_from_an_unheld_one() {
        let (platform, _state, version) = setup_platform_with_a_token_pool();

        assert!(matches!(
            ShieldedPoolSelector::Token(POOLED_TOKEN_ID)
                .validate_pool_exists(&platform.drive, version),
            Ok(Ok(()))
        ));
        assert!(matches!(
            ShieldedPoolSelector::Token(POOL_LESS_TOKEN_ID)
                .validate_pool_exists(&platform.drive, version),
            Ok(Err(QueryError::NotFound(_)))
        ));
    }

    #[test]
    fn selector_resolves_credit_pool_without_token_id() {
        let platform_version = PlatformVersion::latest();
        assert_eq!(
            ShieldedPoolSelector::from_request(None, platform_version).expect("credit"),
            ShieldedPoolSelector::Credit
        );
    }

    #[test]
    fn selector_resolves_token_pool_with_32_byte_token_id() {
        let platform_version = PlatformVersion::latest();
        let selector = ShieldedPoolSelector::from_request(Some(vec![7u8; 32]), platform_version)
            .expect("token pool");
        assert_eq!(selector, ShieldedPoolSelector::Token([7u8; 32]));
        assert_eq!(
            selector.nullifiers_path_vec(),
            token_shielded_pool_nullifiers_path_vec([7u8; 32])
        );
    }

    #[test]
    fn selector_rejects_wrong_length_token_id() {
        let platform_version = PlatformVersion::latest();
        assert!(matches!(
            ShieldedPoolSelector::from_request(Some(vec![7u8; 20]), platform_version),
            Err(QueryError::InvalidArgument(_))
        ));
    }

    #[test]
    fn selector_rejects_token_pools_before_activation() {
        let platform_version =
            PlatformVersion::get(TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION - 1)
                .expect("previous version");
        assert!(matches!(
            ShieldedPoolSelector::from_request(Some(vec![7u8; 32]), platform_version),
            Err(QueryError::InvalidArgument(_))
        ));
    }
}
