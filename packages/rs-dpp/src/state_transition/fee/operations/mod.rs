mod delete_operation;

pub use delete_operation::*;

mod precalculated_operation;
pub use precalculated_operation::*;

mod read_operation;
pub use read_operation::*;

mod write_operation;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use write_operation::*;

mod signature_verification_operation;
pub use signature_verification_operation::*;

pub const STORAGE_CREDIT_PER_BYTE: i64 = 5000;
pub const STORAGE_PROCESSING_CREDIT_PER_BYTE: i64 = 5000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Operation {
    Read(ReadOperation),
    Write(WriteOperation),
    Delete(DeleteOperation),
    PreCalculated(PreCalculatedOperation),
    SignatureVerification(SignatureVerificationOperation),
}

pub trait OperationLike {
    /// Get CPU cost of the operation
    fn get_processing_cost(&self) -> i64;
    /// Get storage cost of the operation
    fn get_storage_cost(&self) -> i64;
}

macro_rules! call_method {
    ($operation_type:expr, $method:ident ) => {
        match $operation_type {
            Operation::Read(op) => op.$method(),
            Operation::Write(op) => op.$method(),
            Operation::Delete(op) => op.$method(),
            Operation::PreCalculated(op) => op.$method(),
            Operation::SignatureVerification(op) => op.$method(),
        }
    };
}

impl OperationLike for Operation {
    fn get_processing_cost(&self) -> i64 {
        call_method!(self, get_processing_cost)
    }

    fn get_storage_cost(&self) -> i64 {
        call_method!(self, get_storage_cost)
    }
}

impl Operation {
    fn try_from_json_str(from: &str) -> Result<Self, anyhow::Error> {
        let operation = serde_json::from_str(from)?;
        Ok(operation)
    }

    fn try_from_json_value(from: Value) -> Result<Self, anyhow::Error> {
        let operation = serde_json::from_value(from)?;
        Ok(operation)
    }

    fn to_json_value(&self) -> Result<Value, anyhow::Error> {
        let value = serde_json::to_value(self)?;
        Ok(value)
    }

    fn to_json_string(&self) -> Result<String, anyhow::Error> {
        let json_string = serde_json::to_string_pretty(self)?;
        Ok(json_string)
    }
}

#[cfg(test)]
mod test {
    use super::{
        DeleteOperation, Operation, PreCalculatedOperation, ReadOperation,
        SignatureVerificationOperation,
    };
    use crate::{identity::KeyType, state_transition::fee::operations::WriteOperation};
    use serde_json::json;

    struct TestCase {
        json_str: String,
        operation: Operation,
    }

    macro_rules! json_string {
        ($($json:tt)+) => {
            serde_json::to_string_pretty(&json!($($json)+)).unwrap()
        };
    }

    #[test]
    fn test_deserialize_json_read_operation() {
        let cases = vec![
            TestCase {
                json_str: json_string!({
                    "type": "read",
                    "valueSize" : 123,
                }),
                operation: Operation::Read(ReadOperation { value_size: 123 }),
            },
            TestCase {
                json_str: json_string!({
                    "type": "write",
                    "keySize" : 65,
                    "valueSize" : 321,
                }),
                operation: Operation::Write(WriteOperation {
                    key_size: 65,
                    value_size: 321,
                }),
            },
            TestCase {
                json_str: json_string!({
                    "type": "delete",
                    "keySize" : 65,
                    "valueSize" : 123,
                }),
                operation: Operation::Delete(DeleteOperation {
                    key_size: 65,
                    value_size: 123,
                }),
            },
            TestCase {
                json_str: json_string!({
                    "type": "preCalculated",
                    "storageCost" : 12357,
                    "processingCost" : 468910,
                }),
                operation: Operation::PreCalculated(PreCalculatedOperation {
                    storage_cost: 12357,
                    processing_cost: 468910,
                }),
            },
            TestCase {
                json_str: json_string!({
                    "type": "signatureVerification",
                    "signatureType" : 1,

                }),
                operation: Operation::SignatureVerification(SignatureVerificationOperation {
                    signature_type: KeyType::BLS12_381,
                }),
            },
        ];

        for case in cases {
            let operation = Operation::try_from_json_str(&case.json_str)
                .unwrap_or_else(|e| panic!("failed deserializing: {}: {}", case.json_str, e));
            assert_eq!(
                case.operation, operation,
                "failed deserializing: {}",
                case.json_str
            );
            assert_eq!(
                case.json_str,
                operation
                    .to_json_string()
                    .unwrap_or_else(|e| panic!("failed serializing: {}: {}", case.json_str, e)),
                "failed serializing: {}",
                case.json_str
            );
        }
    }
}
