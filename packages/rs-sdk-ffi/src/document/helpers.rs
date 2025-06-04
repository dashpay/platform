//! Helper functions for document operations

use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::prelude::UserFeeIncrease;
use dash_sdk::dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dash_sdk::dpp::state_transition::StateTransitionSigningOptions;
use dash_sdk::dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dash_sdk::dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use dash_sdk::dpp::tokens::token_payment_info::TokenPaymentInfo;

use crate::types::{
    DashSDKGasFeesPaidBy, DashSDKStateTransitionCreationOptions, DashSDKTokenPaymentInfo,
};
use crate::FFIError;

/// Convert FFI GasFeesPaidBy to Rust enum
pub unsafe fn convert_gas_fees_paid_by(ffi_value: DashSDKGasFeesPaidBy) -> GasFeesPaidBy {
    match ffi_value {
        DashSDKGasFeesPaidBy::DocumentOwner => GasFeesPaidBy::DocumentOwner,
        DashSDKGasFeesPaidBy::ContractOwner => GasFeesPaidBy::ContractOwner,
        DashSDKGasFeesPaidBy::PreferContractOwner => GasFeesPaidBy::PreferContractOwner,
    }
}

/// Convert FFI TokenPaymentInfo to Rust TokenPaymentInfo
pub unsafe fn convert_token_payment_info(
    ffi_token_payment_info: *const DashSDKTokenPaymentInfo,
) -> Result<Option<TokenPaymentInfo>, FFIError> {
    if ffi_token_payment_info.is_null() {
        return Ok(None);
    }

    let token_info = &*ffi_token_payment_info;

    let payment_token_contract_id = if token_info.payment_token_contract_id.is_null() {
        None
    } else {
        let id_bytes = &*token_info.payment_token_contract_id;
        Some(Identifier::from_bytes(id_bytes).map_err(|e| {
            FFIError::InternalError(format!("Invalid payment token contract ID: {}", e))
        })?)
    };

    let token_payment_info_v0 = TokenPaymentInfoV0 {
        payment_token_contract_id,
        token_contract_position: token_info.token_contract_position,
        minimum_token_cost: if token_info.minimum_token_cost == 0 {
            None
        } else {
            Some(token_info.minimum_token_cost)
        },
        maximum_token_cost: if token_info.maximum_token_cost == 0 {
            None
        } else {
            Some(token_info.maximum_token_cost)
        },
        gas_fees_paid_by: convert_gas_fees_paid_by(token_info.gas_fees_paid_by),
    };

    Ok(Some(TokenPaymentInfo::V0(token_payment_info_v0)))
}

/// Convert FFI StateTransitionCreationOptions to Rust StateTransitionCreationOptions
pub unsafe fn convert_state_transition_creation_options(
    ffi_options: *const DashSDKStateTransitionCreationOptions,
) -> Option<StateTransitionCreationOptions> {
    if ffi_options.is_null() {
        return None;
    }

    let options = &*ffi_options;

    let signing_options = StateTransitionSigningOptions {
        allow_signing_with_any_security_level: options.allow_signing_with_any_security_level,
        allow_signing_with_any_purpose: options.allow_signing_with_any_purpose,
    };

    Some(StateTransitionCreationOptions {
        signing_options,
        batch_feature_version: if options.batch_feature_version == 0 {
            None
        } else {
            Some(options.batch_feature_version)
        },
        method_feature_version: if options.method_feature_version == 0 {
            None
        } else {
            Some(options.method_feature_version)
        },
        base_feature_version: if options.base_feature_version == 0 {
            None
        } else {
            Some(options.base_feature_version)
        },
    })
}
