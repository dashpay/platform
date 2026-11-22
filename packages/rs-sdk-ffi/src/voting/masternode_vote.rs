//! Signing and broadcasting a masternode vote with the masternode's voting key.
//!
//! Shared by every FFI that casts a masternode vote: a contested resource vote
//! (`dash_sdk_contested_resource_cast_vote`), an answer on a decision proposal
//! (`dash_sdk_decision_proposal_cast_vote`) and an answer on a multiple choice proposal
//! (`dash_sdk_multiple_choice_proposal_cast_vote`), each also in a `_with_signer` form. Each
//! builds its `Vote`; this module turns the caller's arguments into what [`PutVote`] needs,
//! broadcasts, and explains a voting key rejection.
//!
//! # Who can cast a vote
//!
//! A vote is cast by a masternode with its voting key: an `ECDSA_HASH160` key whose 20-byte
//! data is the hash160 of the voting public key, tied to the masternode's pro_tx_hash (see
//! `get_voter_identity_key_v0` in rs-drive-abci). The voter identity is derived from both as
//! `create_voter_identifier(pro_tx_hash, voting_address)`.
//!
//! The voting key is built locally, not fetched (rs-sdk's `voter_identity_voting_key`):
//! Platform always gives a voter identity's voting key id 0, and a rotation creates a
//! different identity (the identifier includes the voting address) whose key is also 0.
//!
//! # Two ways in
//!
//! Both take the 32-byte pro_tx_hash in wire order (see below) and differ in what signs.
//!
//! - **The raw voting private key** ([`MasternodeVoter::from_raw`], the functions without a
//!   suffix). From the 32-byte private key the FFI derives a [`SingleKeySigner`] that signs
//!   the transition, and `hash160(pubkey)`, the voting address, which names both the voter
//!   identity and the key on it. `SingleKeySigner::can_sign_with` recomputes the same
//!   `hash160(pubkey)`, so key and signer agree by construction.
//! - **A host signer** ([`MasternodeVoter::from_signer_handle`], the `_with_signer`
//!   functions). The host passes the 20-byte voting address and a `SignerHandle`, and keeps
//!   the private key on its side (the iOS Keychain). The voting key is built from the address
//!   exactly as above, and the signer is asked to sign with that key: the callback receives
//!   the key's data, the voting address, with key type `ECDSA_HASH160`. A host that holds the
//!   private key but not the address gets it from
//!   [`dash_sdk_masternode_voting_address_from_private_key`], so the address is formed here
//!   for both ways in.
//!
//! [`MasternodeVoter::put_vote`] is the single place that broadcasts either.
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

use crate::signer::{VTableSigner, VTableSignerRef};
use crate::types::{FFINetwork, Network, SignerHandle};
use crate::{DashSDKResult, FFIError};
use dash_sdk::dpp::dashcore::ProTxHash;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::KeyType;
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::dpp::prelude::IdentityPublicKey;
use dash_sdk::dpp::voting::votes::Vote;
use dash_sdk::platform::transition::masternode_vote_keys::{
    put_masternode_vote, MasternodeVoteFailure, VotingKeyProblem,
};
use dash_sdk::Sdk;
use simple_signer::SingleKeySigner;
use zeroize::Zeroizing;

/// A masternode about to vote: its pro_tx_hash as Platform names it, its voting address, and
/// what signs with its voting key: a [`SingleKeySigner`] over the raw voting private key
/// ([`MasternodeVoter::from_raw`]) or the host's signer
/// ([`MasternodeVoter::from_signer_handle`]).
pub(crate) struct MasternodeVoter<S> {
    /// The pro_tx_hash in `ProTxHash` order, the wire-order bytes the caller passed reversed.
    pro_tx_hash: [u8; 32],
    /// `hash160` of the compressed voting public key.
    voting_address: [u8; 20],
    /// Signs with the voting key.
    signer: S,
}

impl MasternodeVoter<SingleKeySigner> {
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
        let key = read_voting_private_key(voting_private_key)?;
        let network: Network = network.into();

        let signer = SingleKeySigner::new_from_slice(key.as_slice(), network)
            .map_err(|e| invalid(&format!("Invalid voting private key: {}", e)))?;
        let voting_address = voting_address_of(&key)?;

        Ok(MasternodeVoter {
            pro_tx_hash,
            voting_address,
            signer,
        })
    }
}

