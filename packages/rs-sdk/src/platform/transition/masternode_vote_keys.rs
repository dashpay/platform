//! Voting-key facts for masternode contested-resource votes.
//!
//! Casting a vote needs three things that are properties of Platform, not of
//! any particular binding: which key a voter identity holds, how to recognise
//! that Platform rejected that key, and what the rejection actually meant.
//! They lived in `rs-sdk-ffi`, which meant Swift and Kotlin callers got them
//! (they route through the FFI) while Rust callers of [`PutVote`] did not, and
//! had to rediscover them — including the byte-order rule that made a vote
//! address an identity that never existed.
//!
//! Signing stays with the caller: producing a [`Signer`] from a raw key is a
//! binding concern, and pulling `simple-signer` into the public SDK to do it
//! here would be the wrong trade. [`put_masternode_vote`] takes the caller's
//! signer and owns the rest: the broadcast, and the diagnosis of a refused
//! voting key.
//!
//!
//! [`check_masternode_voting_key`] asks the same question before any vote: whether a
//! `(pro_tx_hash, voting address)` pair is a masternode voter Platform knows, so a host can
//! refuse a mismatched pair when it is entered rather than when a vote is refused.
//!
//! [`PutVote`]: super::vote::PutVote
//! [`Signer`]: dpp::identity::signer::Signer

use dpp::dashcore::ProTxHash;
use dpp::identifier::MasternodeIdentifiers;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{BinaryData, Identifier};

use dpp::identity::signer::Signer;
use dpp::voting::votes::Vote;

use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::vote::PutVote;
use crate::platform::Fetch;
use crate::{Error, Sdk};

/// Why Platform refused to accept a vote's voting key.
///
/// Both cases reach the caller as the same opaque "Public key 0 doesn't exist",
/// so they are only distinguishable by fetching the voter identity — see
/// [`diagnose_voting_key_failure`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VotingKeyProblem {
    /// No voter identity exists for this `(pro_tx_hash, voting address)` pair.
    /// Either `pro_tx_hash` is not a masternode, the voting key does not match
    /// the masternode's registered voting address, or Platform has not created
    /// the identity yet.
    NoVoterIdentity {
        /// The masternode the vote was cast for.
        pro_tx_hash: ProTxHash,
        /// The identity that would have held the key.
        expected_voter_identity: Identifier,
    },
    /// The voter identity exists but holds no enabled `ECDSA_HASH160` voting
    /// key for this address — what a rotation leaves behind, since
    /// `update_voter_identity_v0` disables the old identity's keys rather than
    /// removing them, and what a removed masternode leaves behind
    /// (`disable_identity_keys_v0`).
    NoUsableVotingKey {
        /// The identity that was fetched.
        voter_identity: Identifier,
    },
}

/// The voter identity id for a masternode's voting key.
///
/// Takes a typed [`ProTxHash`] rather than bytes on purpose: the derivation is
/// orientation-sensitive and `Txid` bytes for the same transaction are its
/// exact reverse (`ProTxHash` is `#[hash_newtype(forward)]`, `Txid` is not).
/// Passing the wrong one yields an identity that has never existed instead of
/// an error, so the type is the guard.
pub fn voter_identity_id(pro_tx_hash: ProTxHash, voting_address: &[u8; 20]) -> Identifier {
    Identifier::create_voter_identifier(&pro_tx_hash.to_byte_array(), voting_address)
}

/// The voting key Platform holds for a voter identity at `voting_address`.
///
/// Knowable without a round trip: `create_voter_identity_v0` assigns id 0, and
/// a rotation creates a *different* identity — the identifier includes the
/// voting address — whose key is likewise 0. So a caller can build this and
/// broadcast, rather than fetching the identity first.
pub fn voter_identity_voting_key(voting_address: &[u8; 20]) -> IdentityPublicKey {
    IdentityPublicKeyV0 {
        id: 0,
        purpose: Purpose::VOTING,
        security_level: SecurityLevel::HIGH,
        key_type: KeyType::ECDSA_HASH160,
        read_only: true,
        data: BinaryData::new(voting_address.to_vec()),
        disabled_at: None,
        contract_bounds: None,
    }
    .into()
}

