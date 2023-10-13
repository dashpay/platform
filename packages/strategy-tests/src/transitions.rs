use dashcore_rpc::dashcore::secp256k1::SecretKey;
use dashcore_rpc::dashcore::{Network, PrivateKey};
use dpp::dashcore::secp256k1::Secp256k1;
use dpp::dashcore::{
    bls_sig_utils::BLSSignature, hash_types::CycleHash, InstantLock, OutPoint, ScriptBuf,
    Transaction, TxIn, TxOut, Txid,
};
use dpp::identifier::Identifier;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::core_script::CoreScript;
use dpp::identity::identity_public_key::accessors::v0::{
    IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
};
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::identity::KeyType::ECDSA_SECP256K1;
use dpp::identity::Purpose::AUTHENTICATION;
use dpp::identity::SecurityLevel::{CRITICAL, MASTER};
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::prelude::AssetLockProof;
use dpp::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;

use dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;

use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::{GetDataContractSecurityLevelRequirementFn, StateTransition};
use dpp::version::PlatformVersion;
use dpp::withdrawal::Pooling;
use dpp::NativeBlsModule;
use simple_signer::signer::SimpleSigner;
use rand::prelude::{IteratorRandom, StdRng};

use std::collections::HashSet;
use std::str::FromStr;

/// Constructs an `AssetLockProof` representing an instant asset lock proof.
///
/// Asset locking is a mechanism that temporarily restricts the movement of assets within a blockchain system, often used in protocols that require certain collateral conditions to be met. The "instant asset lock" variant provides a quick way to lock assets without waiting for confirmations.
///
/// This function simulates the creation of an instant asset lock proof by combining a constructed transaction with its corresponding instant lock. The resultant `AssetLockProof` signifies the locking of the asset.
///
/// # Parameters
/// - `one_time_private_key`: A unique private key (`PrivateKey`) utilized for generating the underlying locking transaction.
///
/// # Returns
/// - `AssetLockProof`: An asset lock proof derived from the instant asset lock mechanism. Specifically, it contains:
///   1. An `InstantAssetLockProof`, composed of:
///      - An `InstantLock`, which represents the instant locking conditions.
///      - A `Transaction`, representing the transaction where the assets are locked.
///      - The index (`0` in this case) indicating which output in the transaction corresponds to the locked asset.
///
/// # Examples
/// ```rust
/// let one_time_private_key = PrivateKey::from_str("some_valid_private_key").unwrap();
/// let asset_lock_proof = instant_asset_lock_proof_fixture(one_time_private_key);
/// ```
///
/// # Panics
/// This function may panic if there's an error in generating the underlying transaction or instant lock, typically due to the provided `one_time_private_key` or the hardcoded data within the helper functions.
pub fn instant_asset_lock_proof_fixture(one_time_private_key: PrivateKey) -> AssetLockProof {
    let transaction = instant_asset_lock_proof_transaction_fixture(one_time_private_key);

    let instant_lock = instant_asset_lock_is_lock_fixture(transaction.txid());

    let is_lock_proof = InstantAssetLockProof::new(instant_lock, transaction, 0);

    AssetLockProof::Instant(is_lock_proof)
}

