use crate::balances::credits::TokenAmount;
use crate::prelude::{
    DerivationEncryptionKeyIndex, RecipientKeyIndex, RootEncryptionKeyIndex, SenderKeyIndex,
};
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;

pub type TokenEventPublicNote = Option<String>;
pub type TokenEventSharedEncryptedNote = Option<(SenderKeyIndex, RecipientKeyIndex, Vec<u8>)>;
pub type TokenEventPersonalEncryptedNote = Option<(
    RootEncryptionKeyIndex,
    DerivationEncryptionKeyIndex,
    Vec<u8>,
)>;
use crate::ProtocolError;

pub type RecipientIdentifier = Identifier;

#[derive(
    Debug, PartialEq, PartialOrd, Clone, Eq, Encode, Decode, PlatformDeserialize, PlatformSerialize,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
pub enum TokenEvent {
    Mint(TokenAmount, RecipientIdentifier, TokenEventPublicNote),
    Burn(TokenAmount, TokenEventPublicNote),
    Transfer(
        RecipientIdentifier,
        TokenEventPublicNote,
        TokenEventSharedEncryptedNote,
        TokenEventPersonalEncryptedNote,
        TokenAmount,
    ),
}