impl<'a> MasternodeVoter<VTableSignerRef<'a>> {
    /// Reads the masternode from the FFI arguments of a `_with_signer` vote: the pro_tx_hash
    /// in wire order (see the module docs), the 20-byte voting address and the host's signer,
    /// which is asked to sign with the voting key built from that address. Nothing reaches
    /// the network.
    ///
    /// # Safety
    /// `voter_pro_tx_hash` must be null or point to 32 readable bytes, `voting_address` null
    /// or to 20 readable bytes, and `signer_handle` null or a valid `VTableSigner` that
    /// outlives `'a`.
    pub(crate) unsafe fn from_signer_handle(
        voter_pro_tx_hash: *const u8,
        voting_address: *const u8,
        signer_handle: *const SignerHandle,
    ) -> Result<Self, FFIError> {
        let pro_tx_hash = pro_tx_hash_from_wire_order(voter_pro_tx_hash)?.to_buffer();
        if voting_address.is_null() {
            return Err(invalid("voting_address is null"));
        }
        if signer_handle.is_null() {
            return Err(invalid("signer_handle is null"));
        }
        let mut address = [0u8; 20];
        address.copy_from_slice(std::slice::from_raw_parts(voting_address, 20));
        let signer = &*(signer_handle as *const VTableSigner);

        Ok(MasternodeVoter {
            pro_tx_hash,
            voting_address: address,
            signer: VTableSignerRef(signer),
        })
    }
}

impl<S: Signer<IdentityPublicKey>> MasternodeVoter<S> {
    /// Signs `vote` with the voting key, broadcasts it and waits for the response, through
    /// rs-sdk's [`put_masternode_vote`], which builds the voting key from the voting address,
    /// asks the signer to sign with it, and explains a refused voting key. This only words
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

/// Writes the voting address of a masternode voting private key to `out_voting_address`: the
/// hash160 of the compressed voting public key, the 20 bytes the `_with_signer` vote
/// functions take as `voting_address` and that name the voting key on the voter identity.
///
/// The address is formed here, as `dash_sdk_contested_resource_cast_vote` forms it from the
/// same key, so a host that holds the voting private key never has to.
///
/// # Parameters
/// * `voting_private_key` - Pointer to the 32-byte masternode voting private key. It is read
///   into a buffer that is zeroed on return and never leaves this call.
/// * `out_voting_address` - Pointer to 20 writable bytes that receive the voting address.
///   They need not be initialized.
///
/// # Returns
/// No data on success, with the address written. `InvalidParameter` for a null pointer or a
/// key that is not a secp256k1 secret key (zero, or not below the curve order), and nothing
/// is written then.
///
/// # Safety
/// `voting_private_key` must be null or point to 32 readable bytes, and `out_voting_address`
/// null or to 20 writable bytes, for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_masternode_voting_address_from_private_key(
    voting_private_key: *const u8,
    out_voting_address: *mut u8,
) -> DashSDKResult {
    let result = (|| {
        if out_voting_address.is_null() {
            return Err(invalid("out_voting_address is null"));
        }
        let key = read_voting_private_key(voting_private_key)?;
        voting_address_of(&key)
    })();
    match result {
        Ok(voting_address) => {
            // A raw copy: the caller's buffer may be uninitialized, so no `&mut [u8]` is
            // formed over it.
            std::ptr::copy_nonoverlapping(voting_address.as_ptr(), out_voting_address, 20);
            DashSDKResult::success(std::ptr::null_mut())
        }
        Err(error) => DashSDKResult::error(error.into()),
    }
}

/// The 32-byte voting private key at `voting_private_key`, in a buffer that is zeroed when it
/// is dropped.
///
/// # Safety
/// `voting_private_key` must be null or point to 32 readable bytes for the duration of the
/// call.
unsafe fn read_voting_private_key(
    voting_private_key: *const u8,
) -> Result<Zeroizing<[u8; 32]>, FFIError> {
    if voting_private_key.is_null() {
        return Err(invalid("voting_private_key is null"));
    }
    let mut key = Zeroizing::new([0u8; 32]);
    key.copy_from_slice(std::slice::from_raw_parts(voting_private_key, 32));
    Ok(key)
}