/// Constructs a fixture of a `Transaction` representing an instant asset lock proof.
///
/// The `Transaction` structure is a basic unit of data in a blockchain, recording the transfer of assets between parties.
///
/// This function simulates the creation of a transaction where assets are locked using an "instant asset lock" mechanism, typically used in systems that support locking assets immediately upon transaction broadcast without waiting for confirmations.
///
/// # Parameters
/// - `one_time_private_key`: A unique private key (`PrivateKey`) to be used for generating the locking transaction. This key typically corresponds to a one-time address where assets will be locked.
///
/// # Returns
/// - `Transaction`: A constructed transaction with a specific structure: 
///   - An input that spends from a predetermined `Txid` and address.
///   - Three outputs:
///     1. A burn output, which is an `OP_RETURN` output containing the hash of the one-time public key, effectively "locking" the assets.
///     2. A change output, which returns unspent assets to the original sender's address.
///     3. An unrelated burn output with arbitrary data.
///
/// # Examples
/// ```rust
/// let one_time_private_key = PrivateKey::from_str("some_valid_private_key").unwrap();
/// let transaction = instant_asset_lock_proof_transaction_fixture(one_time_private_key);
/// ```
///
/// # Panics
/// This function may panic if there's an error in converting the hardcoded strings to their respective types (`PrivateKey`, `Txid`).
pub fn instant_asset_lock_proof_transaction_fixture(
    one_time_private_key: PrivateKey,
) -> Transaction {
    let secp = Secp256k1::new();

    let private_key_hex = "cSBnVM4xvxarwGQuAfQFwqDg9k5tErHUHzgWsEfD4zdwUasvqRVY";
    let private_key = PrivateKey::from_str(private_key_hex).unwrap();
    let public_key = private_key.public_key(&secp);
    let public_key_hash = public_key.pubkey_hash();
    //let from_address = Address::p2pkh(&public_key, Network::Testnet);
    let one_time_public_key = one_time_private_key.public_key(&secp);

    let txid =
        Txid::from_str("a477af6b2667c29670467e4e0728b685ee07b240235771862318e29ddbe58458").unwrap();
    let outpoint = OutPoint::new(txid, 0);
    let input = TxIn {
        previous_output: outpoint,
        script_sig: ScriptBuf::new_p2pkh(&public_key_hash),
        sequence: 0,
        witness: Default::default(),
    };
    let one_time_key_hash = one_time_public_key.pubkey_hash();
    let burn_output = TxOut {
        value: 100000000, // 1 Dash
        script_pubkey: ScriptBuf::new_op_return(&one_time_key_hash),
    };
    let change_output = TxOut {
        value: 5000,
        script_pubkey: ScriptBuf::new_p2pkh(&public_key_hash),
    };
    let unrelated_burn_output = TxOut {
        value: 5000,
        script_pubkey: ScriptBuf::new_op_return(&[1, 2, 3]),
    };
    Transaction {
        version: 0,
        lock_time: 0,
        input: vec![input],
        output: vec![burn_output, change_output, unrelated_burn_output],
        special_transaction_payload: None,
    }
}

/// Constructs a fixture of `InstantLock` representing an instant asset lock.
///
/// The `InstantLock` structure is often used in blockchain systems to represent a condition where funds (or assets) are locked instantly, making them non-spendable until a specified condition is met or the lock duration expires.
///
/// This function is particularly useful in testing scenarios where you need to generate predictable instant asset locks.
///
/// # Parameters
/// - `tx_id`: The transaction ID (`Txid`) for which this instant lock is being created.
///
/// # Returns
/// - `InstantLock`: A constructed `InstantLock` with predefined values for version, inputs, cyclehash, and signature, but with the `tx_id` as provided in the function argument.
///
/// # Examples
/// ```rust
/// let tx_id = Txid::from_str("some_valid_tx_id").unwrap();
/// let instant_lock = instant_asset_lock_is_lock_fixture(tx_id);
/// ```
///
/// # Panics
/// This function may panic if there's an error in converting the hardcoded strings to their respective types (`Txid`, `CycleHash`, and `BLSSignature`).
pub fn instant_asset_lock_is_lock_fixture(tx_id: Txid) -> InstantLock {
    InstantLock {
        version: 1,
        inputs: vec![
            OutPoint { txid: Txid::from_str("6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d").unwrap(), vout: 0 }
        ],
        txid: tx_id,
        cyclehash: CycleHash::from_str("7c30826123d0f29fe4c4a8895d7ba4eb469b1fafa6ad7b23896a1a591766a536").unwrap(),
        signature: BLSSignature::from_str("8967c46529a967b3822e1ba8a173066296d02593f0f59b3a78a30a7eef9c8a120847729e62e4a32954339286b79fe7590221331cd28d576887a263f45b595d499272f656c3f5176987c976239cac16f972d796ad82931d532102a4f95eec7d80").unwrap(),
    }
}

