//! Signing and broadcasting a masternode vote from the masternode's voting private key.
//!
//! Shared by every FFI that casts a masternode vote: a contested resource vote
//! (`dash_sdk_contested_resource_cast_vote`) and an answer on a decision proposal
//! (`dash_sdk_decision_proposal_cast_vote`). Each builds its `Vote`; this module turns the
//! caller's raw key material into what [`PutVote`] needs, broadcasts, and explains a voting
//! key rejection.
//!
//! # Who can cast a vote
//!
//! A vote is cast by a masternode with its voting key: an `ECDSA_HASH160` key whose 20-byte
//! data is the hash160 of the voting public key, tied to the masternode's pro_tx_hash (see
//! `get_voter_identity_key_v0` in rs-drive-abci). The voter identity is derived from both as
//! `create_voter_identifier(pro_tx_hash, voting_address)`.
//!
//! The FFI therefore takes the raw 32-byte voting private key plus the 32-byte pro_tx_hash.
//! From the private key it derives a [`SingleKeySigner`] that signs the transition, and
//! `hash160(pubkey)`, the voting address, which names both the voter identity and the key on
//! it.
//!
//! The voting key is built locally, not fetched (rs-sdk's `voter_identity_voting_key`): Platform
//! always gives a voter identity's voting key id 0, and a rotation creates a different
//! identity (the identifier includes the voting address) whose key is also 0.
//! `SingleKeySigner::can_sign_with` recomputes the same `hash160(pubkey)` from the private
//! key, so key and signer agree by construction.
//!
//! # pro_tx_hash byte order
//!
//! Callers pass the pro_tx_hash in wire order: the orientation `Txid` stores, which is what a
//! parsed ProRegTx yields (`reg.txid()`) and what a wallet holds. Platform names masternodes
//! by the other orientation: `ProTxHash` is `#[hash_newtype(forward)]` while `Txid` is not,
//! so `ProTxHash::to_byte_array()` is display order, the reverse of the `Txid` bytes of the
//! same transaction, and drive-abci derives the voter identity from
//! `masternode.pro_tx_hash.to_byte_array()`. [`pro_tx_hash_from_wire_order`] reverses once,
//! and the reversed value is used for the voter identity, the transition and the queries
//! keyed by masternode, so they cannot drift apart.
//!
//! # Diagnosis is deferred to the failure path
//!
//! Platform reports two different failures as the same opaque "Public key 0 doesn't exist":
//! no voter identity exists for this `(pro_tx_hash, voting address)` pair, or a rotation
//! disabled the old identity's keys (`update_voter_identity_v0`), so key 0 exists but is
//! unusable. Telling them apart needs the identity, and fetching it before every cast would
//! spend a Platform round trip per vote on runs that overwhelmingly succeed. So the fetch
//! happens only after a broadcast failed on the voting key ([`is_voting_key_failure`]). A
//! diagnosis replaces the opaque error; an unrelated failure (network, fees, a closed poll)
//! is returned unchanged rather than recast as a key problem.
//!
//! A regular wallet is not a masternode and has no voting key, so its vote reaches a
//! deterministic rejection (no voter identity), the expected end state when testing without
//! real masternode credentials.

use crate::types::{FFINetwork, Network};
use crate::FFIError;
use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
use dash_sdk::dpp::dashcore::ProTxHash;
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::dpp::secp256k1::{PublicKey, SecretKey};
use dash_sdk::dpp::voting::votes::Vote;
use dash_sdk::platform::transition::masternode_vote_keys::{
    put_masternode_vote, MasternodeVoteFailure, VotingKeyProblem,
};
use dash_sdk::Sdk;
use simple_signer::SingleKeySigner;
use zeroize::Zeroizing;

/// A masternode about to vote: its pro_tx_hash as Platform names it, its voting address, and
/// the signer and key its voting private key gives.
pub(crate) struct MasternodeVoter {
    /// The pro_tx_hash in `ProTxHash` order, the wire-order bytes the caller passed reversed.
    pro_tx_hash: [u8; 32],
    /// `hash160` of the compressed voting public key.
    voting_address: [u8; 20],
    /// Signs with the voting private key.
    signer: SingleKeySigner,
}

