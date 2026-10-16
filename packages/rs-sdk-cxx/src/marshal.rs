// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Conversions between the bridge's plain structs and dpp types.

use std::collections::BTreeMap;

use dash_sdk::dpp::dashcore::consensus::Decodable;
use dash_sdk::dpp::dashcore::hashes::Hash;
use dash_sdk::dpp::dashcore::{InstantLock, OutPoint, ProTxHash, Transaction};
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dash_sdk::dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dash_sdk::dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::{AssetLockProof, Identifier};
use dash_sdk::dpp::system_data_contracts::SystemDataContract;
use dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dash_sdk::dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dash_sdk::dpp::voting::vote_polls::VotePoll;
use dash_sdk::dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dash_sdk::dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dash_sdk::dpp::voting::votes::resource_vote::ResourceVote;
use dash_sdk::dpp::voting::votes::Vote;
use dash_sdk::platform::dashpay::ProfileFields;
use dash_sdk::platform::dpns_usernames::{convert_to_homograph_safe_chars, DpnsUsername};
use dash_sdk::query_types::Contenders;

use crate::ffi;

/// A 32-byte chain hash (quorum hash, proTxHash) between the embedder's
/// internal `uint256` byte order and the order Platform uses: reversed.
pub fn reversed(hash: &[u8; 32]) -> [u8; 32] {
    let mut out = *hash;
    out.reverse();
    out
}

/// A proTxHash in the embedder's internal order, as the typed `ProTxHash`
/// (Core's RPC hex order) the SDK's voting calls take.
pub fn pro_tx_hash(internal_order: &[u8; 32]) -> ProTxHash {
    ProTxHash::from_byte_array(reversed(internal_order))
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

/// The DPNS poll a vote on `label` goes to, as the SDK's DPNS contest
/// queries build it, with the label normalized the same way.
fn dpns_vote_poll(label: &str) -> ContestedDocumentResourceVotePoll {
    ContestedDocumentResourceVotePoll {
        contract_id: SystemDataContract::DPNS.id(),
        document_type_name: "domain".to_string(),
        index_name: "parentNameAndLabel".to_string(),
        index_values: vec![
            Value::Text("dash".to_string()),
            Value::Text(convert_to_homograph_safe_chars(label)),
        ],
    }
}

pub fn dpns_vote(label: &str, choice: ffi::VoteChoice, towards: &[u8; 32]) -> Result<Vote, String> {
    let resource_vote_choice = match choice {
        ffi::VoteChoice::TowardsIdentity => {
            ResourceVoteChoice::TowardsIdentity(Identifier::from(*towards))
        }
        ffi::VoteChoice::Abstain => ResourceVoteChoice::Abstain,
        ffi::VoteChoice::Lock => ResourceVoteChoice::Lock,
        other => return Err(format!("unknown vote choice {}", other.repr)),
    };
    Ok(Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
        vote_poll: VotePoll::ContestedDocumentResourceVotePoll(dpns_vote_poll(label)),
        resource_vote_choice,
    })))
}

/// A DPNS vote, `None` for a vote on any other poll.
pub fn name_vote(vote: &ResourceVote) -> Option<ffi::NameVote> {
    let VotePoll::ContestedDocumentResourceVotePoll(poll) = vote.vote_poll();
    if poll.contract_id != SystemDataContract::DPNS.id() || poll.document_type_name != "domain" {
        return None;
    }
    let Some(Value::Text(label)) = poll.index_values.get(1) else {
        return None;
    };
    let (choice, towards) = match vote.resource_vote_choice() {
        ResourceVoteChoice::TowardsIdentity(id) => {
            (ffi::VoteChoice::TowardsIdentity, id.to_buffer())
        }
        ResourceVoteChoice::Abstain => (ffi::VoteChoice::Abstain, [0; 32]),
        ResourceVoteChoice::Lock => (ffi::VoteChoice::Lock, [0; 32]),
    };
    Some(ffi::NameVote {
        normalized_label: label.clone(),
        choice,
        towards,
    })
}

/// A document's optional property, `T::default()` when absent.
fn optional_property<T: Default, E: std::fmt::Display>(
    document: &Document,
    kind: &str,
    name: &str,
    read: impl FnOnce(&Value) -> Result<T, E>,
) -> Result<T, String> {
    document
        .properties()
        .get(name)
        .map(read)
        .transpose()
        .map(Option::unwrap_or_default)
        .map_err(|e| format!("{kind} {name}: {e}"))
}

/// An embedder string where empty means absent.
fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

/// An optional fixed-size field: empty is `None`, any other length an error.
fn optional_array<const N: usize>(bytes: &[u8], name: &str) -> Result<Option<[u8; N]>, String> {
    if bytes.is_empty() {
        return Ok(None);
    }
    bytes
        .try_into()
        .map(Some)
        .map_err(|_| format!("{name} must be {N} bytes, not {}", bytes.len()))
}

pub fn profile_fields(fields: &ffi::ProfileFields) -> Result<ProfileFields, String> {
    Ok(ProfileFields {
        display_name: non_empty(&fields.display_name),
        public_message: non_empty(&fields.public_message),
        avatar_url: non_empty(&fields.avatar_url),
        avatar_hash: optional_array(&fields.avatar_hash, "avatar_hash")?,
        avatar_fingerprint: optional_array(&fields.avatar_fingerprint, "avatar_fingerprint")?,
    })
}

