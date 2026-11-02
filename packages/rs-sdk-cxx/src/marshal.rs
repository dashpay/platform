// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Conversions between the bridge's plain structs and dpp types.

use std::collections::BTreeMap;

use dash_sdk::dpp::dashcore::consensus::Decodable;
use dash_sdk::dpp::dashcore::{InstantLock, OutPoint, Transaction};
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dash_sdk::dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dash_sdk::dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::AssetLockProof;
use dash_sdk::dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dash_sdk::platform::dpns_usernames::DpnsUsername;
use dash_sdk::query_types::Contenders;

use crate::ffi;

/// A quorum hash between the embedder's internal `uint256` byte order and
/// the order Platform uses: reversed.
pub fn reversed(hash: &[u8; 32]) -> [u8; 32] {
    let mut out = *hash;
    out.reverse();
    out
}

pub fn identity(identity: &Identity) -> ffi::Identity {
    ffi::Identity {
        id: identity.id().to_buffer(),
        balance: identity.balance(),
        revision: identity.revision(),
        keys: identity
            .public_keys()
            .values()
            .map(|key| ffi::IdentityKey {
                id: key.id(),
                purpose: key.purpose() as u8,
                security_level: key.security_level() as u8,
                key_type: key.key_type() as u8,
                read_only: key.read_only(),
                data: key.data().to_vec(),
                disabled_at: key.disabled_at().unwrap_or(0),
            })
            .collect(),
    }
}

/// A key to register with a new identity; it must be enabled.
pub fn new_identity_key(key: &ffi::IdentityKey) -> Result<IdentityPublicKey, String> {
    if key.disabled_at != 0 {
        return Err(format!("key {} is disabled", key.id));
    }
    Ok(IdentityPublicKeyV0 {
        id: key.id,
        purpose: Purpose::try_from(key.purpose).map_err(|e| format!("key {}: {e}", key.id))?,
        security_level: SecurityLevel::try_from(key.security_level)
            .map_err(|e| format!("key {}: {e}", key.id))?,
        contract_bounds: None,
        key_type: KeyType::try_from(key.key_type).map_err(|e| format!("key {}: {e}", key.id))?,
        read_only: key.read_only,
        data: key.data.clone().into(),
        disabled_at: None,
    }
    .into())
}

pub fn asset_lock_proof(proof: &ffi::AssetLockProof) -> Result<AssetLockProof, String> {
    match proof.kind {
        ffi::AssetLockProofKind::Instant => {
            let transaction = Transaction::consensus_decode(&mut proof.transaction.as_slice())
                .map_err(|e| format!("bad asset lock transaction: {e}"))?;
            let instant_lock = InstantLock::consensus_decode(&mut proof.instant_lock.as_slice())
                .map_err(|e| format!("bad instant lock: {e}"))?;
            Ok(AssetLockProof::Instant(InstantAssetLockProof::new(
                instant_lock,
                transaction,
                proof.output_index,
            )))
        }
        ffi::AssetLockProofKind::Chain => Ok(AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: proof.core_chain_locked_height,
            out_point: OutPoint::consensus_decode(&mut proof.out_point.as_slice())
                .map_err(|e| format!("bad asset lock outpoint: {e}"))?,
        })),
        other => Err(format!("unknown asset lock proof kind {}", other.repr)),
    }
}

pub fn dpns_name(name: DpnsUsername) -> ffi::DpnsName {
    ffi::DpnsName {
        label: name.label,
        normalized_label: name.normalized_label,
        owner: name.owner_id.to_buffer(),
        identity: name
            .records_identity_id
            .map(|id| id.to_buffer())
            .unwrap_or_default(),
    }
}

pub fn vote_state(contenders: &Contenders) -> ffi::VoteState {
    let (winner_kind, winner) = match contenders.winner.as_ref().map(|(info, _)| info) {
        Some(ContestedDocumentVotePollWinnerInfo::WonByIdentity(id)) => {
            (ffi::WinnerKind::Identity, id.to_buffer())
        }
        Some(ContestedDocumentVotePollWinnerInfo::Locked) => (ffi::WinnerKind::Locked, [0; 32]),
        _ => (ffi::WinnerKind::None, [0; 32]),
    };
    ffi::VoteState {
        contenders: contenders
            .contenders
            .iter()
            .map(|(id, contender)| ffi::Contender {
                identity: id.to_buffer(),
                votes: contender.vote_tally().unwrap_or(0),
            })
            .collect(),
        abstain: contenders.abstain_vote_tally.unwrap_or(0),
        lock: contenders.lock_vote_tally.unwrap_or(0),
        winner_kind,
        winner,
    }
}

