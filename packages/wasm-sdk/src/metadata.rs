//! # Metadata Module
//!
//! This module provides functionality for metadata verification including
//! height and time tolerance checks.

use js_sys::{Date, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Metadata from a Platform response
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Metadata {
    height: u64,
    core_chain_locked_height: u32,
    epoch: u32,
    time_ms: u64,
    protocol_version: u32,
    chain_id: String,
}

#[wasm_bindgen]
impl Metadata {
    /// Create new metadata
    #[wasm_bindgen(constructor)]
    pub fn new(
        height: u64,
        core_chain_locked_height: u32,
        epoch: u32,
        time_ms: u64,
        protocol_version: u32,
        chain_id: String,
    ) -> Metadata {
        Metadata {
            height,
            core_chain_locked_height,
            epoch,
            time_ms,
            protocol_version,
            chain_id,
        }
    }

    /// Get the block height
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u64 {
        self.height
    }

    /// Get the core chain locked height
    #[wasm_bindgen(getter, js_name = coreChainLockedHeight)]
    pub fn core_chain_locked_height(&self) -> u32 {
        self.core_chain_locked_height
    }

    /// Get the epoch
    #[wasm_bindgen(getter)]
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    /// Get the time in milliseconds
    #[wasm_bindgen(getter, js_name = timeMs)]
    pub fn time_ms(&self) -> u64 {
        self.time_ms
    }

    /// Get the protocol version
    #[wasm_bindgen(getter, js_name = protocolVersion)]
    pub fn protocol_version(&self) -> u32 {
        self.protocol_version
    }

    /// Get the chain ID
    #[wasm_bindgen(getter, js_name = chainId)]
    pub fn chain_id(&self) -> String {
        self.chain_id.clone()
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"height".into(), &self.height.into())
            .map_err(|_| JsError::new("Failed to set height"))?;
        Reflect::set(
            &obj,
            &"coreChainLockedHeight".into(),
            &self.core_chain_locked_height.into(),
        )
        .map_err(|_| JsError::new("Failed to set core chain locked height"))?;
        Reflect::set(&obj, &"epoch".into(), &self.epoch.into())
            .map_err(|_| JsError::new("Failed to set epoch"))?;
        Reflect::set(&obj, &"timeMs".into(), &self.time_ms.into())
            .map_err(|_| JsError::new("Failed to set time"))?;
        Reflect::set(
            &obj,
            &"protocolVersion".into(),
            &self.protocol_version.into(),
        )
        .map_err(|_| JsError::new("Failed to set protocol version"))?;
        Reflect::set(&obj, &"chainId".into(), &self.chain_id.clone().into())
            .map_err(|_| JsError::new("Failed to set chain ID"))?;
        Ok(obj.into())
    }
}

/// Configuration for metadata verification
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct MetadataVerificationConfig {
    /// Maximum allowed height difference
    max_height_difference: u64,
    /// Maximum allowed time difference in milliseconds
    max_time_difference_ms: u64,
    /// Whether to verify time
    verify_time: bool,
    /// Whether to verify height
    verify_height: bool,
    /// Whether to verify chain ID
    verify_chain_id: bool,
    /// Expected chain ID
    expected_chain_id: Option<String>,
}

#[wasm_bindgen]
impl MetadataVerificationConfig {
    /// Create default verification config
    #[wasm_bindgen(constructor)]
    pub fn new() -> MetadataVerificationConfig {
        MetadataVerificationConfig {
            max_height_difference: 100,     // ~4 hours at 2.5 min blocks
            max_time_difference_ms: 300000, // 5 minutes
            verify_time: true,
            verify_height: true,
            verify_chain_id: true,
            expected_chain_id: None,
        }
    }

    /// Set maximum height difference
    #[wasm_bindgen(js_name = setMaxHeightDifference)]
    pub fn set_max_height_difference(&mut self, blocks: u64) {
        self.max_height_difference = blocks;
    }

    /// Set maximum time difference
    #[wasm_bindgen(js_name = setMaxTimeDifference)]
    pub fn set_max_time_difference(&mut self, ms: u64) {
        self.max_time_difference_ms = ms;
    }

    /// Enable/disable time verification
    #[wasm_bindgen(js_name = setVerifyTime)]
    pub fn set_verify_time(&mut self, verify: bool) {
        self.verify_time = verify;
    }

    /// Enable/disable height verification
    #[wasm_bindgen(js_name = setVerifyHeight)]
    pub fn set_verify_height(&mut self, verify: bool) {
        self.verify_height = verify;
    }

    /// Enable/disable chain ID verification
    #[wasm_bindgen(js_name = setVerifyChainId)]
    pub fn set_verify_chain_id(&mut self, verify: bool) {
        self.verify_chain_id = verify;
    }

