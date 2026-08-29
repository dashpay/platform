//! Test-side helpers that drive identity-mutation flows on a
//! [`super::wallet_factory::RegisteredIdentity`] without re-implementing
//! the production wallet's transition wiring.
//!
//! Today this is just the ID-004 key-rotation helper used by TK-001c —
//! more identity-side operations land here as new test specs require
//! them.

use std::sync::Arc;
use std::time::Duration;

use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use key_wallet::wallet::root_extended_keys::RootExtendedPrivKey;
use platform_wallet::wallet::identity::network::derive_ecdsa_identity_auth_keypair_from_master;

use super::signer::derive_identity_key;
use super::wait::wait_for_identity_visible_to_platform;
use super::wallet_factory::{RegisteredIdentity, TestWallet};
use super::{FrameworkError, FrameworkResult};

/// Deadline for the post-rotation visibility gate. Mirrors the
/// `setup_with_n_identities` budget so a slow Platform replica
/// doesn't false-fail the rotation pin.
const POST_ROTATE_VISIBILITY_TIMEOUT: Duration = Duration::from_secs(60);

/// Number of `Identity::fetch` successes the post-rotation visibility
/// gate must observe. Two distinct sockets is the same streak the
/// post-registration gate uses.
const POST_ROTATE_VISIBILITY_STREAK: u32 = 2;

/// Rotate (add + disable) the AUTHENTICATION key on `identity` at the
/// caller-chosen `(new_key_index, purpose, security_level)` slot,
/// disabling the key currently sitting at `disable_key_id`.
///
/// On success:
///   1. The new key is broadcast to Platform via
///      `IdentityUpdateTransition` and confirmed visible.
///   2. The matching private bytes are injected into
///      `identity.signer` so subsequent state transitions sign with
///      the freshly-rotated key.
///   3. `identity.critical_key` is overwritten with the new
///      [`IdentityPublicKey`] when the rotation targets the CRITICAL
///      auth slot (the only `RegisteredIdentity` field that holds a
///      rotatable cached key today).
///
/// Returns the freshly-derived [`IdentityPublicKey`] so callers that
/// rotate non-CRITICAL slots (or want to inspect the new key
/// independently of the cached field) have direct access without
/// re-deriving.
///
/// Caveats:
/// - Cache layering — `update_identity_with_external_signer` already
///   bumps the cached `ManagedIdentity` revision and adds the new
///   key, but it explicitly does NOT stamp `disabled_at` on the
///   superseded entry (see the production code's `disable-keys`
///   TODO). For TK-001c that's acceptable: the test signs the
///   post-rotation transfer with the NEW key, so the local stale
///   `disabled_at` flag never matters.
/// - The new key must live in the seed's DIP-9 derivation tree —
///   `key_index` is hardened-derived from `test_wallet`'s seed at
///   `identity.identity_index`, so the new private bytes match the
///   public payload broadcast on chain.
pub async fn rotate_identity_authentication_key(
    test_wallet: &TestWallet,
    identity: &mut RegisteredIdentity,
    new_key_index: u32,
    purpose: Purpose,
    security_level: SecurityLevel,
    disable_key_id: u32,
) -> FrameworkResult<IdentityPublicKey> {
    let network = test_wallet.platform_wallet().sdk().network;
    let seed = test_wallet.seed_bytes();

    // Re-derive the secret alongside the public key so the cache
    // injection below uses the *same* bytes the broadcast keeps.
    let new_secret =
        derive_identity_secret(&seed, network, identity.identity_index, new_key_index)?;
    let new_public_key = derive_identity_key(
        &seed,
        network,
        identity.identity_index,
        new_key_index,
        purpose,
        security_level,
    )?;

    // Inject the new (pubkey-hash, secret) pair into the signer
    // BEFORE broadcast — `try_from_identity_with_signer` signs a
    // proof-of-possession against the new key as part of the
    // identity-update transition, so the signer must already resolve
    // the new key to its matching secret at that point.
    let signer_mut = Arc::make_mut(&mut identity.signer);
    let pubkey_compressed = compressed_pubkey(&new_public_key)?;
    signer_mut.inject_identity_key(&pubkey_compressed, new_secret);

    // Broadcast the add + disable in a single transition. The
    // production wallet handles MASTER-key selection internally
    // (DPP requires MASTER for identity-update); we just hand it the
    // identity id, the new key payload, and the id of the key being
    // retired.
    test_wallet
        .platform_wallet()
        .identity()
        .update_identity_with_external_signer(
            &identity.id,
            vec![new_public_key.clone()],
            vec![disable_key_id],
            identity.signer.as_ref(),
            None,
        )
        .await
        .map_err(|err| {
            FrameworkError::Wallet(format!(
                "rotate_identity_authentication_key: update_identity broadcast: {err}"
            ))
        })?;

    // Visibility gate — the post-rotation transition (a token
    // transfer in TK-001c) round-robins onto a sibling DAPI replica
    // that may not yet have seen the IdentityUpdate. Two
    // `Identity::fetch` successes mirror the post-registration gate
    // in `setup_with_n_identities`.
    wait_for_identity_visible_to_platform(
        test_wallet.platform_wallet().sdk(),
        identity.id,
        POST_ROTATE_VISIBILITY_TIMEOUT,
        POST_ROTATE_VISIBILITY_STREAK,
    )
    .await?;

    // Update the cached key reference on `RegisteredIdentity` so
    // tests sign subsequent transitions with the rotated key. Today
    // only the CRITICAL auth slot is wired through — other slots
    // surface via the returned `IdentityPublicKey` and the test is
    // responsible for routing.
    if purpose == Purpose::AUTHENTICATION && security_level == SecurityLevel::CRITICAL {
        identity.critical_key = new_public_key.clone();
    }

    Ok(new_public_key)
}

