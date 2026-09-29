// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Pure helpers the embedder calls on its GUI thread: the DPNS label rules,
//! protocol constants, and the DIP-15 pieces that need nothing beyond the
//! 32-byte outputs Core's wallet is willing to hand over (an ECDH secret, an
//! accountReference MAC). No key material is derived here.

use dash_sdk::dpp::balances::credits::CREDITS_PER_DUFF;
use dash_sdk::dpp::identity::{KeyType, Purpose};
use dash_sdk::dpp::system_data_contracts::SystemDataContract;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::platform::dashpay::{
    recipient_key_purpose_is_acceptable_on_receive, recipient_key_purpose_is_valid,
    sender_key_purpose_is_acceptable_on_receive,
};
pub use dash_sdk::platform::dpns_usernames::{
    convert_to_homograph_safe_chars as normalize_label, is_contested_username, is_valid_username,
};
use platform_encryption::{compact_xpub_bytes, decrypt_extended_public_key, parse_compact_xpub};

use crate::ffi;

/// Credits per duff, the unit the GUI converts identity balances with.
pub const fn credits_per_duff() -> u64 {
    CREDITS_PER_DUFF
}

/// Id of a compiled-in system contract.
pub fn system_contract_id(which: ffi::SystemContract) -> Result<[u8; 32], String> {
    let contract = match which {
        ffi::SystemContract::Dpns => SystemDataContract::DPNS,
        ffi::SystemContract::Dashpay => SystemDataContract::Dashpay,
        other => return Err(format!("unknown system contract {}", other.repr)),
    };
    Ok(contract.id().to_buffer())
}

/// Credits a contested DPNS domain registration must prefund for the vote
/// resolution, under `version`'s fee schedule.
pub fn contested_vote_fund_credits(version: &PlatformVersion) -> u64 {
    version
        .fee_version
        .vote_resolution_fund_fees
        .contested_document_vote_resolution_fund_required_amount
}

/// The 69-byte DIP-15 compact xpub of a contact request's
/// `encryptedPublicKey`, serialized for the C++ side.
pub fn compact_xpub_to_bytes(xpub: &ffi::CompactXpub) -> Vec<u8> {
    compact_xpub_bytes(xpub.parent_fingerprint, xpub.chain_code, xpub.public_key).to_vec()
}

/// Decrypts a contact request's `encryptedPublicKey` (16-byte IV followed by
/// the AES-256-CBC ciphertext) with the ECDH secret Core derived.
pub fn dip15_decrypt_xpub(
    shared_secret: &[u8; 32],
    ciphertext: &[u8],
) -> Result<ffi::CompactXpub, String> {
    let plaintext = decrypt_extended_public_key(shared_secret, ciphertext)
        .map_err(|e| format!("unable to decrypt the contact's public key: {e}"))?;
    let xpub = parse_compact_xpub(&plaintext).map_err(|e| e.to_string())?;
    Ok(ffi::CompactXpub {
        parent_fingerprint: xpub.parent_fingerprint,
        chain_code: xpub.chain_code,
        public_key: xpub.public_key,
    })
}

/// `ASK28`: the 28 low bits of the big-endian tail of the accountReference
/// MAC, the reading iOS dash-shared-core uses (`platform-encryption`'s
/// `extract_ask28`, inlined until it is exported).
fn ask28(mac: &[u8; 32]) -> u32 {
    u32::from_be_bytes([mac[28], mac[29], mac[30], mac[31]]) >> 4
}

/// Masks `account_index` into a DIP-15 `accountReference` carrying the
/// rotation `version` in its top four bits (`platform-encryption`'s
/// `calculate_account_reference` over a MAC Core computed with the
/// ENCRYPTION key it never exports).
pub fn dip15_account_reference_from_mac(mac: &[u8; 32], account_index: u32, version: u32) -> u32 {
    (version << 28) | (ask28(mac) ^ (account_index & 0x0FFF_FFFF))
}

/// Inverse of [`dip15_account_reference_from_mac`] for the same MAC.
pub fn dip15_unmask_account_reference_from_mac(
    mac: &[u8; 32],
    account_reference: u32,
) -> ffi::AccountRef {
    ffi::AccountRef {
        version: account_reference >> 28,
        account_index: (account_reference & 0x0FFF_FFFF) ^ ask28(mac),
    }
}

/// The recipient key a contact request to `identity` should reference: an
/// enabled ECDSA key whose purpose `dash-sdk` accepts when minting, a
/// DECRYPTION key before an ENCRYPTION one, lowest id first.
pub fn dip15_select_recipient_key(identity: &ffi::Identity) -> Result<u32, String> {
    let mut eligible: Vec<(u32, Purpose)> = identity
        .keys
        .iter()
        .filter_map(|key| {
            let purpose = Purpose::try_from(key.purpose).ok()?;
            let key_type = KeyType::try_from(key.key_type).ok()?;
            (recipient_key_purpose_is_valid(purpose)
                && key_type == KeyType::ECDSA_SECP256K1
                && key.disabled_at == 0)
                .then_some((key.id, purpose))
        })
        .collect();
    eligible.sort_by_key(|(id, purpose)| (*purpose != Purpose::DECRYPTION, *id));
    eligible.first().map(|(id, _)| *id).ok_or_else(|| {
        "the recipient has no enabled ECDSA DECRYPTION or ENCRYPTION key".to_string()
    })
}

