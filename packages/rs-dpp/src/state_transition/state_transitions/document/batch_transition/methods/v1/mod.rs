use crate::balances::credits::TokenAmount;
use crate::group::GroupStateTransitionInfoStatus;
use crate::identity::signer::Signer;
use crate::identity::IdentityPublicKey;
use crate::prelude::{
    DerivationEncryptionKeyIndex, IdentityNonce, RecipientKeyIndex, RootEncryptionKeyIndex,
    SenderKeyIndex, UserFeeIncrease,
};
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::ProtocolError;
use platform_value::Identifier;
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
        shared_encrypted_note: Option<(SenderKeyIndex, RecipientKeyIndex, Vec<u8>)>,
        private_encrypted_note: Option<(
            RootEncryptionKeyIndex,
            DerivationEncryptionKeyIndex,
            Vec<u8>,
        )>,
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
}