pub fn empty_vote_state() -> ffi::VoteState {
    ffi::VoteState {
        contenders: Vec::new(),
        abstain: 0,
        lock: 0,
        winner_kind: ffi::WinnerKind::None,
        winner: [0; 32],
    }
}

pub fn contact_request(document: &Document) -> Result<ffi::ContactRequest, String> {
    let properties: &BTreeMap<String, Value> = document.properties();
    let field = |name: &str| {
        properties
            .get(name)
            .ok_or_else(|| format!("contactRequest {} has no {name}", document.id()))
    };
    let index = |name: &str| {
        field(name)?
            .to_integer::<u64>()
            .map_err(|e| format!("contactRequest {name}: {e}"))
    };
    let optional_bytes = |name: &str| {
        properties
            .get(name)
            .map(|value| value.to_binary_bytes())
            .transpose()
            .map(Option::unwrap_or_default)
            .map_err(|e| format!("contactRequest {name}: {e}"))
    };
    Ok(ffi::ContactRequest {
        document_id: document.id().to_buffer(),
        owner: document.owner_id().to_buffer(),
        to_user_id: field("toUserId")?
            .to_identifier()
            .map_err(|e| format!("contactRequest toUserId: {e}"))?
            .to_buffer(),
        encrypted_public_key: field("encryptedPublicKey")?
            .to_binary_bytes()
            .map_err(|e| format!("contactRequest encryptedPublicKey: {e}"))?,
        sender_key_index: index("senderKeyIndex")?,
        recipient_key_index: index("recipientKeyIndex")?,
        account_reference: index("accountReference")?,
        encrypted_account_label: optional_bytes("encryptedAccountLabel")?,
        auto_accept_proof: optional_bytes("autoAcceptProof")?,
        created_at_ms: document.created_at().unwrap_or(0),
        created_at_core_height: document.created_at_core_block_height().unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::document::DocumentV0;

    #[test]
    fn a_contact_request_document_is_marshalled_field_by_field() {
        let mut properties = BTreeMap::from([
            ("toUserId".to_string(), Value::Identifier([2; 32])),
            ("encryptedPublicKey".to_string(), Value::Bytes(vec![3; 96])),
            ("senderKeyIndex".to_string(), Value::U32(4)),
            ("recipientKeyIndex".to_string(), Value::U64(5)),
            ("accountReference".to_string(), Value::U32(6)),
            ("autoAcceptProof".to_string(), Value::Bytes(vec![7; 38])),
        ]);
        let document = |properties| -> Document {
            DocumentV0 {
                id: [1; 32].into(),
                owner_id: [8; 32].into(),
                properties,
                created_at: Some(9),
                created_at_core_block_height: Some(10),
                ..Default::default()
            }
            .into()
        };
        let request = contact_request(&document(properties.clone())).expect("marshals");
        assert_eq!(
            (request.document_id, request.owner, request.to_user_id),
            ([1; 32], [8; 32], [2; 32])
        );
        assert_eq!(request.encrypted_public_key, vec![3; 96]);
        assert_eq!(
            (
                request.sender_key_index,
                request.recipient_key_index,
                request.account_reference
            ),
            (4, 5, 6)
        );
        assert!(request.encrypted_account_label.is_empty());
        assert_eq!(request.auto_accept_proof, vec![7; 38]);
        assert_eq!(
            (request.created_at_ms, request.created_at_core_height),
            (9, 10)
        );

        properties.remove("toUserId");
        assert!(contact_request(&document(properties)).is_err());
    }

    /// The schema bounds the indices below only: one above u32 is passed
    /// as stored, not narrowed or refused here.
    #[test]
    fn a_contact_request_index_above_u32_is_passed_as_stored() {
        let above = u32::MAX as u64 + 1;
        let document: Document = DocumentV0 {
            properties: BTreeMap::from([
                ("toUserId".to_string(), Value::Identifier([2; 32])),
                ("encryptedPublicKey".to_string(), Value::Bytes(vec![3; 96])),
                ("senderKeyIndex".to_string(), Value::U64(above)),
                ("recipientKeyIndex".to_string(), Value::I64(above as i64)),
                ("accountReference".to_string(), Value::U64(u64::MAX)),
            ]),
            ..Default::default()
        }
        .into();
        let request = contact_request(&document).expect("marshals");
        assert_eq!(
            (
                request.sender_key_index,
                request.recipient_key_index,
                request.account_reference
            ),
            (above, above, u64::MAX)
        );
    }
}