/// Whether an inbound contact request's key references are acceptable for
/// the ECDH that unwraps its `encryptedPublicKey`: `dash-sdk`'s receive-side
/// purpose policy, plus the rule that Core never runs ECDH with its MASTER
/// key (id 0).
pub fn dip15_receive_keys_acceptable(
    sender_purpose: u8,
    recipient_purpose: u8,
    recipient_key_id: u32,
) -> bool {
    let (Ok(sender), Ok(recipient)) = (
        Purpose::try_from(sender_purpose),
        Purpose::try_from(recipient_purpose),
    ) else {
        return false;
    };
    recipient_key_id >= 1
        && sender_key_purpose_is_acceptable_on_receive(sender)
        && recipient_key_purpose_is_acceptable_on_receive(recipient)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(id: u32, purpose: Purpose, key_type: KeyType, disabled_at: u64) -> ffi::IdentityKey {
        ffi::IdentityKey {
            id,
            purpose: purpose as u8,
            key_type: key_type as u8,
            disabled_at,
            ..Default::default()
        }
    }

    #[test]
    fn recipient_key_prefers_decryption_then_lowest_id() {
        let identity = ffi::Identity {
            keys: vec![
                key(0, Purpose::AUTHENTICATION, KeyType::ECDSA_SECP256K1, 0),
                key(2, Purpose::ENCRYPTION, KeyType::ECDSA_SECP256K1, 0),
                key(3, Purpose::DECRYPTION, KeyType::ECDSA_SECP256K1, 0),
                key(4, Purpose::DECRYPTION, KeyType::ECDSA_SECP256K1, 0),
            ],
            ..Default::default()
        };
        assert_eq!(dip15_select_recipient_key(&identity), Ok(3));
    }

    #[test]
    fn recipient_key_skips_disabled_and_non_ecdsa_keys() {
        let identity = ffi::Identity {
            keys: vec![
                key(2, Purpose::DECRYPTION, KeyType::ECDSA_SECP256K1, 1),
                key(3, Purpose::DECRYPTION, KeyType::BLS12_381, 0),
                key(5, Purpose::ENCRYPTION, KeyType::ECDSA_SECP256K1, 0),
            ],
            ..Default::default()
        };
        assert_eq!(dip15_select_recipient_key(&identity), Ok(5));
        let none = ffi::Identity {
            keys: vec![key(1, Purpose::AUTHENTICATION, KeyType::ECDSA_SECP256K1, 0)],
            ..Default::default()
        };
        assert!(dip15_select_recipient_key(&none).is_err());
    }

    #[test]
    fn receive_policy_never_accepts_the_master_key() {
        let encryption = Purpose::ENCRYPTION as u8;
        let authentication = Purpose::AUTHENTICATION as u8;
        assert!(dip15_receive_keys_acceptable(encryption, encryption, 2));
        // The legacy Android cohort references AUTHENTICATION keys on both
        // sides; still payable, as long as it is not key 0.
        assert!(dip15_receive_keys_acceptable(
            authentication,
            authentication,
            1
        ));
        assert!(!dip15_receive_keys_acceptable(
            authentication,
            authentication,
            0
        ));
        assert!(!dip15_receive_keys_acceptable(
            Purpose::VOTING as u8,
            encryption,
            2
        ));
        assert!(!dip15_receive_keys_acceptable(
            encryption,
            Purpose::SYSTEM as u8,
            2
        ));
        assert!(!dip15_receive_keys_acceptable(200, encryption, 2));
    }

    #[test]
    fn account_reference_round_trips_and_pins_the_ios_extraction() {
        let mac: [u8; 32] = std::array::from_fn(|i| i as u8);
        // be(mac[28..32]) >> 4 for 28,29,30,31 = 0x1c1d1e1f >> 4.
        assert_eq!(ask28(&mac), 0x01c1_d1e1);
        for version in [0u32, 1, 7, 15] {
            for account in [0u32, 1, 5, 0x0FFF_FFFF] {
                let reference = dip15_account_reference_from_mac(&mac, account, version);
                let unmasked = dip15_unmask_account_reference_from_mac(&mac, reference);
                assert_eq!(
                    (unmasked.version, unmasked.account_index),
                    (version, account)
                );
            }
        }
        assert_eq!(dip15_account_reference_from_mac(&mac, 0, 0), 0x01c1_d1e1);
    }

    #[test]
    fn xpub_round_trip_through_dip15_encryption() {
        let secret = [0x42u8; 32];
        let xpub = ffi::CompactXpub {
            parent_fingerprint: [1, 2, 3, 4],
            chain_code: [0x55; 32],
            public_key: std::array::from_fn(|i| if i == 0 { 2 } else { i as u8 }),
        };
        let ciphertext = platform_encryption::encrypt_extended_public_key(
            &secret,
            &[7u8; 16],
            &compact_xpub_to_bytes(&xpub),
        );
        assert_eq!(ciphertext.len(), 96);
        let decrypted = dip15_decrypt_xpub(&secret, &ciphertext).expect("decrypts");
        assert_eq!(decrypted.parent_fingerprint, xpub.parent_fingerprint);
        assert_eq!(decrypted.chain_code, xpub.chain_code);
        assert_eq!(decrypted.public_key, xpub.public_key);
        assert!(dip15_decrypt_xpub(&[0u8; 32], &ciphertext).is_err());
        assert!(dip15_decrypt_xpub(&secret, &ciphertext[..40]).is_err());
    }

    #[test]
    fn system_contract_ids_match_dpp() {
        assert_eq!(
            system_contract_id(ffi::SystemContract::Dpns),
            Ok(SystemDataContract::DPNS.id().to_buffer())
        );
        assert_eq!(
            system_contract_id(ffi::SystemContract::Dashpay),
            Ok(SystemDataContract::Dashpay.id().to_buffer())
        );
        assert!(system_contract_id(ffi::SystemContract { repr: 9 }).is_err());
    }
}
