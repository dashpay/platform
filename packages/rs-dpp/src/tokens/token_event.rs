use crate::balances::credits::TokenAmount;
use crate::prelude::{
    DerivationEncryptionKeyIndex, RecipientKeyIndex, RootEncryptionKeyIndex, SenderKeyIndex,
};
use platform_value::Identifier;

pub type TokenEventPublicNote = Option<String>;
pub type TokenEventSharedEncryptedNote = Option<(SenderKeyIndex, RecipientKeyIndex, Vec<u8>)>;
pub type TokenEventPersonalEncryptedNote = Option<(
    RootEncryptionKeyIndex,
    DerivationEncryptionKeyIndex,
    Vec<u8>,
)>;

#[derive(Debug, PartialEq, PartialOrd, Clone, Eq)]
pub enum TokenEvent {
    Mint(TokenAmount, TokenEventPublicNote),
    Burn(TokenAmount, TokenEventPublicNote),
    Transfer(
        Identifier,
        TokenEventPublicNote,
        TokenEventSharedEncryptedNote,
        TokenEventPersonalEncryptedNote,
        TokenAmount,
    ),
}
