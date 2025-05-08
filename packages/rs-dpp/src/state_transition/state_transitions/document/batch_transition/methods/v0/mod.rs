#[cfg(feature = "state-transition-signing")]
use crate::data_contract::document_type::DocumentTypeRef;
#[cfg(feature = "state-transition-signing")]
use crate::document::Document;
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::IdentityPublicKey;
use crate::identity::SecurityLevel;
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;
use std::convert::TryFrom;
use crate::state_transition::batch_transition::batched_transition::{BatchedTransition, BatchedTransitionRef};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
#[cfg(feature = "state-transition-signing")]
use crate::tokens::token_payment_info::TokenPaymentInfo;

pub trait DocumentsBatchTransitionMethodsV0: DocumentsBatchTransitionAccessorsV0 {
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_document_creation_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        entropy: [u8; 32],
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_document_replacement_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_document_deletion_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_document_transfer_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        recipient_owner_id: Identifier,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_document_update_price_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        price: Credits,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn new_document_purchase_transition_from_document<S: Signer>(
        document: Document,
        document_type: DocumentTypeRef,
        new_owner_id: Identifier,
        price: Credits,
        identity_public_key: &IdentityPublicKey,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, ProtocolError>;

    fn combined_security_level_requirement(
        &self,
        get_data_contract_security_level_requirement: Option<
            impl Fn(Identifier, String) -> Result<SecurityLevel, ProtocolError>,
        >,
    ) -> Result<Vec<SecurityLevel>, ProtocolError> {
        // Step 1: Get all document types for the ST
        // Step 2: Get document schema for every type
        // If schema has security level, use that, if not, use the default security level
        // Find the highest level (lowest int value) of all documents - the ST's signature
        // requirement is the highest level across all documents affected by the ST./
        let mut highest_security_level = SecurityLevel::lowest_level();

        if self
            .transitions_iter()
            .any(|transition| matches!(transition, BatchedTransitionRef::Token(_)))
        {
            // If we ever have a token transition it will be security level critical and so will the whole state transition
            highest_security_level = SecurityLevel::CRITICAL;
        } else if self
            .transitions_iter()
            .any(|transition| matches!(transition, BatchedTransitionRef::Document(_)))
        {
            // We know we don't have token transitions at this point
            let get_data_contract_security_level_requirement =
                get_data_contract_security_level_requirement.ok_or(
                    ProtocolError::CorruptedCodeExecution(
                        "must supply get_data_contract when signing a documents batch transition"
                            .to_string(),
                    ),
                )?;
            for transition in self.transitions_iter() {
                if let BatchedTransitionRef::Document(document_transition) = transition {
                    let document_type_name = document_transition.base().document_type_name();
                    let data_contract_id = document_transition.base().data_contract_id();
                    let document_security_level = get_data_contract_security_level_requirement(
                        data_contract_id,
                        document_type_name.to_owned(),
                    )?;

                    // lower enum representation means higher in security
                    if document_security_level < highest_security_level {
                        highest_security_level = document_security_level;
                    }
                }
            }
        };

        Ok(if highest_security_level == SecurityLevel::MASTER {
            vec![SecurityLevel::MASTER]
        } else {
            // this might seem wrong until you realize that master is 0, critical 1, etc
            (SecurityLevel::CRITICAL as u8..=highest_security_level as u8)
                .map(|security_level| SecurityLevel::try_from(security_level).unwrap())
                .collect()
        })
    }

    fn set_transitions(&mut self, transitions: Vec<BatchedTransition>);

    fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce);

    fn all_conflicting_index_collateral_voting_funds(
        &self,
    ) -> Result<Option<Credits>, ProtocolError>;

    fn all_document_purchases_amount(&self) -> Result<Option<Credits>, ProtocolError>;
}