/// The usable voting key on a fetched voter identity, if any.
///
/// Matches on the key's own data rather than its position. Position would
/// usually work — Platform assigns id 0 — but it silently picks the wrong key
/// on an identity carrying others, and cannot tell a usable key from one a
/// rotation disabled. `disabled_at` is therefore part of the match, not an
/// afterthought: a disabled key exists and would be selected by id.
pub fn select_voting_key(
    identity: &Identity,
    voting_address: &[u8; 20],
) -> Option<IdentityPublicKey> {
    identity
        .public_keys()
        .values()
        .find(|key| {
            key.purpose() == Purpose::VOTING
                && key.key_type() == KeyType::ECDSA_HASH160
                && key.data().as_slice() == voting_address
                && key.disabled_at().is_none()
        })
        .cloned()
}

/// Whether a failed vote broadcast is Platform rejecting the VOTING KEY, as
/// opposed to anything else that can fail a vote.
///
/// Matched on the typed consensus error rather than its rendered text: these
/// three signature variants are exactly what [`diagnose_voting_key_failure`]
/// can explain, and a message match would silently begin diagnosing unrelated
/// failures the first time a string changed.
///
/// Callers should gate diagnosis on this. Diagnosing every failure lets an
/// absent voter identity masquerade as the cause of a closed poll or a
/// transport error.
pub fn is_voting_key_failure(error: &Error) -> bool {
    use dpp::consensus::signature::SignatureError;
    use dpp::consensus::ConsensusError;

    fn is_key_signature_error(consensus: &ConsensusError) -> bool {
        matches!(
            consensus,
            ConsensusError::SignatureError(
                SignatureError::IdentityNotFoundError(_)
                    | SignatureError::MissingPublicKeyError(_)
                    | SignatureError::PublicKeyIsDisabledError(_)
            )
        )
    }

    match error {
        // Rejected at broadcast: the consensus error rides on the response.
        Error::StateTransitionBroadcastError(e) => {
            e.cause.as_ref().is_some_and(is_key_signature_error)
        }
        // Rejected locally / surfaced as a protocol error.
        Error::Protocol(dpp::ProtocolError::ConsensusError(e)) => is_key_signature_error(e),
        _ => false,
    }
}

/// What Platform's state proves about a masternode voting key.
///
/// Platform creates the voter identity `voter_identity_id(pro_tx_hash, voting_address)` only
/// for a masternode in its masternode list with that registered voting address, gives it the
/// voting key, and disables that key when the masternode is removed or its voting address
/// changes. So one proof-verified fetch of that identity, with no Core or SPV sync, proves
/// whether Platform holds an enabled voter key for the pair.
///
/// That is all it proves: the fetch does not read the masternode list, which a vote is checked
/// against. One history makes the two differ. Before protocol version 15
/// (`update_voter_identity_v0`), a masternode that moved its voting address from A to B and
/// back to A was left with A's key disabled and B's key enabled. B then reads as
/// [`VotingKeyCheck::Valid`], but a vote with it is refused with
/// `MasternodeIncorrectVotingAddressError`, since the list names A; A reads as
/// [`VotingKeyCheck::Invalid`] and cannot vote either. Telling B apart needs the registered
/// voting address from Core.
#[derive(Debug)]
pub enum VotingKeyCheck {
    /// The voter identity exists and holds an enabled voting key for this address. Apart from
    /// the history described on [`VotingKeyCheck`], the key can vote for this masternode. A
    /// host stores it as checked.
    Valid {
        /// The voter identity that holds the key.
        voter_identity: Identifier,
    },
    /// Platform proves the key cannot vote for this masternode: the pro_tx_hash is not a
    /// masternode, the address is not its voting address, or the key was rotated away or
    /// disabled. A host refuses the key.
    Invalid(VotingKeyProblem),
    /// The voter identity could not be fetched (network, proof or SDK failure), so nothing is
    /// known about the key. A host may keep the key and mark it unchecked, to check again
    /// later.
    Undetermined(Box<Error>),
}

