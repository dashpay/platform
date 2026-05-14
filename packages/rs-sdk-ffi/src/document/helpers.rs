//! Helper functions for document operations

use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dash_sdk::dpp::state_transition::StateTransitionSigningOptions;
use dash_sdk::dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dash_sdk::dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use dash_sdk::dpp::tokens::token_payment_info::TokenPaymentInfo;

use crate::types::{
    DashSDKGasFeesPaidBy, DashSDKStateTransitionCreationOptions, DashSDKTokenPaymentInfo,
};
use crate::FFIError;

/// Map a `dash_sdk::Error` produced by a document state-transition
/// builder (build / sign / SDK broadcast helper) into an `FFIError`,
/// preserving caller-supplied context for non-`InvalidArgument` variants
/// while letting `InvalidArgument` flow through the typed
/// `FFIError::SDKError` → `DashSDKErrorCode::InvalidParameter` branch
/// in `error.rs`.
///
/// Without this routing, a typed `Error::InvalidArgument` from the new
/// strict create/replace guards in rs-sdk would be wrapped as
/// `FFIError::InternalError(format!("{context}: {}", e))` and surface as
/// `DashSDKErrorCode::InternalError`, hiding the precise classification
/// from FFI callers.
pub(crate) fn map_document_sdk_error(e: dash_sdk::Error, context: &str) -> FFIError {
    if matches!(e, dash_sdk::Error::InvalidArgument(_)) {
        FFIError::SDKError(e)
    } else {
        FFIError::InternalError(format!("{}: {}", context, e))
    }
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DashSDKErrorCode;

    /// A typed `dash_sdk::Error::InvalidArgument` from the document
    /// builder/sign paths must flow through `FFIError::SDKError` (not
    /// `InternalError`) so the `From<FFIError> for DashSDKError` typed
    /// dispatch in `error.rs` can map it to
    /// `DashSDKErrorCode::InvalidParameter`. Without this routing, the
    /// new strict create/replace guards in rs-sdk would surface as
    /// `InternalError` to FFI callers.
    #[test]
    fn map_document_sdk_error_routes_invalid_argument_to_invalid_parameter() {
        let sdk_err = dash_sdk::Error::InvalidArgument("entropy mismatch".to_string());
        let ffi_err = map_document_sdk_error(sdk_err, "Failed to create document transition");
        assert!(
            matches!(
                ffi_err,
                FFIError::SDKError(dash_sdk::Error::InvalidArgument(_))
            ),
            "InvalidArgument must pass through as FFIError::SDKError, got: {ffi_err:?}"
        );

        // End-to-end through the public `From<FFIError> for DashSDKError`
        // conversion: the user-facing error code must be InvalidParameter,
        // not InternalError.
        let api_err: crate::DashSDKError = ffi_err.into();
        assert_eq!(api_err.code, DashSDKErrorCode::InvalidParameter);
    }

    /// Non-`InvalidArgument` `dash_sdk::Error` variants must keep the
    /// caller-supplied context prefix (e.g. "Failed to create document
    /// transition") so existing FFI error messages are not regressed by
    /// the typed pass-through.
    #[test]
    fn map_document_sdk_error_preserves_context_for_non_invalid_argument() {
        // Use a `Protocol` variant — anything that is not
        // `InvalidArgument`. The exact variant does not matter; what
        // matters is that the context prefix is retained.
        let sdk_err = dash_sdk::Error::Generic("boom".to_string());
        let ffi_err = map_document_sdk_error(sdk_err, "Failed to create document transition");
        match ffi_err {
            FFIError::InternalError(msg) => {
                assert!(
                    msg.starts_with("Failed to create document transition:"),
                    "expected context prefix, got: {msg}"
                );
                assert!(
                    msg.contains("boom"),
                    "expected underlying error in msg, got: {msg}"
                );
            }
            other => {
                panic!("expected FFIError::InternalError for non-InvalidArgument, got: {other:?}")
            }
        }
    }
}