impl MasternodeVoter {
    /// Reads the masternode from the raw FFI arguments: the pro_tx_hash in wire order (see
    /// the module docs) and the 32-byte voting private key. Nothing reaches the network.
    ///
    /// # Safety
    /// `voter_pro_tx_hash` and `voting_private_key` must each be null or point to 32 readable
    /// bytes for the duration of the call.
    pub(crate) unsafe fn from_raw(
        voter_pro_tx_hash: *const u8,
        voting_private_key: *const u8,
        network: FFINetwork,
    ) -> Result<Self, FFIError> {
        let pro_tx_hash = pro_tx_hash_from_wire_order(voter_pro_tx_hash)?.to_buffer();
        if voting_private_key.is_null() {
            return Err(invalid("voting_private_key is null"));
        }
        let network: Network = network.into();

        // Hold the private key in a zeroizing buffer so it does not linger.
        let mut key = Zeroizing::new([0u8; 32]);
        key.copy_from_slice(std::slice::from_raw_parts(voting_private_key, 32));

        let signer = SingleKeySigner::new_from_slice(key.as_slice(), network)
            .map_err(|e| invalid(&format!("Invalid voting private key: {}", e)))?;
        let secret_key = SecretKey::from_secret_bytes(*key)
            .map_err(|e| invalid(&format!("Invalid voting private key: {}", e)))?;
        let public_key = PublicKey::from_secret_key(&secret_key);
        let voting_address = hash160::Hash::hash(&public_key.serialize()).to_byte_array();

        Ok(MasternodeVoter {
            pro_tx_hash,
            voting_address,
            signer,
        })
    }

    /// Signs `vote` with the voting key, broadcasts it and waits for the response, through
    /// rs-sdk's [`put_masternode_vote`], which explains a refused voting key. This only words
    /// the failure for the host.
    pub(crate) async fn put_vote(&self, sdk: &Sdk, vote: &Vote) -> Result<(), FFIError> {
        put_masternode_vote(
            sdk,
            vote,
            ProTxHash::from_byte_array(self.pro_tx_hash),
            &self.voting_address,
            &self.signer,
            None,
        )
        .await
        .map(|_| ())
        .map_err(|failure| match failure {
            MasternodeVoteFailure::VotingKey(problem) => describe_voting_key_problem(problem),
            MasternodeVoteFailure::Broadcast(error) => FFIError::from(*error),
        })
    }
}

/// The pro_tx_hash at `wire_order`, 32 bytes in wire order (see the module docs), as Platform
/// names the masternode: the bytes reversed.
///
/// # Safety
/// `wire_order` must be null or point to 32 readable bytes for the duration of the call.
pub(crate) unsafe fn pro_tx_hash_from_wire_order(
    wire_order: *const u8,
) -> Result<Identifier, FFIError> {
    if wire_order.is_null() {
        return Err(invalid("voter_pro_tx_hash is null"));
    }
    let mut pro_tx_hash = [0u8; 32];
    pro_tx_hash.copy_from_slice(std::slice::from_raw_parts(wire_order, 32));
    pro_tx_hash.reverse();
    Ok(Identifier::new(pro_tx_hash))
}

fn invalid(message: &str) -> FFIError {
    FFIError::InvalidParameter(message.to_string())
}

