use crate::signer::SimpleSigner;
use dashcore_rpc::dashcore::secp256k1::SecretKey;
use dashcore_rpc::dashcore::{Network, PrivateKey};
use dpp::identifier::Identifier;
use dpp::identity::core_script::CoreScript;
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::identity::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::identity::state_transition::identity_credit_withdrawal_transition::{
    IdentityCreditWithdrawalTransition, Pooling,
};
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::identity::KeyType::ECDSA_SECP256K1;
use dpp::identity::Purpose::AUTHENTICATION;
use dpp::identity::SecurityLevel::{CRITICAL, MASTER};
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::state_transition::{
    StateTransition, StateTransitionIdentitySignedV0, StateTransitionType,
};
use dpp::tests::fixtures::instant_asset_lock_proof_fixture;
use dpp::version::{LATEST_VERSION, PlatformVersion};
use dpp::NativeBlsModule;
use rand::prelude::{IteratorRandom, StdRng};
use std::collections::HashSet;
use std::str::FromStr;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::withdrawal::Pooling;

pub fn create_identity_top_up_transition(
    rng: &mut StdRng,
    identity: &Identity,
    platform_version: &PlatformVersion,
) -> StateTransition {
    let (_, pk) = ECDSA_SECP256K1.random_public_and_private_key_data(rng, platform_version);
    let sk: [u8; 32] = pk.try_into().unwrap();
    let secret_key = SecretKey::from_str(hex::encode(sk).as_str()).unwrap();
    let asset_lock_proof =
        instant_asset_lock_proof_fixture(Some(PrivateKey::new(secret_key, Network::Dash)));

    StateTransition::IdentityTopUp(
        IdentityTopUpTransition::try_from_identity(
            identity.clone(),
            asset_lock_proof,
            secret_key.as_ref(),
            &NativeBlsModule::default(),
        )
        .expect("expected to create top up transition"),
    )
}

pub fn create_identity_update_transition_add_keys(
    identity: &mut Identity,
    count: u16,
    signer: &mut SimpleSigner,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> (StateTransition, (Identifier, Vec<IdentityPublicKey>)) {
    identity.revision += 1;
    let keys = IdentityPublicKey::random_authentication_keys_with_private_keys_with_rng(
        identity.public_keys().len() as KeyID,
        count as u32,
        rng,
        platform_version,
    );

    let add_public_keys: Vec<IdentityPublicKey> = keys.iter().map(|(key, _)| key.clone()).collect();
    signer.private_keys_in_creation.extend(keys);
    let (key_id, _) = identity
        .public_keys()
        .iter()
        .find(|(_, key)| key.security_level == MASTER)
        .expect("expected to have a master key");

    let state_transition = StateTransition::IdentityUpdate(
        IdentityUpdateTransition::try_from_identity_with_signer(
            identity,
            key_id,
            add_public_keys.clone(),
            vec![],
            None,
            signer,
        )
        .expect("expected to create top up transition"),
    );

    (state_transition, (identity.id, add_public_keys))
}

pub fn create_identity_update_transition_disable_keys(
    identity: &mut Identity,
    count: u16,
    block_time: u64,
    signer: &mut SimpleSigner,
    rng: &mut StdRng,
) -> Option<StateTransition> {
    identity.revision += 1;
    // we want to find keys that are not disabled
    let key_ids_we_could_disable = identity
        .public_keys()
        .iter()
        .filter(|(_, key)| {
            key.disabled_at.is_none()
                && (key.security_level != MASTER
                    && !(key.security_level == CRITICAL
                        && key.purpose == AUTHENTICATION
                        && key.key_type == ECDSA_SECP256K1))
        })
        .map(|(key_id, _)| *key_id)
        .collect::<Vec<_>>();

    if key_ids_we_could_disable.is_empty() {
        identity.revision -= 1; //since we added 1 before
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
                key.disabled_at = Some(block_time);
            }
        });

    let (key_id, _) = identity
        .public_keys()
        .iter()
        .find(|(_, key)| key.security_level == MASTER)
        .expect("expected to have a master key");

    let state_transition = StateTransition::IdentityUpdate(
        IdentityUpdateTransition::try_from_identity_with_signer(
            identity,
            key_id,
            vec![],
            key_ids_to_disable,
            Some(block_time),
            signer,
        )
        .expect("expected to create top up transition"),
    );

    Some(state_transition)
}

pub fn create_identity_withdrawal_transition(
    identity: &mut Identity,
    signer: &mut SimpleSigner,
    rng: &mut StdRng,
) -> StateTransition {
    identity.revision += 1;
    let mut withdrawal = IdentityCreditWithdrawalTransition {
        protocol_version: LATEST_VERSION,
        transition_type: StateTransitionType::IdentityCreditWithdrawal,
        identity_id: identity.id,
        amount: 100000000, // 0.001 Dash
        core_fee_per_byte: 1,
        pooling: Pooling::Never,
        output_script: CoreScript::random_p2sh(rng),
        revision: identity.revision,
        signature_public_key_id: 0,
        signature: Default::default(),
    };

    let identity_public_key = identity
        .get_first_public_key_matching(
            Purpose::AUTHENTICATION,
            HashSet::from([SecurityLevel::HIGH, SecurityLevel::CRITICAL]),
            HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
        )
        .expect("expected to get a signing key");

    withdrawal
        .sign_external(identity_public_key, signer)
        .expect("expected to sign withdrawal");

    withdrawal.into()
}

pub fn create_identity_credit_transfer_transition(
    identity: &Identity,
    recipient: &Identity,
    signer: &mut SimpleSigner,
    amount: u64,
) -> StateTransition {
    let mut transition = IdentityCreditTransferTransition {
        transition_type: StateTransitionType::IdentityCreditTransfer,
        identity_id: identity.id,
        recipient_id: recipient.id,
        amount,
        protocol_version: LATEST_VERSION,
        signature_public_key_id: 0,
        signature: Default::default(),
    };

    let identity_public_key = identity
        .get_first_public_key_matching(
            Purpose::AUTHENTICATION,
            HashSet::from([SecurityLevel::MASTER]),
            HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
        )
        .expect("expected to get a signing key");

    transition
        .sign_external(identity_public_key, signer)
        .expect("expected to sign transfer");

    transition.into()
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
    identities
        .into_iter()
        .map(|mut identity| {
            let (_, pk) = ECDSA_SECP256K1.random_public_and_private_key_data(rng, platform_version);
            let sk: [u8; 32] = pk.clone().try_into().unwrap();
            let secret_key = SecretKey::from_str(hex::encode(sk).as_str()).unwrap();
            let asset_lock_proof =
                instant_asset_lock_proof_fixture(Some(PrivateKey::new(secret_key, Network::Dash)));
            let identity_create_transition =
                IdentityCreateTransition::try_from_identity_with_signer(
                    identity.clone(),
                    asset_lock_proof,
                    pk.as_slice(),
                    signer,
                    &NativeBlsModule::default(),
                )
                .expect("expected to transform identity into identity create transition");
            identity.id = *identity_create_transition.get_identity_id();
            (identity, identity_create_transition.into())
        })
        .collect()
}
