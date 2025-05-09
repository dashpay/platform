#[cfg(feature = "state-transition-signing")]
use crate::balances::credits::TokenAmount;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::group::GroupStateTransitionInfoStatus;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::IdentityPublicKey;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::methods::StateTransitionCreationOptions;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::emergency_action::TokenEmergencyAction;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::token_pricing_schedule::TokenPricingSchedule;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::{PrivateEncryptedNote, SharedEncryptedNote};
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

/// A trait defining methods to create various token-related state transitions as part of a document batch.
///
/// This trait builds on `DocumentsBatchTransitionAccessorsV0` and provides a unified interface
/// for constructing signed `StateTransition`s for token operations such as minting, burning,
/// transferring, freezing, claiming, and direct purchases.
///
/// These methods are primarily used by clients or services that need to programmatically
/// generate transitions while applying protocol-specific rules like nonce tracking,
/// version selection, group action metadata, and fee calculations.
///
/// All methods in this trait require the `state-transition-signing` feature to be enabled.
pub trait DocumentsBatchTransitionMethodsV1: DocumentsBatchTransitionAccessorsV0 {
    /// Creates a `StateTransition` to mint new tokens.
    ///
    /// # Parameters
    /// - `token_id`: ID of the token being minted.
    /// - `owner_id`: ID of the token's owner.
    /// - `issued_to_identity_id`: Optionally specifies the identity receiving the minted tokens. If this is not set the tokens will go to the identity set in the data contract. If neither is set we will get an error.
    /// - `data_contract_id`: The contract ID associated with the token.
    /// - `token_contract_position`: The token's index within the contract.
    /// - `amount`: Number of tokens to mint.
    /// - `public_note`: Optional plaintext note.
    /// - `using_group_info`: Optional group/multisig info if performing this task within a group.
    /// - `identity_public_key`: Public key used for signing.
    /// - `identity_contract_nonce`: Nonce to prevent replay.
    /// - `user_fee_increase`: Fee adjustment parameter.
    /// - `signer`: Object implementing the signer trait that much contain the private key for the identity public key.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_token_mint_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        amount: TokenAmount,
        issued_to_identity_id: Option<Identifier>,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Creates a `StateTransition` to burn tokens, permanently removing them from circulation.
    ///
    /// # Parameters
    /// - `token_id`: ID of the token being burned.
    /// - `owner_id`: ID of the token's owner.
    /// - `data_contract_id`: The contract ID associated with the token.
    /// - `token_contract_position`: The token's index within the contract.
    /// - `amount`: Number of tokens to burn.
    /// - `public_note`: Optional plaintext note.
    /// - `using_group_info`: Optional group/multisig info if performing this task within a group.
    /// - `identity_public_key`: Public key used for signing.
    /// - `identity_contract_nonce`: Nonce to prevent replay.
    /// - `user_fee_increase`: Fee adjustment parameter.
    /// - `signer`: Object implementing the signer trait that must contain the private key for the identity public key.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_token_burn_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        amount: TokenAmount,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Creates a `StateTransition` to transfer tokens from one identity to another.
    ///
    /// # Parameters
    /// - `token_id`: ID of the token being transferred.
    /// - `owner_id`: ID of the token's current owner.
    /// - `data_contract_id`: The contract ID associated with the token.
    /// - `token_contract_position`: The token's index within the contract.
    /// - `amount`: Number of tokens to transfer.
    /// - `recipient_id`: Identity ID of the recipient.
    /// - `public_note`: Optional plaintext note.
    /// - `shared_encrypted_note`: Optional encrypted note viewable by multiple parties.
    /// - `private_encrypted_note`: Optional encrypted note viewable only by the recipient.
    /// - `identity_public_key`: Public key used for signing.
    /// - `identity_contract_nonce`: Nonce to prevent replay.
    /// - `user_fee_increase`: Fee adjustment parameter.
    /// - `signer`: Object implementing the signer trait that must contain the private key for the identity public key.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_token_transfer_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        amount: TokenAmount,
        recipient_id: Identifier,
        public_note: Option<String>,
        shared_encrypted_note: Option<SharedEncryptedNote>,
        private_encrypted_note: Option<PrivateEncryptedNote>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Creates a `StateTransition` to freeze tokens belonging to a specific identity.
    ///
    /// Frozen tokens cannot be transferred or used until explicitly unfrozen or destroyed.
    ///
    /// # Parameters
    /// - `token_id`: ID of the token being frozen.
    /// - `owner_id`: ID of the token's owner.
    /// - `data_contract_id`: The contract ID associated with the token.
    /// - `token_contract_position`: The token's index within the contract.
    /// - `frozen_identity_id`: ID of the identity whose tokens are being frozen.
    /// - `public_note`: Optional plaintext note.
    /// - `using_group_info`: Optional group/multisig info if performing this task within a group.
    /// - `identity_public_key`: Public key used for signing.
    /// - `identity_contract_nonce`: Nonce to prevent replay.
    /// - `user_fee_increase`: Fee adjustment parameter.
    /// - `signer`: Object implementing the signer trait that must contain the private key for the identity public key.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_token_freeze_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        frozen_identity_id: Identifier,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Creates a `StateTransition` to unfreeze tokens previously frozen for a specific identity.
    ///
    /// # Parameters
    /// - `token_id`: ID of the token being unfrozen.
    /// - `owner_id`: ID of the token's owner.
    /// - `data_contract_id`: The contract ID associated with the token.
    /// - `token_contract_position`: The token's index within the contract.
    /// - `frozen_identity_id`: ID of the identity whose tokens are being unfrozen.
    /// - `public_note`: Optional plaintext note.
    /// - `using_group_info`: Optional group/multisig info if performing this task within a group.
    /// - `identity_public_key`: Public key used for signing.
    /// - `identity_contract_nonce`: Nonce to prevent replay.
    /// - `user_fee_increase`: Fee adjustment parameter.
    /// - `signer`: Object implementing the signer trait that must contain the private key for the identity public key.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_token_unfreeze_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        frozen_identity_id: Identifier,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Creates a `StateTransition` to destroy previously frozen tokens, removing them permanently from supply.
    ///
    /// # Parameters
    /// - `token_id`: ID of the token being destroyed.
    /// - `owner_id`: ID of the token's owner.
    /// - `data_contract_id`: The contract ID associated with the token.
    /// - `token_contract_position`: The token's index within the contract.
    /// - `frozen_identity_id`: ID of the identity whose frozen tokens are being destroyed.
    /// - `public_note`: Optional plaintext note.
    /// - `using_group_info`: Optional group/multisig info if performing this task within a group.
    /// - `identity_public_key`: Public key used for signing.
    /// - `identity_contract_nonce`: Nonce to prevent replay.
    /// - `user_fee_increase`: Fee adjustment parameter.
    /// - `signer`: Object implementing the signer trait that must contain the private key for the identity public key.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_token_destroy_frozen_funds_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        frozen_identity_id: Identifier,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Creates a `StateTransition` to execute an emergency action for a token.
    ///
    /// Emergency actions may include critical interventions such as pausing operations,
    /// revoking permissions, or executing recovery operations.
    ///
    /// # Parameters
    /// - `token_id`: ID of the token involved.
    /// - `owner_id`: ID of the token's owner.
    /// - `data_contract_id`: The contract ID associated with the token.
    /// - `token_contract_position`: The token's index within the contract.
    /// - `emergency_action`: The action to be executed.
    /// - `public_note`: Optional plaintext note.
    /// - `using_group_info`: Optional group/multisig info if performing this task within a group.
    /// - `identity_public_key`: Public key used for signing.
    /// - `identity_contract_nonce`: Nonce to prevent replay.
    /// - `user_fee_increase`: Fee adjustment parameter.
    /// - `signer`: Object implementing the signer trait that must contain the private key for the identity public key.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_token_emergency_action_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        emergency_action: TokenEmergencyAction,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Creates a `StateTransition` to update the configuration of a token.
    ///
    /// This includes changing properties such as max supply, permissions, or other
    /// configurable behavior defined in the token contract.
    ///
    /// # Parameters
    /// - `token_id`: ID of the token being updated.
    /// - `owner_id`: ID of the token's owner.
    /// - `data_contract_id`: The contract ID associated with the token.
    /// - `token_contract_position`: The token's index within the contract.
    /// - `update_token_configuration_item`: The configuration change to be applied.
    /// - `public_note`: Optional plaintext note.
    /// - `using_group_info`: Optional group/multisig info if performing this task within a group.
    /// - `identity_public_key`: Public key used for signing.
    /// - `identity_contract_nonce`: Nonce to prevent replay.
    /// - `user_fee_increase`: Fee adjustment parameter.
    /// - `signer`: Object implementing the signer trait that must contain the private key for the identity public key.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_token_config_update_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        update_token_configuration_item: TokenConfigurationChangeItem,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Creates a `StateTransition` to claim tokens from a distribution source.
    ///
    /// Used when tokens are distributed through rewards, airdrops, or other allocation
    /// mechanisms that require explicit claiming by the identity.
    ///
    /// # Parameters
    /// - `token_id`: ID of the token being claimed.
    /// - `owner_id`: ID of the token's owner.
    /// - `data_contract_id`: The contract ID associated with the token.
    /// - `token_contract_position`: The token's index within the contract.
    /// - `distribution_type`: Type of token distribution (e.g., reward pool, airdrop).
    /// - `public_note`: Optional plaintext note.
    /// - `identity_public_key`: Public key used for signing.
    /// - `identity_contract_nonce`: Nonce to prevent replay.
    /// - `user_fee_increase`: Fee adjustment parameter.
    /// - `signer`: Object implementing the signer trait that must contain the private key for the identity public key.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_token_claim_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        distribution_type: TokenDistributionType,
        public_note: Option<String>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Creates a `StateTransition` to set or update the price of a token for direct purchase.
    ///
    /// This defines a `TokenPricingSchedule` that determines how credits are exchanged
    /// for tokens. Setting the price to `None` disables the ability to purchase the token.
    ///
    /// # Parameters
    /// - `token_id`: ID of the token whose price is being updated.
    /// - `owner_id`: ID of the token's owner.
    /// - `data_contract_id`: The contract ID associated with the token.
    /// - `token_contract_position`: The token's index within the contract.
    /// - `token_pricing_schedule`: The new pricing schedule. `None` disables purchases.
    /// - `public_note`: Optional plaintext note.
    /// - `using_group_info`: Optional group/multisig info if performing this task within a group.
    /// - `identity_public_key`: Public key used for signing.
    /// - `identity_contract_nonce`: Nonce to prevent replay.
    /// - `user_fee_increase`: Fee adjustment parameter.
    /// - `signer`: Object implementing the signer trait that must contain the private key for the identity public key.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_token_change_direct_purchase_price_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        token_pricing_schedule: Option<TokenPricingSchedule>,
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfoStatus>,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Creates a `StateTransition` to perform a direct purchase of tokens by a user.
    ///
    /// The user agrees to purchase a given amount of tokens and provides a maximum
    /// total credit value they're willing to pay. If the configured price is lower than
    /// this amount, the user will be charged less.
    ///
    /// # Parameters
    /// - `token_id`: ID of the token being purchased.
    /// - `owner_id`: ID of the token's owner.
    /// - `data_contract_id`: The contract ID associated with the token.
    /// - `token_contract_position`: The token's index within the contract.
    /// - `amount`: Number of tokens to purchase.
    /// - `agreed_total_cost`: Maximum credits the user agrees to pay.
    /// - `identity_public_key`: Public key used for signing.
    /// - `identity_contract_nonce`: Nonce to prevent replay.
    /// - `user_fee_increase`: Fee adjustment parameter.
    /// - `signer`: Object implementing the signer trait that must contain the private key for the identity public key.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_token_direct_purchase_transition<S: Signer>(
        token_id: Identifier,
        owner_id: Identifier,
        data_contract_id: Identifier,
        token_contract_position: u16,
        amount: TokenAmount,
        total_agreed_price: Credits,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;
}
