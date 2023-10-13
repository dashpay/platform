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
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;

use dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;

use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::{GetDataContractSecurityLevelRequirementFn, StateTransition};
use dpp::util::vec::hex_to_array;
use dpp::version::PlatformVersion;
use dpp::withdrawal::Pooling;
use dpp::NativeBlsModule;
use rand::prelude::{IteratorRandom, StdRng};
use simple_signer::signer::SimpleSigner;

use std::collections::HashSet;
use std::str::FromStr;

pub fn instant_asset_lock_proof_fixture(one_time_private_key: PrivateKey) -> AssetLockProof {
    let transaction = instant_asset_lock_proof_transaction_fixture(one_time_private_key);

    let instant_lock = instant_asset_lock_is_lock_fixture(transaction.txid());

    let is_lock_proof = InstantAssetLockProof::new(instant_lock, transaction, 0);

    AssetLockProof::Instant(is_lock_proof)
}

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