/// Constructs a state transition for topping up the balance of a given identity.
///
/// The function achieves this by doing the following:
/// 1. It generates a random public and private key pair using the ECDSA_SECP256K1 algorithm.
/// 2. It uses the generated private key to create an asset lock proof. This proves the commitment of some asset (e.g., Dash) which will be used to top up the identity.
/// 3. Using the identity and the asset lock proof, it constructs the identity top-up state transition.
///
/// # Parameters
/// - `rng`: A mutable reference to a random number generator, used for generating the new public and private key pair.
/// - `identity`: A reference to the identity that needs its balance to be topped up.
/// - `platform_version`: A reference to the platform version for compatibility purposes.
///
/// # Returns
/// - `StateTransition`: A constructed and signed state transition that represents the identity top-up action.
///
/// # Examples
/// ```rust
/// let top_up_transition = create_identity_top_up_transition(
///     &mut rng,
///     &identity,
///     &platform_version,
/// );
/// ```
///
/// # Panics
/// This function may panic under the following conditions:
/// - If there's an error during the random key generation.
/// - If there's an error in converting the generated public key to its private counterpart.
/// - If there's an error during the creation of the identity top-up transition.
pub fn create_identity_top_up_transition(
    rng: &mut StdRng,
    identity: &Identity,
    platform_version: &PlatformVersion,
) -> StateTransition {
    let (_, pk) = ECDSA_SECP256K1
        .random_public_and_private_key_data(rng, platform_version)
        .unwrap();
    let sk: [u8; 32] = pk.try_into().unwrap();
    let secret_key = SecretKey::from_str(hex::encode(sk).as_str()).unwrap();
    let asset_lock_proof =
        instant_asset_lock_proof_fixture(PrivateKey::new(secret_key, Network::Dash));

    IdentityTopUpTransition::try_from_identity(
        identity.clone(),
        asset_lock_proof,
        secret_key.as_ref(),
        &NativeBlsModule,
        platform_version,
        None,
    )
    .expect("expected to create top up transition")
}

/// Creates a state transition for updating an identity by adding a specified number of new public authentication keys.
///
/// This function performs the following key steps:
/// 1. Increments the revision of the identity to represent a new version.
/// 2. Generates a specified number (`count`) of random public authentication keys.
/// 3. Extends the signer with these generated keys.
/// 4. Constructs the identity update state transition incorporating these new keys.
///
/// # Parameters
/// - `identity`: A mutable reference to the identity being updated.
/// - `count`: The number of new authentication public keys to be added to the identity.
/// - `signer`: A mutable reference to the signer, used for creating cryptographic signatures and managing key data.
/// - `rng`: A mutable reference to a random number generator, used for generating the new public authentication keys.
/// - `platform_version`: A reference to the platform version for compatibility purposes.
///
/// # Returns
/// - `(StateTransition, (Identifier, Vec<IdentityPublicKey>))`: A tuple consisting of:
///   * The constructed and signed state transition representing the identity update.
///   * An identifier of the identity being updated.
///   * A vector of the newly added public authentication keys.
///
/// # Examples
/// ```rust
/// let (update_transition, (id, added_keys)) = create_identity_update_transition_add_keys(
///     &mut identity,
///     2,
///     &mut signer,
///     &mut rng,
///     &platform_version,
/// );
/// ```
///
/// # Panics
/// This function may panic under the following conditions:
/// - If the identity does not have a master key.
/// - If there's an error during the random key generation.
/// - If there's an error during the creation of the identity update transition.
pub fn create_identity_update_transition_add_keys(
    identity: &mut Identity,
    count: u16,
    signer: &mut SimpleSigner,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> (StateTransition, (Identifier, Vec<IdentityPublicKey>)) {
    identity.bump_revision();
    let keys = IdentityPublicKey::random_authentication_keys_with_private_keys_with_rng(
        identity.public_keys().len() as KeyID,
        count as u32,
        rng,
        platform_version,
    )
    .expect("expected to get random keys");

    let add_public_keys: Vec<IdentityPublicKey> = keys.iter().map(|(key, _)| key.clone()).collect();
    signer.private_keys_in_creation.extend(keys);
    let (key_id, _) = identity
        .public_keys()
        .iter()
        .find(|(_, key)| key.security_level() == MASTER)
        .expect("expected to have a master key");

    let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
        identity,
        key_id,
        add_public_keys.clone(),
        vec![],
        None,
        signer,
        platform_version,
        None,
    )
    .expect("expected to create top up transition");

    (state_transition, (identity.id(), add_public_keys))
}