/// Re-derive the 32-byte secp256k1 secret for the DIP-9 identity
/// auth slot at `(identity_index, key_index)`.
///
/// Pulled out as a private helper because `derive_identity_key`
/// returns only the public payload and we need the secret bytes for
/// the signer cache injection. Keeps the seed handling in one place
/// rather than threading `RootExtendedPrivKey::new_master` through
/// the rotate body.
fn derive_identity_secret(
    seed: &[u8; 64],
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> FrameworkResult<[u8; 32]> {
    let root_priv = RootExtendedPrivKey::new_master(seed).map_err(|err| {
        FrameworkError::Wallet(format!(
            "rotate_identity_authentication_key: invalid seed for root xpriv: {err}"
        ))
    })?;
    let master = root_priv.to_extended_priv_key(network);
    let derived =
        derive_ecdsa_identity_auth_keypair_from_master(&master, network, identity_index, key_index)
            .map_err(|err| {
                FrameworkError::Wallet(format!(
            "rotate_identity_authentication_key: derive ({identity_index}, {key_index}): {err}"
        ))
            })?;
    Ok(*derived.private_key)
}

/// Extract the 33-byte compressed secp256k1 pubkey from an
/// [`IdentityPublicKey`] built via [`derive_identity_key`].
///
/// The helper only ever produces `ECDSA_SECP256K1` payloads, so the
/// `data` field carries the raw 33-byte public key — exactly the
/// shape the signer cache hashes at construction time.
fn compressed_pubkey(key: &IdentityPublicKey) -> FrameworkResult<[u8; 33]> {
    if key.key_type() != KeyType::ECDSA_SECP256K1 {
        return Err(FrameworkError::Wallet(format!(
            "rotate_identity_authentication_key: expected ECDSA_SECP256K1 key, got {:?}",
            key.key_type()
        )));
    }
    key.data().as_slice().try_into().map_err(|_| {
        FrameworkError::Wallet(format!(
            "rotate_identity_authentication_key: pubkey data length {} != 33",
            key.data().as_slice().len()
        ))
    })
}
