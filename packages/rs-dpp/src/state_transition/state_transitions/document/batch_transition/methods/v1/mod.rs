#[cfg(feature = "state-transition-signing")]
use crate::balances::credits::TokenAmount;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
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
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::emergency_action::TokenEmergencyAction;
use crate::tokens::{PrivateEncryptedNote, SharedEncryptedNote};
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait DocumentsBatchTransitionMethodsV1: DocumentsBatchTransitionAccessorsV0 {
    #[cfg(feature = "state-transition-signing")]
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
        batch_feature_version: Option<FeatureVersion>,
        delete_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    #[cfg(feature = "state-transition-signing")]
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
        batch_feature_version: Option<FeatureVersion>,
        delete_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    #[cfg(feature = "state-transition-signing")]
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
        batch_feature_version: Option<FeatureVersion>,
        delete_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    #[cfg(feature = "state-transition-signing")]
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
        batch_feature_version: Option<FeatureVersion>,
        delete_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    #[cfg(feature = "state-transition-signing")]
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
        batch_feature_version: Option<FeatureVersion>,
        delete_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    #[cfg(feature = "state-transition-signing")]
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
        batch_feature_version: Option<FeatureVersion>,
        delete_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    #[cfg(feature = "state-transition-signing")]
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
        batch_feature_version: Option<FeatureVersion>,
        delete_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    #[cfg(feature = "state-transition-signing")]
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
        batch_feature_version: Option<FeatureVersion>,
        config_update_feature_version: Option<FeatureVersion>,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;
}
