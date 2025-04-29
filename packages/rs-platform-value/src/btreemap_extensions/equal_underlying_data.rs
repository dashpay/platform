use crate::Value;
use std::collections::BTreeMap;
/* ========================================================= *
 *   Trait: EqualUnderlyingData                              *
 * ========================================================= */

/// Compare two structures by the data they ultimately represent,
/// not necessarily by their concrete variants.
pub trait EqualUnderlyingData {
    fn equal_underlying_data(&self, other: &Self) -> bool;
}

/* ========================================================= *
 *   Impl for `BTreeMap<String, Value>`                      *
 * ========================================================= */

impl EqualUnderlyingData for &BTreeMap<String, Value> {
    fn equal_underlying_data(&self, other: &Self) -> bool {
        // quick size check
        if self.len() != other.len() {
            return false;
        }
        // every key must exist in both and values must match by underlying data
        self.iter().all(|(k, v_self)| {
            other
                .get(k)
                .map(|v_other| v_self.equal_underlying_data(v_other))
                .unwrap_or(false)
        })
    }
}

impl EqualUnderlyingData for BTreeMap<String, Value> {
    fn equal_underlying_data(&self, other: &Self) -> bool {
        // quick size check
        if self.len() != other.len() {
            return false;
        }
        // every key must exist in both and values must match by underlying data
        self.iter().all(|(k, v_self)| {
            other
                .get(k)
                .map(|v_other| v_self.equal_underlying_data(v_other))
                .unwrap_or(false)
        })
    }
}

impl EqualUnderlyingData for BTreeMap<&String, Value> {
    fn equal_underlying_data(&self, other: &Self) -> bool {
        // quick size check
        if self.len() != other.len() {
            return false;
        }
        // every key must exist in both and values must match by underlying data
        self.iter().all(|(k, v_self)| {
            other
                .get(k)
                .map(|v_other| v_self.equal_underlying_data(v_other))
                .unwrap_or(false)
        })
    }
}

impl EqualUnderlyingData for BTreeMap<&String, &Value> {
    fn equal_underlying_data(&self, other: &Self) -> bool {
        // quick size check
        if self.len() != other.len() {
            return false;
        }
        // every key must exist in both and values must match by underlying data
        self.iter().all(|(k, v_self)| {
            other
                .get(k)
                .map(|v_other| v_self.equal_underlying_data(v_other))
                .unwrap_or(false)
        })
    }
}

impl EqualUnderlyingData for BTreeMap<String, &Value> {
    fn equal_underlying_data(&self, other: &Self) -> bool {
        // quick size check
        if self.len() != other.len() {
            return false;
        }
        // every key must exist in both and values must match by underlying data
        self.iter().all(|(k, v_self)| {
            other
                .get(k)
                .map(|v_other| v_self.equal_underlying_data(v_other))
                .unwrap_or(false)
        })
    }
}
