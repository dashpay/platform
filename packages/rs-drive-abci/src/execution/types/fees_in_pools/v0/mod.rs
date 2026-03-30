use dpp::fee::Credits;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Struct containing the amount of processing and storage fees in the distribution pools
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeesInPoolsV0 {
    /// Amount of processing fees in the distribution pools
    pub processing_fees: Credits,
    /// Amount of storage fees in the distribution pools
    pub storage_fees: Credits,
}

impl fmt::Display for FeesInPoolsV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "FeesInPoolsV0 {{")?;
        writeln!(f, "    processing_fees: {},", self.processing_fees)?;
        writeln!(f, "    storage_fees: {}", self.storage_fees)?;
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats_correctly() {
        let fees = FeesInPoolsV0 {
            processing_fees: 100,
            storage_fees: 200,
        };
        let output = format!("{}", fees);
        assert!(output.contains("processing_fees: 100"));
        assert!(output.contains("storage_fees: 200"));
        assert!(output.contains("FeesInPoolsV0"));
    }

    #[test]
    fn display_with_zero_fees() {
        let fees = FeesInPoolsV0 {
            processing_fees: 0,
            storage_fees: 0,
        };
        let output = format!("{}", fees);
        assert!(output.contains("processing_fees: 0"));
        assert!(output.contains("storage_fees: 0"));
    }

    #[test]
    fn serialization_roundtrip() {
        let fees = FeesInPoolsV0 {
            processing_fees: 42,
            storage_fees: 84,
        };
        let serialized = serde_json::to_string(&fees).unwrap();
        let deserialized: FeesInPoolsV0 = serde_json::from_str(&serialized).unwrap();
        assert_eq!(fees, deserialized);
    }

    #[test]
    fn equality() {
        let a = FeesInPoolsV0 {
            processing_fees: 10,
            storage_fees: 20,
        };
        let b = FeesInPoolsV0 {
            processing_fees: 10,
            storage_fees: 20,
        };
        let c = FeesInPoolsV0 {
            processing_fees: 10,
            storage_fees: 21,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn clone_produces_equal_value() {
        let fees = FeesInPoolsV0 {
            processing_fees: 999,
            storage_fees: 888,
        };
        let cloned = fees;
        assert_eq!(fees, cloned);
    }
}
