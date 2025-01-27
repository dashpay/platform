use crate::platform::transition::builder::StateTransitionBuilder;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use crate::{Error, Sdk};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use dpp::state_transition::batch_transition::token_transfer_transition::{
    PrivateEncryptedNote, SharedEncryptedNote,
};
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::tokens::calculate_token_id;
use dpp::version::PlatformVersion;

/// A builder to configure minting tokens.
pub struct TokenTransferTransitionBuilder<'a> {
    data_contract: &'a DataContract,
    token_position: TokenContractPosition,
    issuer_id: Identifier,
    amount: TokenAmount,
    recipient_id: Identifier,
    public_note: Option<String>,
    shared_encrypted_note: Option<SharedEncryptedNote>,
    private_encrypted_note: Option<PrivateEncryptedNote>,
    settings: Option<PutSettings>,
    user_fee_increase: Option<UserFeeIncrease>,
}

impl<'a> TokenTransferTransitionBuilder<'a> {
    /// Start building a mint tokens request for the provided DataContract.
    pub fn new(
        data_contract: &'a DataContract,
        token_position: TokenContractPosition,
        sender_id: Identifier,
        recipient_id: Identifier,
        amount: TokenAmount,
    ) -> Self {
        // TODO: Validate token position

        Self {
            data_contract,
            token_position,
            issuer_id: sender_id,
            amount,
            recipient_id,
            public_note: None,
            settings: None,
            user_fee_increase: None,
            private_encrypted_note: None,
            shared_encrypted_note: None,
        }
    }

    pub fn with_shared_encrypted_note(
        mut self,
        shared_encrypted_note: SharedEncryptedNote,
    ) -> Self {
        self.shared_encrypted_note = Some(shared_encrypted_note);
        self
    }

    pub fn with_public_note(mut self, note: String) -> Self {
        self.public_note = Some(note);
        self
    }

    pub fn with_private_encrypted_note(
        mut self,
        private_encrypted_note: PrivateEncryptedNote,
    ) -> Self {
        self.private_encrypted_note = Some(private_encrypted_note);

        self
    }

    pub fn with_user_fee_increase(mut self, user_fee_increase: UserFeeIncrease) -> Self {
        self.user_fee_increase = Some(user_fee_increase);
        self
    }

    pub fn with_settings(mut self, settings: PutSettings) -> Self {
        self.settings = Some(settings);
        self
    }
}

impl StateTransitionBuilder for TokenTransferTransitionBuilder<'_> {
    fn settings(&self) -> Option<PutSettings> {
        self.settings
    }

    async fn sign(
        &self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, Error> {
        let token_id = Identifier::from(calculate_token_id(
            self.data_contract.id().as_bytes(),
            self.token_position,
        ));

        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(
                self.issuer_id,
                self.data_contract.id(),
                false,
                self.settings,
            )
            .await?;

        let state_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            self.issuer_id,
            self.data_contract.id(),
            self.token_position,
            self.amount,
            self.recipient_id,
            self.public_note.clone(),
            self.shared_encrypted_note.clone(),
            self.private_encrypted_note.clone(),
            identity_public_key,
            identity_contract_nonce,
            self.user_fee_increase.unwrap_or_default(),
            signer,
            platform_version,
            None,
            None,
            None,
        )?;

        Ok(state_transition)
    }
}
