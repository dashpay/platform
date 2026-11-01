//! Decision proposals (protocol version 15): yes/no questions any identity puts to the
//! masternodes, as `proposal` documents of the decision proposals system contract. Thin
//! bindings over `dash_sdk::platform::decision_proposals` and
//! `dash_sdk::platform::yes_no_vote_polls`: rs-sdk decides everything (which documents are
//! proposals, the poll's id, the vote); these functions read C arguments, call it, and write
//! the result.
//!
//! # Creating and editing a proposal
//!
//! A proposal is created and edited through the platform wallet, which signs with the
//! owner's identity keys: `platform_wallet_decision_proposal_create` and
//! `platform_wallet_decision_proposal_edit` in `rs-platform-wallet-ffi`. rs-sdk builds the
//! proposal (the create states the decision proposal fund itself) and the edit (the next
//! revision, refusing one that changes nothing); the wallet reads the proposal once for an
//! edit and puts it at the revision after the one read, so Platform refuses an edit another
//! one overtook instead of letting it overwrite that one.
//!
//! # Reading
//!
//! The reads return JSON, null when what they read is absent. A proposal is always the same
//! object (see [`dash_sdk_decision_proposal_fetch`]); the listings return arrays of it.
//!
//! # Voting
//!
//! [`dash_sdk_decision_proposal_cast_vote`] answers a proposal for a masternode from its voting
//! private key and pro_tx_hash, exactly as `dash_sdk_contested_resource_cast_vote` votes on a
//! contest (see `crate::voting::masternode_vote`).

mod json;
mod queries;
mod transitions;

pub use queries::*;
pub use transitions::*;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKResult, FFIError};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use std::ffi::{c_char, CStr, CString};

/// The SDK behind `sdk_handle`.
///
/// # Safety
/// `sdk_handle` must be null or point to an initialized `SDKHandle` that outlives `'a`.
unsafe fn sdk_wrapper<'a>(sdk_handle: *const SDKHandle) -> Result<&'a SDKWrapper, FFIError> {
    if sdk_handle.is_null() {
        return Err(FFIError::InvalidParameter("SDK handle is null".to_string()));
    }
    Ok(&*(sdk_handle as *const SDKWrapper))
}

/// The UTF-8 string `field` points to.
///
/// # Safety
/// `ptr` must be null or point to a NUL-terminated string that outlives `'a`.
unsafe fn required_str<'a>(ptr: *const c_char, field: &str) -> Result<&'a str, FFIError> {
    if ptr.is_null() {
        return Err(FFIError::InvalidParameter(format!("{field} is null")));
    }
    optional_str(ptr, field).map(|value| value.unwrap_or_default())
}

/// The UTF-8 string `field` points to, `None` when it is null.
///
/// # Safety
/// `ptr` must be null or point to a NUL-terminated string that outlives `'a`.
unsafe fn optional_str<'a>(ptr: *const c_char, field: &str) -> Result<Option<&'a str>, FFIError> {
    if ptr.is_null() {
        return Ok(None);
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map(Some)
        .map_err(|e| FFIError::InvalidParameter(format!("{field} is not UTF-8: {e}")))
}

/// The base58 identifier `field` points to.
///
/// # Safety
/// `ptr` must be null or point to a NUL-terminated string.
unsafe fn required_identifier(ptr: *const c_char, field: &str) -> Result<Identifier, FFIError> {
    parse_identifier(required_str(ptr, field)?, field)
}

/// The base58 identifier `field` points to, `None` when it is null.
///
/// # Safety
/// `ptr` must be null or point to a NUL-terminated string.
unsafe fn optional_identifier(
    ptr: *const c_char,
    field: &str,
) -> Result<Option<Identifier>, FFIError> {
    optional_str(ptr, field)?
        .map(|value| parse_identifier(value, field))
        .transpose()
}

fn parse_identifier(value: &str, field: &str) -> Result<Identifier, FFIError> {
    Identifier::from_string(value, Encoding::Base58)
        .map_err(|e| FFIError::InvalidParameter(format!("{field} is not a base58 identifier: {e}")))
}

/// A result holding `string` as an owned C string (freed with `dash_sdk_string_free`), or no
/// data for `None`.
fn string_result(string: Result<Option<String>, FFIError>) -> DashSDKResult {
    match string.and_then(|string| string.map(CString::new).transpose().map_err(Into::into)) {
        Ok(Some(string)) => DashSDKResult::success_string(string.into_raw()),
        Ok(None) => DashSDKResult::success(std::ptr::null_mut()),
        Err(error) => DashSDKResult::error(error.into()),
    }
}

/// A result holding `json` written as a string, or no data for `None`.
fn json_result(json: Result<Option<serde_json::Value>, FFIError>) -> DashSDKResult {
    string_result(json.and_then(|json| {
        json.map(|json| serde_json::to_string(&json))
            .transpose()
            .map_err(Into::into)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dash_sdk_error_free, dash_sdk_string_free, DashSDKErrorCode, DashSDKResultDataType,
    };

    #[test]
    fn should_read_a_base58_identifier_and_refuse_anything_else() {
        let id = Identifier::new([7; 32]);
        let base58 = CString::new(id.to_string(Encoding::Base58)).expect("no NUL");
        let not_base58 = CString::new("0OIl").expect("no NUL");
        let too_short = CString::new("3mJr7AoUXx2Wqd").expect("no NUL");
        unsafe {
            assert_eq!(
                required_identifier(base58.as_ptr(), "id").expect("reads"),
                id
            );
            assert_eq!(
                optional_identifier(base58.as_ptr(), "id").expect("reads"),
                Some(id)
            );
            assert_eq!(
                optional_identifier(std::ptr::null(), "id").expect("reads"),
                None
            );
            for refused in [
                required_identifier(std::ptr::null(), "id"),
                required_identifier(not_base58.as_ptr(), "id"),
                required_identifier(too_short.as_ptr(), "id"),
            ] {
                assert!(
                    matches!(refused, Err(FFIError::InvalidParameter(_))),
                    "{refused:?}"
                );
            }
        }
    }

    #[test]
    fn should_return_json_as_an_owned_string_and_absence_as_no_data() {
        let result = json_result(Ok(Some(serde_json::json!({"a": 1}))));
        assert!(result.error.is_null());
        assert_eq!(result.data_type, DashSDKResultDataType::String);
        unsafe {
            let string = CStr::from_ptr(result.data as *const c_char);
            assert_eq!(string.to_str().expect("UTF-8"), r#"{"a":1}"#);
            dash_sdk_string_free(result.data as *mut c_char);
        }

        let absent = json_result(Ok(None));
        assert!(absent.error.is_null());
        assert!(absent.data.is_null());
        assert_eq!(absent.data_type, DashSDKResultDataType::NoData);

        let failed = json_result(Err(FFIError::InvalidParameter("bad".to_string())));
        assert!(failed.data.is_null());
        unsafe {
            assert_eq!((*failed.error).code, DashSDKErrorCode::InvalidParameter);
            dash_sdk_error_free(failed.error);
        }
    }
}