/// The voting address of `key`: the data of its `ECDSA_HASH160` key, hash160 of its
/// compressed public key, derived by DPP so it cannot drift from how Platform forms it.
fn voting_address_of(key: &Zeroizing<[u8; 32]>) -> Result<[u8; 20], FFIError> {
    // DPP ignores the network for `ECDSA_HASH160`: the key data is the same on every network.
    let data = KeyType::ECDSA_HASH160
        .public_key_data_from_private_key_data(key, Network::Mainnet)
        .map_err(|e| invalid(&format!("Invalid voting private key: {}", e)))?;
    <[u8; 20]>::try_from(data.as_slice()).map_err(|_| {
        FFIError::InternalError(format!(
            "An ECDSA_HASH160 key has {} bytes of data, not 20",
            data.len()
        ))
    })
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
    use crate::signer::{SignCompletionCallback, SignerVTable};
    use crate::{dash_sdk_error_free, DashSDKErrorCode};
    use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dash_sdk::platform::transition::masternode_vote_keys::voter_identity_voting_key;
    use std::ffi::c_void;
    use std::mem::MaybeUninit;
    use std::sync::Mutex;

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
        let pro_tx_hash: [u8; 32] = std::array::from_fn(|i| i as u8);
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

    // ---- a voter that signs with a host signer -------------------------------

    /// What the host signer was last asked about: the key type byte and the key data.
    struct SignerCapture {
        asked: Mutex<Option<(u8, Vec<u8>)>>,
    }

    unsafe extern "C" fn capture_sign_async(
        signer: *const c_void,
        pubkey_bytes: *const u8,
        pubkey_len: usize,
        key_type: u8,
        _data: *const u8,
        _data_len: usize,
        completion_ctx: *mut c_void,
        completion: SignCompletionCallback,
    ) {
        let capture = &*(signer as *const SignerCapture);
        let bytes = std::slice::from_raw_parts(pubkey_bytes, pubkey_len).to_vec();
        *capture.asked.lock().expect("not poisoned") = Some((key_type, bytes));
        let signature = [0x77u8; 65];
        completion(
            completion_ctx,
            signature.as_ptr(),
            signature.len(),
            0,
            std::ptr::null(),
        );
    }

    unsafe extern "C" fn capture_can_sign(
        signer: *const c_void,
        pubkey_bytes: *const u8,
        pubkey_len: usize,
        key_type: u8,
    ) -> bool {
        let capture = &*(signer as *const SignerCapture);
        let bytes = std::slice::from_raw_parts(pubkey_bytes, pubkey_len).to_vec();
        *capture.asked.lock().expect("not poisoned") = Some((key_type, bytes));
        true
    }

    unsafe extern "C" fn capture_destroy(signer: *mut c_void) {
        if !signer.is_null() {
            drop(Box::from_raw(signer as *mut SignerCapture));
        }
    }

    /// A callback signer that records what it is asked, and its capture slot. The signer's
    /// destructor frees the slot, so the slot is valid while the signer lives.
    fn capturing_signer() -> (VTableSigner, *const SignerCapture) {
        let capture = Box::into_raw(Box::new(SignerCapture {
            asked: Mutex::new(None),
        }));
        let vtable = Box::into_raw(Box::new(SignerVTable {
            sign_async: capture_sign_async,
            can_sign_with: capture_can_sign,
            destroy: capture_destroy,
        }));
        // SAFETY: both pointers come from `Box::into_raw`; the destructor reclaims the
        // capture and `owns_vtable` reclaims the vtable when the signer drops.
        let signer = unsafe { VTableSigner::from_callback(capture as *mut c_void, vtable, true) };
        (signer, capture)
    }

    #[test]
    fn should_refuse_a_null_pro_tx_hash_voting_address_or_signer() {
        let (signer, _) = capturing_signer();
        let signer_handle = &signer as *const VTableSigner as *const SignerHandle;
        let pro_tx_hash = [1u8; 32];
        let voting_address = [3u8; 20];
        for (pro_tx_hash, voting_address, signer_handle, named) in [
            (
                std::ptr::null(),
                voting_address.as_ptr(),
                signer_handle,
                "voter_pro_tx_hash",
            ),
            (
                pro_tx_hash.as_ptr(),
                std::ptr::null(),
                signer_handle,
                "voting_address",
            ),
            (
                pro_tx_hash.as_ptr(),
                voting_address.as_ptr(),
                std::ptr::null(),
                "signer_handle",
            ),
        ] {
            let error = unsafe {
                MasternodeVoter::from_signer_handle(pro_tx_hash, voting_address, signer_handle)
            }
            .err()
            .expect("refused");
            match error {
                FFIError::InvalidParameter(message) => {
                    assert!(message.contains(named), "{message} names {named}")
                }
                other => panic!("expected InvalidParameter, got {other:?}"),
            }
        }
    }

    /// The voter asks the host signer about the voting key rs-sdk builds from the voting
    /// address: the address as key data, with key type `ECDSA_HASH160`. The broadcast itself
    /// needs the network and is not run here.
    #[tokio::test]
    async fn should_ask_the_signer_for_the_voting_key_of_the_voting_address() {
        let (signer, capture) = capturing_signer();
        let signer_handle = &signer as *const VTableSigner as *const SignerHandle;
        let pro_tx_hash: [u8; 32] = std::array::from_fn(|i| i as u8);
        let voting_address = [3u8; 20];
        let voter = unsafe {
            MasternodeVoter::from_signer_handle(
                pro_tx_hash.as_ptr(),
                voting_address.as_ptr(),
                signer_handle,
            )
        }
        .expect("readable arguments");
        let mut reversed = pro_tx_hash;
        reversed.reverse();
        assert_eq!(voter.pro_tx_hash, reversed);
        assert_eq!(voter.voting_address, voting_address);

        let voting_key = voter_identity_voting_key(&voter.voting_address);
        let expected = Some((KeyType::ECDSA_HASH160 as u8, voting_address.to_vec()));

        assert!(voter.signer.can_sign_with(&voting_key));
        let asked = unsafe { &*capture }
            .asked
            .lock()
            .expect("not poisoned")
            .take();
        assert_eq!(asked, expected);

        voter
            .signer
            .sign(&voting_key, b"transition")
            .await
            .expect("the capture signs");
        let asked = unsafe { &*capture }
            .asked
            .lock()
            .expect("not poisoned")
            .take();
        assert_eq!(asked, expected);
    }

    // ---- dash_sdk_masternode_voting_address_from_private_key -----------------

    /// The address is DPP's `ECDSA_HASH160` key data for the key, the one the raw voter names
    /// and its signer signs with.
    #[test]
    fn should_give_dpp_s_ecdsa_hash160_data_as_the_voting_address() {
        let pro_tx_hash = [1u8; 32];
        let key = [2u8; 32];
        let voter = unsafe {
            MasternodeVoter::from_raw(pro_tx_hash.as_ptr(), key.as_ptr(), FFINetwork::Testnet)
        }
        .expect("a valid key");
        let mut voting_address = [0u8; 20];
        let result = unsafe {
            dash_sdk_masternode_voting_address_from_private_key(
                key.as_ptr(),
                voting_address.as_mut_ptr(),
            )
        };
        assert!(result.error.is_null());
        assert!(result.data.is_null());

        let dpp_data = KeyType::ECDSA_HASH160
            .public_key_data_from_private_key_data(&key, Network::Testnet)
            .expect("a valid key");
        assert_eq!(voting_address.as_slice(), dpp_data.as_slice());
        assert_eq!(voting_address, voter.voting_address);
        assert!(voter
            .signer
            .can_sign_with(&voter_identity_voting_key(&voting_address)));
    }

    #[test]
    fn should_write_the_voting_address_into_an_uninitialized_buffer() {
        let key = [2u8; 32];
        let mut out = MaybeUninit::<[u8; 20]>::uninit();
        let voting_address = unsafe {
            let result = dash_sdk_masternode_voting_address_from_private_key(
                key.as_ptr(),
                out.as_mut_ptr().cast::<u8>(),
            );
            assert!(result.error.is_null());
            assert!(result.data.is_null());
            out.assume_init()
        };
        let expected = voting_address_of(&Zeroizing::new(key)).expect("a valid key");
        assert_eq!(voting_address, expected);
    }

    #[test]
    fn should_refuse_a_zero_key_or_a_null_pointer_for_the_voting_address() {
        let key = [2u8; 32];
        let zero_key = [0u8; 32];
        let mut voting_address = [0xEEu8; 20];
        for (voting_private_key, out_voting_address) in [
            (zero_key.as_ptr(), voting_address.as_mut_ptr()),
            (std::ptr::null(), voting_address.as_mut_ptr()),
            (key.as_ptr(), std::ptr::null_mut()),
        ] {
            unsafe {
                let result = dash_sdk_masternode_voting_address_from_private_key(
                    voting_private_key,
                    out_voting_address,
                );
                assert!(result.data.is_null());
                assert!(!result.error.is_null(), "expected an error");
                let code = (*result.error).code;
                dash_sdk_error_free(result.error);
                assert_eq!(code, DashSDKErrorCode::InvalidParameter);
            }
        }
        // Nothing is written on a refusal
        assert_eq!(voting_address, [0xEEu8; 20]);
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