pub fn profile(document: &Document) -> Result<ffi::Profile, String> {
    let text = |name| optional_property(document, "profile", name, Value::to_text);
    let bytes = |name| optional_property(document, "profile", name, Value::to_binary_bytes);
    Ok(ffi::Profile {
        document_id: document.id().to_buffer(),
        owner: document.owner_id().to_buffer(),
        revision: document.revision().unwrap_or(0),
        display_name: text("displayName")?,
        public_message: text("publicMessage")?,
        avatar_url: text("avatarUrl")?,
        avatar_hash: bytes("avatarHash")?,
        avatar_fingerprint: bytes("avatarFingerprint")?,
        updated_at_ms: document.updated_at().unwrap_or(0),
    })
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
            .to_integer::<u32>()
            .map_err(|e| format!("contactRequest {name}: {e}"))
    };
    let optional_bytes =
        |name| optional_property(document, "contactRequest", name, Value::to_binary_bytes);
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

    #[test]
    fn a_profile_document_is_marshalled_and_its_fields_convert() {
        let document: Document = DocumentV0 {
            id: [1; 32].into(),
            owner_id: [2; 32].into(),
            properties: BTreeMap::from([
                ("displayName".to_string(), Value::Text("Alice".into())),
                ("avatarHash".to_string(), Value::Bytes32([3; 32])),
                ("avatarFingerprint".to_string(), Value::Bytes(vec![4; 8])),
            ]),
            revision: Some(5),
            updated_at: Some(6),
            ..Default::default()
        }
        .into();
        let profile = profile(&document).expect("marshals");
        assert_eq!((profile.document_id, profile.owner), ([1; 32], [2; 32]));
        assert_eq!((profile.revision, profile.updated_at_ms), (5, 6));
        assert_eq!(profile.display_name, "Alice");
        assert!(profile.public_message.is_empty() && profile.avatar_url.is_empty());
        assert_eq!(profile.avatar_hash, vec![3; 32]);
        assert_eq!(profile.avatar_fingerprint, vec![4; 8]);

        let mut fields = ffi::ProfileFields {
            display_name: "Alice".into(),
            public_message: String::new(),
            avatar_url: "https://a.example/a.png".into(),
            avatar_hash: vec![3; 32],
            avatar_fingerprint: Vec::new(),
        };
        let converted = profile_fields(&fields).expect("converts");
        assert_eq!(converted.display_name.as_deref(), Some("Alice"));
        assert_eq!(converted.public_message, None);
        assert_eq!(converted.avatar_hash, Some([3; 32]));
        assert_eq!(converted.avatar_fingerprint, None);
        fields.avatar_fingerprint = vec![4; 7];
        assert!(profile_fields(&fields).is_err());
    }

    /// Core hands over a proTxHash in its internal `uint256` order; the
    /// SDK's `ProTxHash` reads as Core's RPC hex.
    #[test]
    fn a_core_pro_tx_hash_becomes_the_rpc_order_pro_tx_hash() {
        use std::str::FromStr;

        let rpc_hex = "74e26f433328be4b833b8958c04c51615e03853378e0d56fbe5ecf24977f884b";
        let mut core_internal: [u8; 32] =
            std::array::from_fn(|i| u8::from_str_radix(&rpc_hex[2 * i..2 * i + 2], 16).unwrap());
        core_internal.reverse();
        assert_eq!(
            pro_tx_hash(&core_internal),
            ProTxHash::from_str(rpc_hex).unwrap()
        );
    }

    /// A vote built for a label reads back as the same DPNS name vote; a
    /// vote on another contract's poll is not a name vote.
    #[test]
    fn a_dpns_vote_round_trips_and_other_polls_are_skipped() {
        let Vote::ResourceVote(vote) =
            dpns_vote("Alice", ffi::VoteChoice::TowardsIdentity, &[4; 32]).expect("builds");
        let marshalled = name_vote(&vote).expect("a DPNS vote");
        assert_eq!(marshalled.normalized_label, "a11ce");
        assert_eq!(marshalled.choice, ffi::VoteChoice::TowardsIdentity);
        assert_eq!(marshalled.towards, [4; 32]);

        // The poll names DPNS's contested index, its values in the index's
        // property order.
        use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dash_sdk::dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use dash_sdk::dpp::system_data_contracts::load_system_data_contract;
        use dash_sdk::dpp::version::PlatformVersion;
        let mut poll = dpns_vote_poll("alice");
        let dpns =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest()).unwrap();
        let domain = dpns
            .document_type_for_name(&poll.document_type_name)
            .unwrap();
        let index = &domain.indexes()[&poll.index_name];
        assert!(index.contested_index.is_some());
        assert_eq!(
            index
                .properties
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["normalizedParentDomainName", "normalizedLabel"]
        );

        poll.contract_id = [9; 32].into();
        let other = ResourceVote::V0(ResourceVoteV0 {
            vote_poll: VotePoll::ContestedDocumentResourceVotePoll(poll),
            resource_vote_choice: ResourceVoteChoice::Lock,
        });
        assert!(name_vote(&other).is_none());
        assert!(dpns_vote("alice", ffi::VoteChoice { repr: 9 }, &[0; 32]).is_err());
    }
}
