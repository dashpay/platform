use crate::data_contract::document::DocumentWasm;
use crate::state_transitions::batch::action_fee_agreement::DocumentActionFeeAgreementWasm;
use crate::state_transitions::batch::prefunded_voting_balance::PrefundedVotingBalanceWasm;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use dpp::data_contract::document_type::action_fees::agreement::DocumentActionFeeAgreement;
use dpp::document::DocumentV0Getters;
use dpp::fee::Credits;
use dpp::prelude::{Identifier, IdentityNonce};
use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::DocumentUpdatePriceTransitionV0;
use dpp::state_transition::batch_transition::batched_transition::{
    DocumentPurchaseTransition, DocumentTransferTransition, DocumentUpdatePriceTransition,
};
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
use dpp::state_transition::batch_transition::document_base_transition::v2::DocumentBaseTransitionV2;
use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransitionV0;
use dpp::state_transition::batch_transition::document_replace_transition::DocumentReplaceTransitionV0;
use dpp::state_transition::batch_transition::{
    DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition,
};
use dpp::tokens::token_payment_info::TokenPaymentInfo;

/// The base of a document transition built here, where no platform version is at hand: version
/// 2 when it carries an action fee agreement, which protocol version 14 introduces and is the
/// only one to ask for, and version 1 otherwise, which every protocol version accepts. A
/// version 2 base is refused while an earlier protocol version is active.
pub fn document_base_transition(
    id: Identifier,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    data_contract_id: Identifier,
    token_payment_info: Option<TokenPaymentInfo>,
    action_fee_agreement: Option<DocumentActionFeeAgreement>,
) -> DocumentBaseTransition {
    match action_fee_agreement {
        Some(action_fee_agreement) => DocumentBaseTransition::V2(DocumentBaseTransitionV2 {
            id,
            identity_contract_nonce,
            document_type_name,
            data_contract_id,
            token_payment_info,
            action_fee_agreement: Some(action_fee_agreement),
        }),
        None => DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id,
            identity_contract_nonce,
            document_type_name,
            data_contract_id,
            token_payment_info,
        }),
    }
}

pub fn generate_create_transition(
    document: &DocumentWasm,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    prefunded_voting_balance: Option<PrefundedVotingBalanceWasm>,
    token_payment_info: Option<TokenPaymentInfoWasm>,
    action_fee_agreement: Option<DocumentActionFeeAgreementWasm>,
) -> DocumentCreateTransition {
    DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
        base: document_base_transition(
            document.document.id(),
            identity_contract_nonce,
            document_type_name,
            document.data_contract_id.into(),
            token_payment_info.map(TokenPaymentInfo::from),
            action_fee_agreement.map(DocumentActionFeeAgreement::from),
        ),
        entropy: document.entropy.unwrap(),
        data: document.document.properties().clone(),
        prefunded_voting_balance: prefunded_voting_balance.map(|pb| pb.into()),
    })
}

pub fn generate_delete_transition(
    document: &DocumentWasm,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    token_payment_info: Option<TokenPaymentInfoWasm>,
    action_fee_agreement: Option<DocumentActionFeeAgreementWasm>,
) -> DocumentDeleteTransition {
    DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 {
        base: document_base_transition(
            document.document.id(),
            identity_contract_nonce,
            document_type_name,
            document.data_contract_id.into(),
            token_payment_info.map(TokenPaymentInfo::from),
            action_fee_agreement.map(DocumentActionFeeAgreement::from),
        ),
    })
}

pub fn generate_replace_transition(
    document: &DocumentWasm,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    token_payment_info: Option<TokenPaymentInfoWasm>,
    action_fee_agreement: Option<DocumentActionFeeAgreementWasm>,
) -> DocumentReplaceTransition {
    DocumentReplaceTransition::V0(DocumentReplaceTransitionV0 {
        base: document_base_transition(
            document.document.id(),
            identity_contract_nonce,
            document_type_name,
            document.data_contract_id.into(),
            token_payment_info.map(TokenPaymentInfo::from),
            action_fee_agreement.map(DocumentActionFeeAgreement::from),
        ),
        revision: document.document.revision().unwrap() + 1,
        data: document.document.properties().clone(),
    })
}

pub fn generate_transfer_transition(
    document: &DocumentWasm,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    recipient_owner_id: Identifier,
    token_payment_info: Option<TokenPaymentInfoWasm>,
    action_fee_agreement: Option<DocumentActionFeeAgreementWasm>,
) -> DocumentTransferTransition {
    DocumentTransferTransition::V0(DocumentTransferTransitionV0 {
        base: document_base_transition(
            document.document.id(),
            identity_contract_nonce,
            document_type_name,
            document.data_contract_id.into(),
            token_payment_info.map(TokenPaymentInfo::from),
            action_fee_agreement.map(DocumentActionFeeAgreement::from),
        ),
        revision: document.document.revision().unwrap() + 1,
        recipient_owner_id,
    })
}

pub fn generate_update_price_transition(
    document: &DocumentWasm,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    price: Credits,
    token_payment_info: Option<TokenPaymentInfoWasm>,
    action_fee_agreement: Option<DocumentActionFeeAgreementWasm>,
) -> DocumentUpdatePriceTransition {
    DocumentUpdatePriceTransition::V0(DocumentUpdatePriceTransitionV0 {
        base: document_base_transition(
            document.document.id(),
            identity_contract_nonce,
            document_type_name,
            document.data_contract_id.into(),
            token_payment_info.map(TokenPaymentInfo::from),
            action_fee_agreement.map(DocumentActionFeeAgreement::from),
        ),
        revision: document.document.revision().unwrap() + 1,
        price,
    })
}

pub fn generate_purchase_transition(
    document: &DocumentWasm,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    price: Credits,
    token_payment_info: Option<TokenPaymentInfoWasm>,
    action_fee_agreement: Option<DocumentActionFeeAgreementWasm>,
) -> DocumentPurchaseTransition {
    DocumentPurchaseTransition::V0(DocumentPurchaseTransitionV0 {
        base: document_base_transition(
            document.document.id(),
            identity_contract_nonce,
            document_type_name,
            document.data_contract_id.into(),
            token_payment_info.map(TokenPaymentInfo::from),
            action_fee_agreement.map(DocumentActionFeeAgreement::from),
        ),
        revision: document.document.revision().unwrap() + 1,
        price,
    })
}
