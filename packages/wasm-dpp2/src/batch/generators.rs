use crate::batch::prefunded_voting_balance::PrefundedVotingBalanceWASM;
use crate::batch::token_payment_info::TokenPaymentInfoWASM;
use crate::document::DocumentWASM;
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
use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransitionV0;
use dpp::state_transition::batch_transition::document_replace_transition::DocumentReplaceTransitionV0;
use dpp::state_transition::batch_transition::{
    DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition,
};
use dpp::tokens::token_payment_info::TokenPaymentInfo;

pub fn generate_create_transition(
    document: DocumentWASM,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    prefunded_voting_balance: Option<PrefundedVotingBalanceWASM>,
    token_payment_info: Option<TokenPaymentInfoWASM>,
) -> DocumentCreateTransition {
    DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
        base: DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: document.rs_get_id(),
            identity_contract_nonce,
            document_type_name,
            data_contract_id: document.rs_get_data_contract_id(),
            token_payment_info: token_payment_info.map(TokenPaymentInfo::from),
        }),
        entropy: document.rs_get_entropy().unwrap(),
        data: document.rs_get_properties(),
        prefunded_voting_balance: prefunded_voting_balance.map(|pb| pb.into()),
    })
}

pub fn generate_delete_transition(
    document: DocumentWASM,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    token_payment_info: Option<TokenPaymentInfoWASM>,
) -> DocumentDeleteTransition {
    DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 {
        base: DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: document.rs_get_id(),
            identity_contract_nonce,
            document_type_name,
            data_contract_id: document.rs_get_data_contract_id(),
            token_payment_info: token_payment_info.map(TokenPaymentInfo::from),
        }),
    })
}

pub fn generate_replace_transition(
    document: DocumentWASM,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    token_payment_info: Option<TokenPaymentInfoWASM>,
) -> DocumentReplaceTransition {
    DocumentReplaceTransition::V0(DocumentReplaceTransitionV0 {
        base: DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: document.rs_get_id(),
            identity_contract_nonce,
            document_type_name,
            data_contract_id: document.rs_get_data_contract_id(),
            token_payment_info: token_payment_info.map(TokenPaymentInfo::from),
        }),
        revision: document.get_revision().unwrap() + 1,
        data: document.rs_get_properties(),
    })
}

pub fn generate_transfer_transition(
    document: DocumentWASM,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    recipient_owner_id: Identifier,
    token_payment_info: Option<TokenPaymentInfoWASM>,
) -> DocumentTransferTransition {
    DocumentTransferTransition::V0(DocumentTransferTransitionV0 {
        base: DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: document.rs_get_id(),
            identity_contract_nonce,
            document_type_name,
            data_contract_id: document.rs_get_data_contract_id(),
            token_payment_info: token_payment_info.map(TokenPaymentInfo::from),
        }),
        revision: document.get_revision().unwrap() + 1,
        recipient_owner_id,
    })
}

pub fn generate_update_price_transition(
    document: DocumentWASM,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    price: Credits,
    token_payment_info: Option<TokenPaymentInfoWASM>,
) -> DocumentUpdatePriceTransition {
    DocumentUpdatePriceTransition::V0(DocumentUpdatePriceTransitionV0 {
        base: DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: document.rs_get_id(),
            identity_contract_nonce,
            document_type_name,
            data_contract_id: document.rs_get_data_contract_id(),
            token_payment_info: token_payment_info.map(TokenPaymentInfo::from),
        }),
        revision: document.get_revision().unwrap() + 1,
        price,
    })
}

pub fn generate_purchase_transition(
    document: DocumentWASM,
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
    price: Credits,
    token_payment_info: Option<TokenPaymentInfoWASM>,
) -> DocumentPurchaseTransition {
    DocumentPurchaseTransition::V0(DocumentPurchaseTransitionV0 {
        base: DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: document.rs_get_id(),
            identity_contract_nonce,
            document_type_name,
            data_contract_id: document.rs_get_data_contract_id(),
            token_payment_info: token_payment_info.map(TokenPaymentInfo::from),
        }),
        revision: document.get_revision().unwrap() + 1,
        price,
    })
}
