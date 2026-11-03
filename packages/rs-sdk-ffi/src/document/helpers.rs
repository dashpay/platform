//! Helper functions for document operations

use std::collections::BTreeMap;

use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dash_sdk::dpp::data_contract::document_type::DocumentTypeRef;
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dash_sdk::dpp::state_transition::StateTransitionSigningOptions;
use dash_sdk::dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dash_sdk::dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use dash_sdk::dpp::tokens::token_payment_info::TokenPaymentInfo;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::dpp::ProtocolError;

use crate::types::{
    DashSDKGasFeesPaidBy, DashSDKStateTransitionCreationOptions, DashSDKTokenPaymentInfo,
};
use crate::{DashSDKError, DashSDKErrorCode, FFIError};

/// Parse the properties JSON of a document to create, as `dash_sdk_document_create`
/// takes it: a JSON object keyed by property name, each value read into a platform
/// `Value` as JSON writes it (integers, booleans, strings, arrays and objects). Byte
/// arrays and identifiers are still the strings the caller wrote here;
/// [`build_document_from_properties`] decodes them against the document type.
///
/// Errors carry `InvalidParameter`: the text is not JSON, or not an object.
pub(crate) fn parse_document_properties_json(
    properties_json: &str,
) -> Result<BTreeMap<String, Value>, DashSDKError> {
    let properties_value: serde_json::Value =
        serde_json::from_str(properties_json).map_err(|e| {
            DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid properties JSON: {}", e),
            )
        })?;

    // Convert JSON to platform Value - handle hex strings for byte arrays
    serde_json::from_value::<BTreeMap<String, Value>>(properties_value).map_err(|e| {
        DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!("Failed to convert properties: {}", e),
        )
    })
}

/// Build the document `dash_sdk_document_create` creates from `properties`: they are
/// sanitized against `document_type` (hex or base64 strings become byte arrays, base58
/// or hex strings identifiers, integers narrow to their declared width, typed array
/// elements included), then `DocumentType::create_document_from_data` builds the
/// revision-1 document owned by `owner_id`, its id derived from `entropy`, typing the
/// value at each of the document type's identifier paths as an identifier. Block
/// heights are left at 0 for Platform to set.
pub(crate) fn build_document_from_properties(
    document_type: DocumentTypeRef<'_>,
    mut properties: BTreeMap<String, Value>,
    owner_id: Identifier,
    entropy: [u8; 32],
    platform_version: &PlatformVersion,
) -> Result<Document, ProtocolError> {
    document_type.sanitize_document_properties(&mut properties);
    document_type.create_document_from_data(
        properties.into(),
        owner_id,
        0, // block_height - will be set by platform
        0, // core_block_height - will be set by platform
        entropy,
        platform_version,
    )
}

/// Convert FFI GasFeesPaidBy to Rust enum
///
/// # Safety
/// - `ffi_value` is passed by value; no pointer preconditions.
pub unsafe fn convert_gas_fees_paid_by(ffi_value: DashSDKGasFeesPaidBy) -> GasFeesPaidBy {
    match ffi_value {
        DashSDKGasFeesPaidBy::DocumentOwner => GasFeesPaidBy::DocumentOwner,
        DashSDKGasFeesPaidBy::GasFeesContractOwner => GasFeesPaidBy::ContractOwner,
        DashSDKGasFeesPaidBy::GasFeesPreferContractOwner => GasFeesPaidBy::PreferContractOwner,
    }
}

/// Convert FFI TokenPaymentInfo to Rust TokenPaymentInfo
///
/// # Safety
/// - `ffi_token_payment_info` may be null; when non-null it must be a valid pointer to a `DashSDKTokenPaymentInfo`
///   that remains valid for the duration of the call.
#[allow(clippy::result_large_err)]
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
///
/// # Safety
/// - `ffi_options` may be null; when non-null it must be a valid pointer to a `DashSDKStateTransitionCreationOptions`
///   that remains valid for the duration of the call.
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
        action_fee_agreement: None,
        // 0 leaves a contested create stating the fund to join, which rs-sdk reads when it signs
        contest_fund: if options.contest_fund == 0 {
            None
        } else {
            Some(options.contest_fund)
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creation_options(contest_fund: u64) -> DashSDKStateTransitionCreationOptions {
        DashSDKStateTransitionCreationOptions {
            allow_signing_with_any_security_level: false,
            allow_signing_with_any_purpose: false,
            batch_feature_version: 0,
            method_feature_version: 0,
            base_feature_version: 0,
            contest_fund,
        }
    }

    #[test]
    fn should_state_the_contest_fund_only_when_it_is_not_zero() {
        let unset = unsafe { convert_state_transition_creation_options(&creation_options(0)) };
        assert_eq!(unset, Some(StateTransitionCreationOptions::default()));

        let stated = unsafe { convert_state_transition_creation_options(&creation_options(7)) };
        assert_eq!(stated.and_then(|options| options.contest_fund), Some(7));
    }
}