/// Renders a [`VotingKeyProblem`] as the message a host shows.
///
/// rs-sdk reports the fact; phrasing it is a binding concern, so the sentence lives here
/// rather than in the SDK where every caller would inherit it.
fn describe_voting_key_problem(problem: VotingKeyProblem) -> FFIError {
    match problem {
        VotingKeyProblem::NoVoterIdentity {
            pro_tx_hash,
            expected_voter_identity,
        } => FFIError::InvalidParameter(format!(
            "No voting identity exists on Platform for masternode {} with this voting key \
             (expected voter identity {}). Either the voting key does not match the \
             masternode's registered voting address, or Platform has not created the \
             voter identity yet.",
            Identifier::new(pro_tx_hash.to_byte_array()),
            expected_voter_identity
        )),
        VotingKeyProblem::NoUsableVotingKey { voter_identity } => {
            FFIError::InvalidParameter(format!(
                "Voter identity {} has no enabled ECDSA_HASH160 voting key matching this \
                 private key. The masternode's voting key may have been rotated.",
                voter_identity
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dash_sdk::dpp::identity::signer::Signer;
    use dash_sdk::platform::transition::masternode_vote_keys::voter_identity_voting_key;

    #[test]
    fn should_reverse_the_wire_order_pro_tx_hash() {
        let mut wire_order = [0u8; 32];
        for (index, byte) in wire_order.iter_mut().enumerate() {
            *byte = index as u8;
        }
        let pro_tx_hash =
            unsafe { pro_tx_hash_from_wire_order(wire_order.as_ptr()) }.expect("32 bytes");
        let mut reversed = wire_order;
        reversed.reverse();
        assert_eq!(pro_tx_hash, Identifier::new(reversed));
    }

    #[test]
    fn should_refuse_a_null_pro_tx_hash_or_voting_key() {
        let key = [2u8; 32];
        let pro_tx_hash = [1u8; 32];
        for (pro_tx_hash, voting_private_key) in [
            (std::ptr::null(), key.as_ptr()),
            (pro_tx_hash.as_ptr(), std::ptr::null()),
        ] {
            let error = unsafe {
                MasternodeVoter::from_raw(pro_tx_hash, voting_private_key, FFINetwork::Testnet)
            }
            .err()
            .expect("refused");
            assert!(matches!(error, FFIError::InvalidParameter(_)), "{error:?}");
        }
    }

    #[test]
    fn should_refuse_a_voting_key_that_is_not_a_secret_key() {
        let pro_tx_hash = [1u8; 32];
        let zero_key = [0u8; 32];
        let error = unsafe {
            MasternodeVoter::from_raw(pro_tx_hash.as_ptr(), zero_key.as_ptr(), FFINetwork::Testnet)
        }
        .err()
        .expect("a zero scalar is not a secret key");
        assert!(matches!(error, FFIError::InvalidParameter(_)), "{error:?}");
    }

    /// The signer and the voting key both come from the private key, so the signer signs with
    /// the key the vote names.
    #[test]
    fn should_derive_a_voting_key_the_signer_signs_with() {
        let pro_tx_hash = [1u8; 32];
        let key = [2u8; 32];
        let voter = unsafe {
            MasternodeVoter::from_raw(pro_tx_hash.as_ptr(), key.as_ptr(), FFINetwork::Testnet)
        }
        .expect("a valid key");
        let voting_key = voter_identity_voting_key(&voter.voting_address);
        assert_eq!(voting_key.id(), 0);
        assert_eq!(voting_key.data().as_slice(), &voter.voting_address);
        assert!(voter.signer.can_sign_with(&voting_key));
        let mut reversed = pro_tx_hash;
        reversed.reverse();
        assert_eq!(voter.pro_tx_hash, reversed);
    }

    // ---- describe_voting_key_problem ----------------------------------------
    //
    // Selection, the failure gate and the diagnosis itself live in rs-sdk
    // (`platform::transition::masternode_vote_keys`) and are tested there. What remains here is
    // the one thing this layer owns: turning the SDK's typed fact into the sentence a host
    // shows.

    #[test]
    fn missing_identity_message_names_both_identifiers() {
        let pro_tx = ProTxHash::from_byte_array([1u8; 32]);
        let expected = Identifier::new([3u8; 32]);
        let msg = format!(
            "{:?}",
            describe_voting_key_problem(VotingKeyProblem::NoVoterIdentity {
                pro_tx_hash: pro_tx,
                expected_voter_identity: expected,
            })
        );

        assert!(msg.contains(&format!("{}", Identifier::new(pro_tx.to_byte_array()))));
        assert!(msg.contains(&format!("{}", expected)));
        // The message crosses the FFI boundary verbatim and is shown to users, so it must
        // read as prose: an earlier version rendered with blank gaps from an unescaped
        // multi-line literal.
        assert!(!msg.contains("  "), "message has whitespace runs: {msg}");
    }

    #[test]
    fn rotated_key_message_names_the_identity_and_the_cause() {
        let voter = Identifier::new([5u8; 32]);
        let msg = format!(
            "{:?}",
            describe_voting_key_problem(VotingKeyProblem::NoUsableVotingKey {
                voter_identity: voter,
            })
        );

        assert!(msg.contains(&format!("{}", voter)));
        assert!(msg.contains("rotated"));
        assert!(!msg.contains("  "), "message has whitespace runs: {msg}");
    }
}
