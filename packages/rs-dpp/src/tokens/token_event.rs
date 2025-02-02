use crate::balances::credits::TokenAmount;
use crate::block::block_info::BlockInfo;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0};
use crate::prelude::{
    DataContract, DerivationEncryptionKeyIndex, IdentityNonce, RootEncryptionKeyIndex,
};
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

pub type TokenEventPublicNote = Option<String>;
pub type TokenEventSharedEncryptedNote = Option<SharedEncryptedNote>;
pub type TokenEventPersonalEncryptedNote = Option<(
    RootEncryptionKeyIndex,
    DerivationEncryptionKeyIndex,
    Vec<u8>,
)>;
use crate::serialization::PlatformSerializableWithPlatformVersion;
use crate::state_transition::batch_transition::token_transfer_transition::SharedEncryptedNote;
use crate::tokens::emergency_action::TokenEmergencyAction;
use crate::ProtocolError;

pub type RecipientIdentifier = Identifier;
pub type FrozenIdentifier = Identifier;

#[derive(
    Debug, PartialEq, PartialOrd, Clone, Eq, Encode, Decode, PlatformDeserialize, PlatformSerialize,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
pub enum TokenEvent {
    Mint(TokenAmount, RecipientIdentifier, TokenEventPublicNote),
    Burn(TokenAmount, TokenEventPublicNote),
    Freeze(FrozenIdentifier, TokenEventPublicNote),
    Unfreeze(FrozenIdentifier, TokenEventPublicNote),
    DestroyFrozenFunds(FrozenIdentifier, TokenAmount, TokenEventPublicNote),
    Transfer(
        RecipientIdentifier,
        TokenEventPublicNote,
        TokenEventSharedEncryptedNote,
        TokenEventPersonalEncryptedNote,
        TokenAmount,
    ),
    EmergencyAction(TokenEmergencyAction, TokenEventPublicNote),
    ConfigUpdate(TokenConfigurationChangeItem, TokenEventPublicNote),
}

impl TokenEvent {
    pub fn associated_document_type_name(&self) -> &str {
        match self {
            TokenEvent::Mint(_, _, _) => "mint",
            TokenEvent::Burn(_, _) => "burn",
            TokenEvent::Freeze(_, _) => "freeze",
            TokenEvent::Unfreeze(_, _) => "unfreeze",
            TokenEvent::DestroyFrozenFunds(_, _, _) => "destroyFrozenFunds",
            TokenEvent::Transfer(_, _, _, _, _) => "transfer",
            TokenEvent::EmergencyAction(_, _) => "emergencyAction",
            TokenEvent::ConfigUpdate(_, _) => "configUpdate",
        }
    }

    pub fn associated_document_type<'a>(
        &self,
        token_history_contract: &'a DataContract,
    ) -> Result<DocumentTypeRef<'a>, ProtocolError> {
        Ok(token_history_contract.document_type_for_name(self.associated_document_type_name())?)
    }

    pub fn build_historical_document_owned(
        self,
        token_history_contract: &DataContract,
        token_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        let document_id = Document::generate_document_id_v0(
            &token_history_contract.id(),
            &owner_id,
            self.associated_document_type_name(),
            owner_nonce.to_be_bytes().as_slice(),
        );

        let properties = match self {
            TokenEvent::Mint(mint_amount, recipient_id, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("recipientId".to_string(), recipient_id.into()),
                    ("amount".to_string(), mint_amount.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::Burn(burn_amount, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("amount".to_string(), burn_amount.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::Transfer(
                to,
                public_note,
                token_event_shared_encrypted_note,
                token_event_personal_encrypted_note,
                amount,
            ) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("amount".to_string(), amount.into()),
                    ("toIdentityId".to_string(), to.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("publicNote".to_string(), note.into());
                }
                if let Some((sender_key_index, recipient_key_index, note)) =
                    token_event_shared_encrypted_note
                {
                    properties.insert("encryptedSharedNote".to_string(), note.into());
                    properties.insert("senderKeyIndex".to_string(), sender_key_index.into());
                    properties.insert("recipientKeyIndex".to_string(), recipient_key_index.into());
                }

                if let Some((root_encryption_key_index, derivation_encryption_key_index, note)) =
                    token_event_personal_encrypted_note
                {
                    properties.insert("encryptedPersonalNote".to_string(), note.into());
                    properties.insert(
                        "rootEncryptionKeyIndex".to_string(),
                        root_encryption_key_index.into(),
                    );
                    properties.insert(
                        "derivationEncryptionKeyIndex".to_string(),
                        derivation_encryption_key_index.into(),
                    );
                }
                properties
            }
            TokenEvent::Freeze(frozen_identity_id, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("frozenIdentityId".to_string(), frozen_identity_id.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::Unfreeze(frozen_identity_id, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("frozenIdentityId".to_string(), frozen_identity_id.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::DestroyFrozenFunds(frozen_identity_id, amount, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("frozenIdentityId".to_string(), frozen_identity_id.into()),
                    ("destroyedAmount".to_string(), amount.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::EmergencyAction(action, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("action".to_string(), (action as u8).into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::ConfigUpdate(configuration_change_item, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    (
                        "changeItemType".to_string(),
                        configuration_change_item.u8_item_index().into(),
                    ),
                    (
                        "changeItem".to_string(),
                        configuration_change_item
                            .serialize_consume_to_bytes_with_platform_version(platform_version)?
                            .into(),
                    ),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
        };

        let document: Document = DocumentV0 {
            id: document_id,
            owner_id,
            properties,
            revision: None,
            created_at: Some(block_info.time_ms),
            updated_at: None,
            transferred_at: None,
            created_at_block_height: Some(block_info.height),
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
        }
        .into();

        Ok(document)
    }
}
