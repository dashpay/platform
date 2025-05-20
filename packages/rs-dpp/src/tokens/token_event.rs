use crate::balances::credits::TokenAmount;
use crate::block::block_info::BlockInfo;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::data_contract::associated_token::token_distribution_key::TokenDistributionTypeWithResolvedRecipient;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionResolvedRecipient;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::DataContract;
use crate::document::{Document, DocumentV0};
use crate::fee::Credits;
use crate::prelude::{DerivationEncryptionKeyIndex, IdentityNonce, RootEncryptionKeyIndex};
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::fmt;

pub type TokenEventPublicNote = Option<String>;
pub type TokenEventSharedEncryptedNote = Option<SharedEncryptedNote>;
pub type TokenEventPersonalEncryptedNote = Option<(
    RootEncryptionKeyIndex,
    DerivationEncryptionKeyIndex,
    Vec<u8>,
)>;
use crate::serialization::PlatformSerializableWithPlatformVersion;
use crate::tokens::emergency_action::TokenEmergencyAction;
use crate::tokens::token_pricing_schedule::TokenPricingSchedule;
use crate::tokens::SharedEncryptedNote;
use crate::ProtocolError;

/// Alias representing the identity that will receive tokens or other effects from a token operation.
pub type RecipientIdentifier = Identifier;

/// Alias representing the identity that will have tokens burned from their account.
pub type BurnFromIdentifier = Identifier;

/// Alias representing the identity performing a token purchase.
pub type PurchaserIdentifier = Identifier;

/// Alias representing the identity whose tokens are subject to freezing or unfreezing.
pub type FrozenIdentifier = Identifier;

/// Represents a recorded token-related operation for use in historical documents and group actions.
///
/// `TokenEvent` is designed to encapsulate a single logical token operation,
/// such as minting, burning, transferring, or freezing tokens. These events are typically:
///
/// - **Persisted as historical records** of state transitions, enabling auditability and tracking.
/// - **Used in group (multisig) actions**, where multiple identities collaborate to authorize complex transitions.
///
/// This enum includes rich metadata for each type of operation, such as optional notes (plaintext or encrypted),
/// involved identities, and amounts. It is **externally versioned** and marked as `unversioned` in platform serialization,
/// meaning each variant is self-contained without requiring version dispatching logic.
#[derive(
    Debug, PartialEq, PartialOrd, Clone, Eq, Encode, Decode, PlatformDeserialize, PlatformSerialize,
)]
#[platform_serialize(unversioned)]
pub enum TokenEvent {
    /// Event representing the minting of tokens to a recipient.
    ///
    /// - `TokenAmount`: The amount of tokens minted.
    /// - `RecipientIdentifier`: The identity receiving the minted tokens.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    Mint(TokenAmount, RecipientIdentifier, TokenEventPublicNote),

    /// Event representing the burning of tokens, removing them from circulation.
    ///
    /// - `TokenAmount`: The amount of tokens burned.
    /// - `BurnFromIdentifier`: The account to burn from.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    Burn(TokenAmount, BurnFromIdentifier, TokenEventPublicNote),

    /// Event representing freezing of tokens for a specific identity.
    ///
    /// - `FrozenIdentifier`: The identity whose tokens are frozen.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    Freeze(FrozenIdentifier, TokenEventPublicNote),

    /// Event representing unfreezing of tokens for a specific identity.
    ///
    /// - `FrozenIdentifier`: The identity whose tokens are unfrozen.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    Unfreeze(FrozenIdentifier, TokenEventPublicNote),

    /// Event representing destruction of tokens that were previously frozen.
    ///
    /// - `FrozenIdentifier`: The identity whose frozen tokens are destroyed.
    /// - `TokenAmount`: The amount of frozen tokens destroyed.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    DestroyFrozenFunds(FrozenIdentifier, TokenAmount, TokenEventPublicNote),

    /// Event representing a transfer of tokens from one identity to another.
    ///
    /// - `RecipientIdentifier`: The recipient of the tokens.
    /// - `TokenEventPublicNote`: Optional plaintext note.
    /// - `TokenEventSharedEncryptedNote`: Optional shared encrypted metadata (multi-party).
    /// - `TokenEventPersonalEncryptedNote`: Optional private encrypted metadata (recipient-only).
    /// - `TokenAmount`: The amount of tokens transferred.
    Transfer(
        RecipientIdentifier,
        TokenEventPublicNote,
        TokenEventSharedEncryptedNote,
        TokenEventPersonalEncryptedNote,
        TokenAmount,
    ),

    /// Event representing a claim of tokens from a distribution pool or source.
    ///
    /// - `TokenDistributionTypeWithResolvedRecipient`: Type and resolved recipient of the claim.
    /// - `TokenAmount`: The amount of tokens claimed.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    Claim(
        TokenDistributionTypeWithResolvedRecipient,
        TokenAmount,
        TokenEventPublicNote,
    ),

    /// Event representing an emergency action taken on a token or identity.
    ///
    /// - `TokenEmergencyAction`: The type of emergency action performed.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    EmergencyAction(TokenEmergencyAction, TokenEventPublicNote),

    /// Event representing an update to the configuration of a token.
    ///
    /// - `TokenConfigurationChangeItem`: The configuration change that was applied.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    ConfigUpdate(TokenConfigurationChangeItem, TokenEventPublicNote),

    /// Event representing a change in the direct purchase price of a token.
    ///
    /// - `Option<TokenPricingSchedule>`: The new pricing schedule. `None` disables direct purchase.
    /// - `TokenEventPublicNote`: Optional note associated with the event.
    ChangePriceForDirectPurchase(Option<TokenPricingSchedule>, TokenEventPublicNote),

    /// Event representing the direct purchase of tokens by a user.
    ///
    /// - `TokenAmount`: The amount of tokens purchased.
    /// - `Credits`: The number of credits paid.
    DirectPurchase(TokenAmount, Credits),
}