/// The verdict for the voter identity `fetched` from Platform at
/// `voter_identity_id(pro_tx_hash, voting_address)`: `None` when Platform proved it absent.
///
/// Kept apart from the fetch so the rule can be tested without a network. Private because it
/// trusts that `fetched` is the identity at that id, which only the proved fetch in
/// [`check_masternode_voting_key`] guarantees.
fn voting_key_check_from_voter_identity(
    pro_tx_hash: ProTxHash,
    voting_address: &[u8; 20],
    fetched: Option<&Identity>,
) -> Result<Identifier, VotingKeyProblem> {
    let expected_voter_identity = voter_identity_id(pro_tx_hash, voting_address);
    let Some(identity) = fetched else {
        return Err(VotingKeyProblem::NoVoterIdentity {
            pro_tx_hash,
            expected_voter_identity,
        });
    };
    match select_voting_key(identity, voting_address) {
        Some(_) => Ok(expected_voter_identity),
        None => Err(VotingKeyProblem::NoUsableVotingKey {
            voter_identity: expected_voter_identity,
        }),
    }
}

/// Checks whether Platform holds an enabled voter key with `voting_address` for the masternode
/// `pro_tx_hash`, by one proof-verified fetch of its voter identity. See [`VotingKeyCheck`] for
/// what that does and does not prove.
///
/// A masternode registered, or a voting key changed, in the last few blocks reads as
/// [`VotingKeyCheck::Invalid`] until Platform processes the masternode list update.
pub async fn check_masternode_voting_key(
    sdk: &Sdk,
    pro_tx_hash: ProTxHash,
    voting_address: &[u8; 20],
) -> VotingKeyCheck {
    let voter_identity = voter_identity_id(pro_tx_hash, voting_address);
    let fetched = match Identity::fetch(sdk, voter_identity).await {
        Ok(fetched) => fetched,
        Err(error) => return VotingKeyCheck::Undetermined(Box::new(error)),
    };
    match voting_key_check_from_voter_identity(pro_tx_hash, voting_address, fetched.as_ref()) {
        Ok(voter_identity) => VotingKeyCheck::Valid { voter_identity },
        Err(problem) => VotingKeyCheck::Invalid(problem),
    }
}

/// Explain a voting-key rejection by fetching the voter identity.
///
/// Costs one Platform round trip, so gate it on [`is_voting_key_failure`] and
/// run it only after a broadcast has already failed — doing it before every
/// cast spends a round trip per (masternode, contest) on runs that
/// overwhelmingly succeed.
///
/// Returns `None` when the identity and its key both check out, or when the
/// fetch itself fails: neither says anything, so the caller keeps its original
/// error rather than replacing it with a guess.
pub async fn diagnose_voting_key_failure(
    sdk: &Sdk,
    pro_tx_hash: ProTxHash,
    voting_address: &[u8; 20],
) -> Option<VotingKeyProblem> {
    match check_masternode_voting_key(sdk, pro_tx_hash, voting_address).await {
        VotingKeyCheck::Invalid(problem) => Some(problem),
        VotingKeyCheck::Valid { .. } | VotingKeyCheck::Undetermined(_) => None,
    }
}

/// Why a masternode vote was not recorded.
#[derive(Debug)]
pub enum MasternodeVoteFailure {
    /// Platform refused the voting key, and the voter identity says why.
    VotingKey(VotingKeyProblem),
    /// Any other failure, or a refused voting key the voter identity does not
    /// explain.
    Broadcast(Box<Error>),
}