    /// Set expected chain ID
    #[wasm_bindgen(js_name = setExpectedChainId)]
    pub fn set_expected_chain_id(&mut self, chain_id: String) {
        self.expected_chain_id = Some(chain_id);
    }
}

impl Default for MetadataVerificationConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of metadata verification
#[wasm_bindgen]
pub struct MetadataVerificationResult {
    valid: bool,
    height_valid: Option<bool>,
    time_valid: Option<bool>,
    chain_id_valid: Option<bool>,
    height_difference: Option<u64>,
    time_difference_ms: Option<u64>,
    error_message: Option<String>,
}

#[wasm_bindgen]
impl MetadataVerificationResult {
    /// Check if metadata is valid
    #[wasm_bindgen(getter)]
    pub fn valid(&self) -> bool {
        self.valid
    }

    /// Check if height is valid
    #[wasm_bindgen(getter, js_name = heightValid)]
    pub fn height_valid(&self) -> Option<bool> {
        self.height_valid
    }

    /// Check if time is valid
    #[wasm_bindgen(getter, js_name = timeValid)]
    pub fn time_valid(&self) -> Option<bool> {
        self.time_valid
    }

    /// Check if chain ID is valid
    #[wasm_bindgen(getter, js_name = chainIdValid)]
    pub fn chain_id_valid(&self) -> Option<bool> {
        self.chain_id_valid
    }

    /// Get height difference
    #[wasm_bindgen(getter, js_name = heightDifference)]
    pub fn height_difference(&self) -> Option<u64> {
        self.height_difference
    }

    /// Get time difference in milliseconds
    #[wasm_bindgen(getter, js_name = timeDifferenceMs)]
    pub fn time_difference_ms(&self) -> Option<u64> {
        self.time_difference_ms
    }

    /// Get error message if validation failed
    #[wasm_bindgen(getter, js_name = errorMessage)]
    pub fn error_message(&self) -> Option<String> {
        self.error_message.clone()
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"valid".into(), &self.valid.into())
            .map_err(|_| JsError::new("Failed to set valid"))?;

        if let Some(height_valid) = self.height_valid {
            Reflect::set(&obj, &"heightValid".into(), &height_valid.into())
                .map_err(|_| JsError::new("Failed to set height valid"))?;
        }

        if let Some(time_valid) = self.time_valid {
            Reflect::set(&obj, &"timeValid".into(), &time_valid.into())
                .map_err(|_| JsError::new("Failed to set time valid"))?;
        }

        if let Some(chain_id_valid) = self.chain_id_valid {
            Reflect::set(&obj, &"chainIdValid".into(), &chain_id_valid.into())
                .map_err(|_| JsError::new("Failed to set chain ID valid"))?;
        }

        if let Some(height_diff) = self.height_difference {
            Reflect::set(&obj, &"heightDifference".into(), &height_diff.into())
                .map_err(|_| JsError::new("Failed to set height difference"))?;
        }

        if let Some(time_diff) = self.time_difference_ms {
            Reflect::set(&obj, &"timeDifferenceMs".into(), &time_diff.into())
                .map_err(|_| JsError::new("Failed to set time difference"))?;
        }

        if let Some(ref error) = self.error_message {
            Reflect::set(&obj, &"errorMessage".into(), &error.clone().into())
                .map_err(|_| JsError::new("Failed to set error message"))?;
        }

        Ok(obj.into())
    }
}

/// Verify metadata against current state
#[wasm_bindgen(js_name = verifyMetadata)]
pub fn verify_metadata(
    metadata: &Metadata,
    current_height: u64,
    current_time_ms: Option<f64>,
    config: &MetadataVerificationConfig,
) -> MetadataVerificationResult {
    let mut result = MetadataVerificationResult {
        valid: true,
        height_valid: None,
        time_valid: None,
        chain_id_valid: None,
        height_difference: None,
        time_difference_ms: None,
        error_message: None,
    };

    // Verify height
    if config.verify_height {
        let height_diff = if metadata.height > current_height {
            metadata.height - current_height
        } else {
            current_height - metadata.height
        };

        result.height_difference = Some(height_diff);
        result.height_valid = Some(height_diff <= config.max_height_difference);

        if height_diff > config.max_height_difference {
            result.valid = false;
            result.error_message = Some(format!(
                "Height difference {} exceeds maximum allowed {}",
                height_diff, config.max_height_difference
            ));
        }
    }

    // Verify time
    if config.verify_time {
        let current_time = current_time_ms.unwrap_or_else(Date::now) as u64;
        let time_diff = if metadata.time_ms > current_time {
            metadata.time_ms - current_time
        } else {
            current_time - metadata.time_ms
        };

        result.time_difference_ms = Some(time_diff);
        result.time_valid = Some(time_diff <= config.max_time_difference_ms);

        if time_diff > config.max_time_difference_ms {
            result.valid = false;
            result.error_message = Some(format!(
                "Time difference {} ms exceeds maximum allowed {} ms",
                time_diff, config.max_time_difference_ms
            ));
        }
    }

    // Verify chain ID
    if config.verify_chain_id {
        if let Some(ref expected_chain_id) = config.expected_chain_id {
            let chain_id_matches = &metadata.chain_id == expected_chain_id;
            result.chain_id_valid = Some(chain_id_matches);

            if !chain_id_matches {
                result.valid = false;
                result.error_message = Some(format!(
                    "Chain ID '{}' does not match expected '{}'",
                    metadata.chain_id, expected_chain_id
                ));
            }
        }
    }

    result
}

