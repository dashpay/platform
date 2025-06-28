//! Error handling for WASM SDK
//!
//! This module provides error types and conversion utilities for WASM bindings.

use std::fmt::Display;
use wasm_bindgen::prelude::*;

/// Error categories for better error handling in JavaScript
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Network-related errors (connection, timeout, etc.)
    Network,
    /// Serialization/deserialization errors
    Serialization,
    /// Validation errors (invalid input, etc.)
    Validation,
    /// Platform errors (from Dash Platform)
    Platform,
    /// Proof verification errors
    ProofVerification,
    /// State transition errors
    StateTransition,
    /// Identity-related errors
    Identity,
    /// Document-related errors
    Document,
    /// Contract-related errors
    Contract,
    /// Unknown or uncategorized errors
    Unknown,
}

#[wasm_bindgen]
#[derive(thiserror::Error, Debug)]
#[error("Dash SDK error: {message}")]
pub struct WasmError {
    #[wasm_bindgen(skip)]
    pub inner: Option<dpp::ProtocolError>,
    message: String,
    category: ErrorCategory,
}

#[wasm_bindgen]
impl WasmError {
    /// Get the error category
    #[wasm_bindgen(getter)]
    pub fn category(&self) -> ErrorCategory {
        self.category
    }

    /// Get the error message
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }
}

// Note: Removed From<Error> implementation as dash-sdk Error type is not available in WASM
// All errors are converted to WasmError through other means

impl From<dpp::ProtocolError> for WasmError {
    fn from(error: dpp::ProtocolError) -> Self {
        // Simplified error handling - just use the error string
        let message = error.to_string();
        let category = if message.contains("identifier") || message.contains("Identifier") {
            ErrorCategory::Validation
        } else if message.contains("contract") || message.contains("Contract") {
            ErrorCategory::Contract
        } else if message.contains("document") || message.contains("Document") {
            ErrorCategory::Document
        } else if message.contains("identity") || message.contains("Identity") {
            ErrorCategory::Identity
        } else if message.contains("transition") || message.contains("Transition") {
            ErrorCategory::StateTransition
        } else if message.contains("decod") || message.contains("Decod") || message.contains("encod") || message.contains("Encod") {
            ErrorCategory::Serialization
        } else {
            ErrorCategory::Platform
        };
        
        WasmError {
            inner: None,
            message,
            category,
        }
    }
}

pub(crate) fn to_js_error(e: impl Display) -> JsError {
    JsError::new(&format!("{}", e))
}

/// Helper function to create a formatted error
pub fn format_error(category: ErrorCategory, message: &str) -> JsValue {
    let error = WasmError {
        inner: None,
        message: message.to_string(),
        category,
    };
    JsValue::from(JsError::new(&error.to_string()))
}