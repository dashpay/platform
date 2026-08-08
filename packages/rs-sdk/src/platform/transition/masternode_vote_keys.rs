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
//! here would be the wrong trade.
//!
//! [`PutVote`]: super::vote::PutVote
//! [`Signer`]: dpp::identity::signer::Signer

use dpp::dashcore::hashes::Hash;
use dpp::dashcore::ProTxHash;
use dpp::identifier::MasternodeIdentifiers;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{BinaryData, Identifier};

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
    /// Either the voting key does not match the masternode's registered voting
    /// address, or Platform has not created the identity yet.
    NoVoterIdentity {
        /// The masternode the vote was cast for.
        pro_tx_hash: ProTxHash,
        /// The identity that would have held the key.
        expected_voter_identity: Identifier,
    },
    /// The voter identity exists but holds no enabled `ECDSA_HASH160` voting
    /// key for this address — what a rotation leaves behind, since
    /// `update_voter_identity_v0` disables the old identity's keys rather than
    /// removing them.
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
    let voter_identity = voter_identity_id(pro_tx_hash, voting_address);

    let fetched = Identity::fetch(sdk, voter_identity).await.ok()?;

    let Some(identity) = fetched else {
        return Some(VotingKeyProblem::NoVoterIdentity {
            pro_tx_hash,
            expected_voter_identity: voter_identity,
        });
    };

    match select_voting_key(&identity, voting_address) {
        Some(_) => None,
        None => Some(VotingKeyProblem::NoUsableVotingKey { voter_identity }),
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