/// Signs `vote` for the masternode `pro_tx_hash` with the voting key of
/// `voting_address` that `signer` holds, broadcasts it, and waits for the vote
/// Platform recorded.
///
/// The voting key is the one Platform gives every voter identity
/// ([`voter_identity_voting_key`]), so nothing is fetched before the
/// broadcast. Only when Platform refuses that key
/// ([`is_voting_key_failure`]) is the voter identity fetched to say why
/// ([`diagnose_voting_key_failure`]); every other failure, and a refusal the
/// identity does not explain, comes back as it was.
pub async fn put_masternode_vote<S: Signer<IdentityPublicKey>>(
    sdk: &Sdk,
    vote: &Vote,
    pro_tx_hash: ProTxHash,
    voting_address: &[u8; 20],
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<Vote, MasternodeVoteFailure> {
    let voting_key = voter_identity_voting_key(voting_address);
    let broadcast_error = match vote
        .put_to_platform_and_wait_for_response(
            Identifier::new(pro_tx_hash.to_byte_array()),
            &voting_key,
            sdk,
            signer,
            settings,
        )
        .await
    {
        Ok(recorded) => return Ok(recorded),
        Err(error) => error,
    };
    if !is_voting_key_failure(&broadcast_error) {
        return Err(MasternodeVoteFailure::Broadcast(Box::new(broadcast_error)));
    }
    match diagnose_voting_key_failure(sdk, pro_tx_hash, voting_address).await {
        Some(problem) => Err(MasternodeVoteFailure::VotingKey(problem)),
        None => Err(MasternodeVoteFailure::Broadcast(Box::new(broadcast_error))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::consensus::signature::{
        BasicECDSAError, IdentityNotFoundError, MissingPublicKeyError, PublicKeyIsDisabledError,
        SignatureError,
    };
    use dpp::consensus::ConsensusError;
    use dpp::identity::v0::IdentityV0;
    use std::collections::BTreeMap;

    const VOTING_ADDRESS: [u8; 20] = [7u8; 20];
    const OTHER_ADDRESS: [u8; 20] = [9u8; 20];

    fn identity_id() -> Identifier {
        Identifier::new([3u8; 32])
    }

    fn key(
        id: u32,
        purpose: Purpose,
        key_type: KeyType,
        data: [u8; 20],
        disabled_at: Option<u64>,
    ) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type,
            read_only: false,
            data: BinaryData::new(data.to_vec()),
            disabled_at,
        })
    }

    fn identity_with(keys: Vec<IdentityPublicKey>) -> Identity {
        let mut public_keys = BTreeMap::new();
        for k in keys {
            public_keys.insert(k.id(), k);
        }
        Identity::V0(IdentityV0 {
            id: identity_id(),
            public_keys,
            balance: 0,
            revision: 0,
        })
    }

    fn broadcast_error_with(cause: ConsensusError) -> Error {
        Error::StateTransitionBroadcastError(crate::error::StateTransitionBroadcastError {
            code: 1,
            message: "rejected".to_string(),
            cause: Some(cause),
        })
    }

    /// The fabricated key must be exactly what Platform holds, or broadcasting
    /// with it fails for a reason that looks like a missing identity.
    #[test]
    fn fabricated_key_matches_what_platform_assigns() {
        let key = voter_identity_voting_key(&VOTING_ADDRESS);
        assert_eq!(key.id(), 0);
        assert_eq!(key.purpose(), Purpose::VOTING);
        assert_eq!(key.key_type(), KeyType::ECDSA_HASH160);
        assert_eq!(key.data().as_slice(), &VOTING_ADDRESS);
        assert!(key.disabled_at().is_none());
    }

    /// The identity id is orientation-sensitive: the same 32 bytes reversed
    /// address a different identity. This is the property that made a vote fail
    /// as "no voter identity" when callers passed `Txid` bytes.
    #[test]
    fn voter_identity_id_is_orientation_sensitive() {
        let bytes = [1u8; 32];
        let mut reversed = bytes;
        reversed.reverse();
        // Distinct bytes so the two orientations cannot coincide.
        let mut asymmetric = [0u8; 32];
        asymmetric[0] = 1;

        let forward = voter_identity_id(ProTxHash::from_byte_array(asymmetric), &VOTING_ADDRESS);
        let mut flipped = asymmetric;
        flipped.reverse();
        let backward = voter_identity_id(ProTxHash::from_byte_array(flipped), &VOTING_ADDRESS);

        assert_ne!(
            forward, backward,
            "reversing the pro_tx_hash must change the identity — the type is \
             the only thing keeping callers honest"
        );
    }

    #[test]
    fn selects_the_enabled_voting_key() {
        let identity = identity_with(vec![key(
            0,
            Purpose::VOTING,
            KeyType::ECDSA_HASH160,
            VOTING_ADDRESS,
            None,
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS).is_some());
    }

    /// Selection is by data, not position: a voting key at a non-zero id behind
    /// another key at 0 must still be found.
    #[test]
    fn selects_by_data_not_by_position() {
        let identity = identity_with(vec![
            key(
                0,
                Purpose::AUTHENTICATION,
                KeyType::ECDSA_HASH160,
                OTHER_ADDRESS,
                None,
            ),
            key(
                4,
                Purpose::VOTING,
                KeyType::ECDSA_HASH160,
                VOTING_ADDRESS,
                None,
            ),
        ]);
        assert_eq!(
            select_voting_key(&identity, &VOTING_ADDRESS).map(|k| k.id()),
            Some(4)
        );
    }

    /// What a rotation leaves behind — the key exists and would be picked by id.
    #[test]
    fn rejects_a_disabled_voting_key() {
        let identity = identity_with(vec![key(
            0,
            Purpose::VOTING,
            KeyType::ECDSA_HASH160,
            VOTING_ADDRESS,
            Some(1_700_000_000),
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS).is_none());
    }

    #[test]
    fn rejects_a_different_address() {
        let identity = identity_with(vec![key(
            0,
            Purpose::VOTING,
            KeyType::ECDSA_HASH160,
            OTHER_ADDRESS,
            None,
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS).is_none());
    }

    /// Purpose must be load-bearing on its own: the same address and key type
    /// under AUTHENTICATION must not be selected.
    #[test]
    fn rejects_a_matching_address_under_the_wrong_purpose() {
        let identity = identity_with(vec![key(
            0,
            Purpose::AUTHENTICATION,
            KeyType::ECDSA_HASH160,
            VOTING_ADDRESS,
            None,
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS).is_none());
    }

    #[test]
    fn rejects_a_matching_address_under_the_wrong_key_type() {
        let identity = identity_with(vec![key(
            0,
            Purpose::VOTING,
            KeyType::BIP13_SCRIPT_HASH,
            VOTING_ADDRESS,
            None,
        )]);
        assert!(select_voting_key(&identity, &VOTING_ADDRESS).is_none());
    }

    // ---- checking a (pro_tx_hash, voting address) pair ----------------------

    /// A pro_tx_hash whose two byte orders differ, so a reversal would show.
    fn pro_tx_hash() -> ProTxHash {
        ProTxHash::from_byte_array(std::array::from_fn(|i| i as u8 + 1))
    }

    /// The voter identity Platform creates for `pro_tx_hash` and `voting_address`: its own id,
    /// not the masternode's (owner) identity id, holding the voting key at id 0.
    fn voter_identity(
        pro_tx_hash: ProTxHash,
        voting_address: [u8; 20],
        disabled_at: Option<u64>,
    ) -> Identity {
        let mut public_keys = BTreeMap::new();
        public_keys.insert(
            0,
            key(
                0,
                Purpose::VOTING,
                KeyType::ECDSA_HASH160,
                voting_address,
                disabled_at,
            ),
        );
        Identity::V0(IdentityV0 {
            id: voter_identity_id(pro_tx_hash, &voting_address),
            public_keys,
            balance: 0,
            revision: 0,
        })
    }

    /// The masternode's identity is named by its pro_tx_hash; its voter identity is a
    /// different id, derived from the pro_tx_hash and the voting address together.
    #[test]
    fn should_derive_a_voter_identity_distinct_from_the_owner_identity() {
        let owner_identity = Identifier::new(pro_tx_hash().to_byte_array());
        let voter = voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS);
        assert_ne!(voter, owner_identity);
        assert_ne!(voter, voter_identity_id(pro_tx_hash(), &OTHER_ADDRESS));
    }

    #[test]
    fn should_accept_a_voter_identity_holding_the_enabled_voting_key() {
        let identity = voter_identity(pro_tx_hash(), VOTING_ADDRESS, None);
        assert_eq!(
            voting_key_check_from_voter_identity(pro_tx_hash(), &VOTING_ADDRESS, Some(&identity)),
            Ok(voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS))
        );
    }

    /// Absent: not a masternode, or not its voting address. The problem names the voter
    /// identity that was looked for, not the masternode's own identity.
    #[test]
    fn should_refuse_a_pair_with_no_voter_identity() {
        assert_eq!(
            voting_key_check_from_voter_identity(pro_tx_hash(), &VOTING_ADDRESS, None),
            Err(VotingKeyProblem::NoVoterIdentity {
                pro_tx_hash: pro_tx_hash(),
                expected_voter_identity: voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS),
            })
        );
    }

    /// What a removed masternode or a rotated voting key leaves behind.
    #[test]
    fn should_refuse_a_voter_identity_whose_voting_key_is_disabled() {
        let identity = voter_identity(pro_tx_hash(), VOTING_ADDRESS, Some(1_700_000_000));
        assert_eq!(
            voting_key_check_from_voter_identity(pro_tx_hash(), &VOTING_ADDRESS, Some(&identity)),
            Err(VotingKeyProblem::NoUsableVotingKey {
                voter_identity: voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS),
            })
        );
    }

    /// The history `update_voter_identity_v0` leaves after a masternode moves its voting
    /// address from A to B and back to A: A's key disabled, B's enabled. The proof can only
    /// say so: B reads as valid although the masternode list names A (see [`VotingKeyCheck`]),
    /// and A reads as invalid. If this changes, update the documented limit.
    #[test]
    fn should_report_what_a_pre_v15_return_to_a_voting_address_leaves_behind() {
        let address_a = VOTING_ADDRESS;
        let address_b = OTHER_ADDRESS;
        let identity_a = voter_identity(pro_tx_hash(), address_a, Some(1_700_000_000));
        let identity_b = voter_identity(pro_tx_hash(), address_b, None);

        assert_eq!(
            voting_key_check_from_voter_identity(pro_tx_hash(), &address_b, Some(&identity_b)),
            Ok(voter_identity_id(pro_tx_hash(), &address_b))
        );
        assert_eq!(
            voting_key_check_from_voter_identity(pro_tx_hash(), &address_a, Some(&identity_a)),
            Err(VotingKeyProblem::NoUsableVotingKey {
                voter_identity: voter_identity_id(pro_tx_hash(), &address_a),
            })
        );
    }

    #[cfg(feature = "mocks")]
    mod with_a_mock_sdk {
        use super::*;

        async fn check_against(fetched: Option<Option<Identity>>) -> VotingKeyCheck {
            let mut sdk = Sdk::new_mock();
            if let Some(fetched) = fetched {
                sdk.mock()
                    .expect_fetch::<Identity, _>(
                        voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS),
                        fetched,
                    )
                    .await
                    .expect("set mock expectation");
            }
            check_masternode_voting_key(&sdk, pro_tx_hash(), &VOTING_ADDRESS).await
        }

        #[tokio::test]
        async fn should_check_a_valid_pair_as_valid() {
            let identity = voter_identity(pro_tx_hash(), VOTING_ADDRESS, None);
            match check_against(Some(Some(identity))).await {
                VotingKeyCheck::Valid { voter_identity } => assert_eq!(
                    voter_identity,
                    voter_identity_id(pro_tx_hash(), &VOTING_ADDRESS)
                ),
                other => panic!("expected Valid, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn should_check_a_pair_platform_proves_absent_as_invalid() {
            assert!(matches!(
                check_against(Some(None)).await,
                VotingKeyCheck::Invalid(VotingKeyProblem::NoVoterIdentity { .. })
            ));
        }

        /// A failed fetch says nothing about the key: it is undetermined, never invalid.
        #[tokio::test]
        async fn should_check_a_pair_whose_fetch_fails_as_undetermined() {
            assert!(matches!(
                check_against(None).await,
                VotingKeyCheck::Undetermined(_)
            ));
        }
    }

    #[test]
    fn key_failures_are_diagnosable() {
        for cause in [
            ConsensusError::SignatureError(SignatureError::MissingPublicKeyError(
                MissingPublicKeyError::new(0),
            )),
            ConsensusError::SignatureError(SignatureError::PublicKeyIsDisabledError(
                PublicKeyIsDisabledError::new(0),
            )),
            ConsensusError::SignatureError(SignatureError::IdentityNotFoundError(
                IdentityNotFoundError::new(identity_id()),
            )),
        ] {
            assert!(is_voting_key_failure(&broadcast_error_with(cause.clone())));
        }
    }

    /// The discrimination the gate exists for: a signature failure that is NOT
    /// about the key's existence or state keeps its own error, as does a
    /// broadcast error carrying no consensus cause.
    #[test]
    fn unrelated_failures_are_not_diagnosable() {
        let other_signature_failure = broadcast_error_with(ConsensusError::SignatureError(
            SignatureError::BasicECDSAError(BasicECDSAError::new("bad signature".to_string())),
        ));
        assert!(!is_voting_key_failure(&other_signature_failure));

        let transport =
            Error::StateTransitionBroadcastError(crate::error::StateTransitionBroadcastError {
                code: 2,
                message: "connection reset".to_string(),
                cause: None,
            });
        assert!(!is_voting_key_failure(&transport));
    }
}