/// Compare two metadata objects and determine which is more recent
#[wasm_bindgen(js_name = compareMetadata)]
pub fn compare_metadata(metadata1: &Metadata, metadata2: &Metadata) -> i32 {
    // First compare by height
    if metadata1.height > metadata2.height {
        return 1;
    } else if metadata1.height < metadata2.height {
        return -1;
    }

    // If heights are equal, compare by time
    if metadata1.time_ms > metadata2.time_ms {
        return 1;
    } else if metadata1.time_ms < metadata2.time_ms {
        return -1;
    }

    // If both height and time are equal
    0
}

/// Get the most recent metadata from a list
#[wasm_bindgen(js_name = getMostRecentMetadata)]
pub fn get_most_recent_metadata(metadata_list: Vec<JsValue>) -> Result<Metadata, JsError> {
    if metadata_list.is_empty() {
        return Err(JsError::new("Metadata list is empty"));
    }

    let mut metadata_objects = Vec::new();

    for js_metadata in metadata_list {
        let height = Reflect::get(&js_metadata, &"height".into())
            .map_err(|_| JsError::new("Failed to get height"))?
            .as_f64()
            .ok_or_else(|| JsError::new("Height must be a number"))? as u64;

        let core_chain_locked_height = Reflect::get(&js_metadata, &"coreChainLockedHeight".into())
            .map_err(|_| JsError::new("Failed to get core chain locked height"))?
            .as_f64()
            .ok_or_else(|| JsError::new("Core chain locked height must be a number"))?
            as u32;

        let epoch = Reflect::get(&js_metadata, &"epoch".into())
            .map_err(|_| JsError::new("Failed to get epoch"))?
            .as_f64()
            .ok_or_else(|| JsError::new("Epoch must be a number"))? as u32;

        let time_ms = Reflect::get(&js_metadata, &"timeMs".into())
            .map_err(|_| JsError::new("Failed to get time"))?
            .as_f64()
            .ok_or_else(|| JsError::new("Time must be a number"))? as u64;

        let protocol_version = Reflect::get(&js_metadata, &"protocolVersion".into())
            .map_err(|_| JsError::new("Failed to get protocol version"))?
            .as_f64()
            .ok_or_else(|| JsError::new("Protocol version must be a number"))?
            as u32;

        let chain_id = Reflect::get(&js_metadata, &"chainId".into())
            .map_err(|_| JsError::new("Failed to get chain ID"))?
            .as_string()
            .ok_or_else(|| JsError::new("Chain ID must be a string"))?;

        metadata_objects.push(Metadata {
            height,
            core_chain_locked_height,
            epoch,
            time_ms,
            protocol_version,
            chain_id,
        });
    }

    // Find the most recent metadata
    metadata_objects
        .into_iter()
        .max_by(|a, b| {
            if a.height != b.height {
                a.height.cmp(&b.height)
            } else {
                a.time_ms.cmp(&b.time_ms)
            }
        })
        .ok_or_else(|| JsError::new("Failed to find most recent metadata"))
}

/// Check if metadata is within acceptable staleness bounds
#[wasm_bindgen(js_name = isMetadataStale)]
pub fn is_metadata_stale(
    metadata: &Metadata,
    max_age_ms: u64,
    max_height_behind: u64,
    current_height: Option<u64>,
) -> bool {
    // Check time staleness
    let current_time = Date::now() as u64;
    if current_time > metadata.time_ms && (current_time - metadata.time_ms) > max_age_ms {
        return true;
    }

    // Check height staleness if current height is provided
    if let Some(current) = current_height {
        if current > metadata.height && (current - metadata.height) > max_height_behind {
            return true;
        }
    }

    false
}