/// Creates a state transition for updating an identity by disabling a certain number of its public keys.
///
/// This function performs the following key steps:
/// 1. Increments the revision of the identity to represent a new version.
/// 2. Filters the identity's public keys to identify those which are not disabled, except for the 
///    master key or critical authentication keys using ECDSA_SECP256K1.
/// 3. Randomly selects a set of keys based on the provided count to disable.
/// 4. Marks these keys as disabled using the given block time.
/// 5. Constructs the identity update state transition with the changes.
///
/// # Parameters
/// - `identity`: A mutable reference to the identity being updated.
/// - `count`: The number of keys that should be disabled.
/// - `block_time`: The block timestamp to set as the disabled timestamp for keys.
/// - `signer`: A mutable reference to the signer utilized to create the cryptographic signature for 
///   the state transition.
/// - `rng`: A mutable reference to a random number generator, used for selecting which keys to disable.
/// - `platform_version`: A reference to the platform version for compatibility purposes.
///
/// # Returns
/// - `Option<StateTransition>`: The constructed and signed state transition representing the identity update. 
///   Returns `None` if there are no keys that can be disabled.
///
/// # Examples
/// ```rust
/// let update_transition = create_identity_update_transition_disable_keys(
///     &mut identity,
///     2,
///     current_block_time,
///     &mut signer,
///     &mut rng,
///     &platform_version,
/// );
/// ```
///
/// # Panics
/// This function may panic under the following conditions:
/// - If the identity does not have a master key.
/// - If there's an error during the creation of the identity update transition.
pub fn create_identity_update_transition_disable_keys(
    identity: &mut Identity,
    count: u16,
    block_time: u64,
    signer: &mut SimpleSigner,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> Option<StateTransition> {
    identity.bump_revision();
    // we want to find keys that are not disabled
    let key_ids_we_could_disable = identity
        .public_keys()
        .iter()
        .filter(|(_, key)| {
            key.disabled_at().is_none()
                && (key.security_level() != MASTER
                    && !(key.security_level() == CRITICAL
                        && key.purpose() == AUTHENTICATION
                        && key.key_type() == ECDSA_SECP256K1))
        })
        .map(|(key_id, _)| *key_id)
        .collect::<Vec<_>>();

    if key_ids_we_could_disable.is_empty() {
        identity.set_revision(identity.revision() - 1); //since we added 1 before
        return None;
    }
    let indices: Vec<_> = (0..key_ids_we_could_disable.len()).choose_multiple(rng, count as usize);

    let key_ids_to_disable: Vec<_> = indices
        .into_iter()
        .map(|index| key_ids_we_could_disable[index])
        .collect();

    identity
        .public_keys_mut()
        .iter_mut()
        .for_each(|(key_id, key)| {
            if key_ids_to_disable.contains(key_id) {
                key.set_disabled_at(block_time);
            }
        });

    let (key_id, _) = identity
        .public_keys()
        .iter()
        .find(|(_, key)| key.security_level() == MASTER)
        .expect("expected to have a master key");

    let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
        identity,
        key_id,
        vec![],
        key_ids_to_disable,
        Some(block_time),
        signer,
        platform_version,
        None,
    )
    .expect("expected to create top up transition");

    Some(state_transition)
}

