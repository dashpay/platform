use dash_sdk::Error;
use js_sys::Reflect;
use serde_json::json;
use std::fmt::Display;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;
use wasm_bindgen::JsValue;
use serde_wasm_bindgen::to_value as to_js_value;
use rs_dapi_client::CanRetry;

#[wasm_bindgen]
#[derive(thiserror::Error, Debug)]
#[error("Dash SDK error: {0:?}")]
pub struct WasmError(#[from] Error);

pub(crate) fn to_js_error(e: impl Display) -> JsError {
    JsError::new(&format!("{}", e))
}

fn set_prop(_obj: &JsValue, _key: &str, _value: JsValue) { /* no-op for JsError */ }

pub(crate) fn new_structured_error(message: &str, _code: &str, _kind: &str, _details: Option<serde_json::Value>, _retriable: Option<bool>) -> JsError {
    // NOTE: JsError doesn't expose a stable way to set custom properties across WASM boundary.
    // We still return a JsError with the human-readable message. Network/SDK errors are mapped
    // centrally via map_sdk_error to produce consistent messages.
    JsError::new(message)
}

pub(crate) fn map_sdk_error(err: Error) -> JsError {
    use dash_sdk::Error as E;
    match err {
            E::Config(msg) => new_structured_error(&format!("SDK misconfigured: {msg}"), "E_CONFIG", "argument", None, Some(false)),
            E::Drive(e) => new_structured_error(&format!("Drive error: {e}"), "E_INTERNAL", "internal", None, Some(false)),
            E::DriveProofError(e, _proof, _bi) => new_structured_error(&format!("Drive proof error: {e}"), "E_PROOF", "proof", None, Some(false)),
            E::Protocol(e) => new_structured_error(&format!("Protocol error: {e}"), "E_PROTOCOL", "protocol", None, Some(false)),
            E::Proof(e) => new_structured_error(&format!("Proof verification error: {e}"), "E_PROOF", "proof", None, Some(false)),
            E::InvalidProvedResponse(msg) => new_structured_error(&format!("Invalid proved response: {msg}"), "E_PROOF", "proof", None, Some(false)),
            E::DapiClientError(e) => new_structured_error(&format!("DAPI client error: {e}"), "E_NETWORK", "network", None, Some(e.can_retry())),
            #[cfg(feature = "mocks")]
            E::DapiMocksError(e) => new_structured_error(&format!("DAPI mocks error: {e}"), "E_INTERNAL", "internal", None, Some(false)),
            E::CoreError(e) => new_structured_error(&format!("Core error: {e}"), "E_INTERNAL", "internal", None, Some(false)),
            E::MerkleBlockError(e) => new_structured_error(&format!("Merkle block error: {e}"), "E_INTERNAL", "internal", None, Some(false)),
            E::CoreClientError(e) => new_structured_error(&format!("Core client error: {e}"), "E_NETWORK", "network", None, Some(false)),
            E::MissingDependency(what, id) => new_structured_error(
                &format!("Required {what} not found: {id}"),
                "E_NOT_FOUND",
                "not_found",
                Some(json!({"resource": what, "id": id })),
                Some(false),
            ),
            E::TotalCreditsNotFound => new_structured_error(
                "Total credits in Platform not found",
                "E_INTERNAL",
                "internal",
                None,
                Some(false),
            ),
            E::EpochNotFound => new_structured_error(
                "No epoch found on Platform",
                "E_INTERNAL",
                "internal",
                None,
                Some(false),
            ),
            E::TimeoutReached(dur, op) => new_structured_error(
                &format!("SDK operation timeout {} secs reached: {}", dur.as_secs(), op),
                "E_TIMEOUT",
                "timeout",
                Some(json!({"operation": op, "seconds": dur.as_secs()})),
                Some(true),
            ),
            E::AlreadyExists(what) => new_structured_error(&format!("Already exists: {what}"), "E_ALREADY_EXISTS", "conflict", None, Some(false)),
            E::Generic(msg) => new_structured_error(&msg, "E_INTERNAL", "internal", None, Some(false)),
            E::ContextProviderError(e) => new_structured_error(&format!("Context provider error: {e}"), "E_CONTEXT", "context", None, Some(false)),
            E::Cancelled(msg) => new_structured_error(&format!("Operation cancelled: {msg}"), "E_CANCELLED", "cancelled", None, Some(true)),
            E::StaleNode(se) => {
                let details = match se {
                    dash_sdk::error::StaleNodeError::Height{expected_height, received_height, tolerance_blocks} =>
                        json!({"type":"height","expected_height":expected_height,"received_height":received_height,"tolerance_blocks":tolerance_blocks}),
                    dash_sdk::error::StaleNodeError::Time{expected_timestamp_ms, received_timestamp_ms, tolerance_ms} =>
                        json!({"type":"time","expected_timestamp_ms":expected_timestamp_ms,"received_timestamp_ms":received_timestamp_ms,"tolerance_ms":tolerance_ms}),
                };
                new_structured_error(&format!("Stale node: {se}"), "E_NETWORK_UNAVAILABLE", "network", Some(details), Some(true))
            }
            E::StateTransitionBroadcastError(be) => {
                let mut d = json!({"code": be.code, "message": be.message});
                if let Some(cause) = be.cause { d["cause"] = json!(format!("{cause}")); }
                new_structured_error(&format!("state transition broadcast error: {}", be.message), "E_BROADCAST", "broadcast", Some(d), Some(false))
            }
        }
}