impl fmt::Display for TokenEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenEvent::Mint(amount, recipient, note) => {
                write!(f, "Mint {} to {}{}", amount, recipient, format_note(note))
            }
            TokenEvent::Burn(amount, burn_from_identifier, note) => {
                write!(
                    f,
                    "Burn {} from {}{}",
                    amount,
                    burn_from_identifier,
                    format_note(note)
                )
            }
            TokenEvent::Freeze(identity, note) => {
                write!(f, "Freeze {}{}", identity, format_note(note))
            }
            TokenEvent::Unfreeze(identity, note) => {
                write!(f, "Unfreeze {}{}", identity, format_note(note))
            }
            TokenEvent::DestroyFrozenFunds(identity, amount, note) => {
                write!(
                    f,
                    "Destroy {} frozen from {}{}",
                    amount,
                    identity,
                    format_note(note)
                )
            }
            TokenEvent::Transfer(to, note, _, _, amount) => {
                write!(f, "Transfer {} to {}{}", amount, to, format_note(note))
            }
            TokenEvent::Claim(recipient, amount, note) => {
                write!(
                    f,
                    "Claim {} by {:?}{}",
                    amount,
                    recipient,
                    format_note(note)
                )
            }
            TokenEvent::EmergencyAction(action, note) => {
                write!(f, "Emergency action {:?}{}", action, format_note(note))
            }
            TokenEvent::ConfigUpdate(change, note) => {
                write!(f, "Configuration update {:?}{}", change, format_note(note))
            }
            TokenEvent::ChangePriceForDirectPurchase(schedule, note) => match schedule {
                Some(s) => write!(f, "Change price schedule to {:?}{}", s, format_note(note)),
                None => write!(f, "Disable direct purchase{}", format_note(note)),
            },
            TokenEvent::DirectPurchase(amount, credits) => {
                write!(f, "Direct purchase of {} for {} credits", amount, credits)
            }
        }
    }
}

fn format_note(note: &Option<String>) -> String {
    match note {
        Some(n) => format!(" (note: {})", n),
        None => String::new(),
    }
}

impl TokenEvent {
    pub fn associated_document_type_name(&self) -> &str {
        match self {
            TokenEvent::Mint(..) => "mint",
            TokenEvent::Burn(..) => "burn",
            TokenEvent::Freeze(..) => "freeze",
            TokenEvent::Unfreeze(..) => "unfreeze",
            TokenEvent::DestroyFrozenFunds(..) => "destroyFrozenFunds",
            TokenEvent::Transfer(..) => "transfer",
            TokenEvent::Claim(..) => "claim",
            TokenEvent::EmergencyAction(..) => "emergencyAction",
            TokenEvent::ConfigUpdate(..) => "configUpdate",
            TokenEvent::DirectPurchase(..) => "directPurchase",
            TokenEvent::ChangePriceForDirectPurchase(..) => "directPricing",
        }
    }

    /// Returns a reference to the public note if the variant includes one.
    pub fn public_note(&self) -> Option<&str> {
        match self {
            TokenEvent::Mint(_, _, Some(note))
            | TokenEvent::Burn(_, _, Some(note))
            | TokenEvent::Freeze(_, Some(note))
            | TokenEvent::Unfreeze(_, Some(note))
            | TokenEvent::DestroyFrozenFunds(_, _, Some(note))
            | TokenEvent::Transfer(_, Some(note), _, _, _)
            | TokenEvent::Claim(_, _, Some(note))
            | TokenEvent::EmergencyAction(_, Some(note))
            | TokenEvent::ConfigUpdate(_, Some(note))
            | TokenEvent::ChangePriceForDirectPurchase(_, Some(note)) => Some(note),
            _ => None,
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
        token_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        let document_id = Document::generate_document_id_v0(
            &token_id,
            &owner_id,
            format!("history_{}", self.associated_document_type_name()).as_str(),
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
            TokenEvent::Burn(burn_amount, burn_from_identifier, public_note) => {
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("burnFromId".to_string(), burn_from_identifier.into()),
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
            TokenEvent::Claim(recipient, amount, public_note) => {
                let (recipient_type, recipient_id, distribution_type) = match recipient {
                    TokenDistributionTypeWithResolvedRecipient::PreProgrammed(identifier) => {
                        (1u8, identifier, 0u8)
                    }
                    TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::ContractOwnerIdentity(identifier),
                    ) => (0, identifier, 1),
                    TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::Identity(identifier),
                    ) => (1, identifier, 1),
                    TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::Evonode(identifier),
                    ) => (2, identifier, 1),
                };

                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("recipientType".to_string(), recipient_type.into()),
                    ("recipientId".to_string(), recipient_id.into()),
                    ("distributionType".to_string(), distribution_type.into()),
                    ("amount".to_string(), amount.into()),
                ]);

                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                properties
            }
            TokenEvent::ChangePriceForDirectPurchase(price, note) => {
                let mut properties = BTreeMap::from([("tokenId".to_string(), token_id.into())]);

                if let Some(price_schedule) = price {
                    properties.insert(
                        "priceSchedule".to_string(),
                        price_schedule
                            .serialize_consume_to_bytes_with_platform_version(platform_version)?
                            .into(),
                    );
                }

                if let Some(note) = note {
                    properties.insert("note".to_string(), note.into());
                }

                properties
            }
            TokenEvent::DirectPurchase(amount, total_cost) => BTreeMap::from([
                ("tokenId".to_string(), token_id.into()),
                ("tokenAmount".to_string(), amount.into()),
                ("purchaseCost".to_string(), total_cost.into()),
            ]),
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