/// Creates a state transition for an identity's credit withdrawal.
///
/// This function generates a state transition representing the withdrawal of credits from an identity.
/// The withdrawal amount is set to 0.001 Dash. The function first bumps the revision 
/// of the identity and then constructs the withdrawal transition. Subsequently, it's signed using the 
/// identity's authentication key for validity and authenticity.
///
/// # Parameters
/// - `identity`: A mutable reference to the identity making the withdrawal.
/// - `signer`: A mutable reference to the signer used to create the cryptographic signature for 
///   the state transition.
/// - `rng`: A mutable reference to a random number generator, used for generating the random Pay-To-Script-Hash (P2SH).
///
/// # Returns
/// - `StateTransition`: The constructed and signed state transition representing the identity's credit withdrawal.
///
/// # Examples
/// ```rust
/// let withdrawal_transition = create_identity_withdrawal_transition(
///     &mut identity,
///     &mut signer,
///     &mut rng,
/// );
/// ```
///
/// # Panics
/// This function may panic under the following conditions:
/// - If the identity does not have a suitable authentication key for signing.
/// - If there's an error during the signing process.
pub fn create_identity_withdrawal_transition(
    identity: &mut Identity,
    signer: &mut SimpleSigner,
    rng: &mut StdRng,
) -> StateTransition {
    identity.bump_revision();
    let mut withdrawal: StateTransition = IdentityCreditWithdrawalTransitionV0 {
        identity_id: identity.id(),
        amount: 100000000, // 0.001 Dash
        core_fee_per_byte: 1,
        pooling: Pooling::Never,
        output_script: CoreScript::random_p2sh(rng),
        revision: identity.revision(),
        signature_public_key_id: 0,
        signature: Default::default(),
    }
    .into();

    let identity_public_key = identity
        .get_first_public_key_matching(
            Purpose::AUTHENTICATION,
            HashSet::from([SecurityLevel::CRITICAL]),
            HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
        )
        .expect("expected to get a signing key");

    withdrawal
        .sign_external(
            identity_public_key,
            signer,
            None::<GetDataContractSecurityLevelRequirementFn>,
        )
        .expect("expected to sign withdrawal");

    withdrawal
}

/// Creates a state transition for transferring credits between two identities.
///
/// This function generates a state transition that represents the transfer of a specified 
/// amount of credits from one identity (`identity`) to another (`recipient`). 
/// After constructing the transition, it's then signed using the sender's (identity's) 
/// authentication key to ensure its validity and authenticity.
///
/// # Parameters
/// - `identity`: A reference to the identity that is the sender of the credit transfer.
/// - `recipient`: A reference to the identity that is the recipient of the credit transfer.
/// - `signer`: A mutable reference to a signer, used for creating the cryptographic signature 
///   for the state transition.
/// - `amount`: The number of credits to be transferred from the sender to the recipient.
///
/// # Returns
/// - `StateTransition`: The constructed and signed state transition representing the credit transfer 
///   between the two specified identities.
///
/// # Examples
/// ```rust
/// let transfer_transition = create_identity_credit_transfer_transition(
///     &sender_identity,
///     &recipient_identity,
///     &mut signer,
///     1000,
/// );
/// ```
///
/// # Panics
/// This function may panic under the following conditions:
/// - If the sender's identity does not have a suitable authentication key available for signing.
/// - If there's an error during the signing process.
pub fn create_identity_credit_transfer_transition(
    identity: &Identity,
    recipient: &Identity,
    signer: &mut SimpleSigner,
    amount: u64,
) -> StateTransition {
    let mut transition: StateTransition = IdentityCreditTransferTransitionV0 {
        identity_id: identity.id(),
        recipient_id: recipient.id(),
        amount,
        signature_public_key_id: 0,
        signature: Default::default(),
    }
    .into();

    let identity_public_key = identity
        .get_first_public_key_matching(
            Purpose::AUTHENTICATION,
            HashSet::from([SecurityLevel::CRITICAL]),
            HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
        )
        .expect("expected to get a signing key");

    transition
        .sign_external(
            identity_public_key,
            signer,
            None::<GetDataContractSecurityLevelRequirementFn>,
        )
        .expect("expected to sign transfer");

    transition
}

/// Generates a specified number of new identities and their corresponding state transitions.
///
/// This function first creates a specified number of random identities along with their 
/// associated cryptographic keys. After generating these identities and their keys, it adds 
/// the keys to the signer and then creates the state transitions representing the creation 
/// of these identities on the blockchain.
///
/// # Parameters
/// - `count`: The number of identities to generate and for which state transitions will be created.
/// - `key_count`: The number of cryptographic keys to generate for each identity.
/// - `signer`: A mutable reference to a signer, used for creating cryptographic signatures for 
///   the state transitions.
/// - `rng`: A mutable reference to a random number generator, used to generate random values during 
///   the cryptographic key creation process and while generating the random identities.
/// - `platform_version`: A reference to the version of the platform being used. Ensuring the correct 
///   platform version is used is crucial for compatibility and consistency in state transition creation.
///
/// # Returns
/// A vector of tuples, where each tuple contains:
/// 1. `Identity`: The generated random identity object.
/// 2. `StateTransition`: The generated state transition representing the creation of the identity.
///
/// # Examples
/// ```rust
/// let transitions = create_identities_state_transitions(
///     10,
///     KeyID::default(),
///     &mut signer,
///     &mut rng,
///     &platform_version,
/// );
/// ```
///
/// # Panics
/// This function may panic under the following conditions:
/// - When unable to generate random cryptographic keys or identities.
/// - Conversion and encoding errors related to the cryptographic data.
pub fn create_identities_state_transitions(
    count: u16,
    key_count: KeyID,
    signer: &mut SimpleSigner,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> Vec<(Identity, StateTransition)> {
    let (identities, keys) = Identity::random_identities_with_private_keys_with_rng::<Vec<_>>(
        count,
        key_count,
        rng,
        platform_version,
    )
    .expect("expected to create identities");
    signer.add_keys(keys);
    create_state_transitions_for_identities(identities, signer, rng, platform_version)
}

/// Generates state transitions for the creation of new identities.
///
/// This function is responsible for converting identities into their respective state transitions,
/// which represent their creation on the blockchain. The process involves generating cryptographic 
/// keys, creating an asset lock proof, and then constructing the identity creation state transition 
/// using the provided data.
///
/// # Parameters
/// - `identities`: A vector containing the identities for which state transitions are to be created.
/// - `signer`: A mutable reference to a signer, used for creating cryptographic signatures for 
///   the state transitions.
/// - `rng`: A mutable reference to a random number generator, used to generate random values during 
///   the cryptographic key creation process.
/// - `platform_version`: A reference to the version of the platform being used. Ensuring the correct 
///   platform version is used is crucial for compatibility and consistency in state transition creation.
///
/// # Returns
/// A vector of tuples, where each tuple contains:
/// 1. `Identity`: The original identity object.
/// 2. `StateTransition`: The generated state transition representing the creation of the identity.
///
/// # Examples
/// ```rust
/// let transitions = create_state_transitions_for_identities(
///     identities,
///     &mut signer,
///     &mut rng,
///     &platform_version,
/// );
/// ```
///
/// # Panics
/// This function may panic under several conditions:
/// - When unable to generate random cryptographic keys.
/// - When failing to transform an identity into its corresponding state transition.
/// - Conversion and encoding errors related to the cryptographic data.
pub fn create_state_transitions_for_identities(
    identities: Vec<Identity>,
    signer: &mut SimpleSigner,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> Vec<(Identity, StateTransition)> {
    identities
        .into_iter()
        .map(|mut identity| {
            let (_, pk) = ECDSA_SECP256K1
                .random_public_and_private_key_data(rng, platform_version)
                .unwrap();
            let sk: [u8; 32] = pk.clone().try_into().unwrap();
            let secret_key = SecretKey::from_str(hex::encode(sk).as_str()).unwrap();
            let asset_lock_proof =
                instant_asset_lock_proof_fixture(PrivateKey::new(secret_key, Network::Dash));
            let identity_create_transition =
                IdentityCreateTransition::try_from_identity_with_signer(
                    identity.clone(),
                    asset_lock_proof,
                    pk.as_slice(),
                    signer,
                    &NativeBlsModule,
                    platform_version,
                )
                .expect("expected to transform identity into identity create transition");
            identity.set_id(identity_create_transition.owner_id());

            (identity, identity_create_transition)
        })
        .collect()
}
