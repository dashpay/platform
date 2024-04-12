use crate::error::{Error, UnexpectedPatchOperationError};
use json_patch::{AddOperation, PatchOperation, RemoveOperation, ReplaceOperation};

/// This structure represents a change in a JSON schema
/// It contains corresponding [PatchOperation]
#[derive(Debug, PartialEq, Clone)]
pub enum JsonSchemaChange {
    /// Addition of JSON Schema element
    Add(AddOperation),
    /// Removal of JSON Schema element
    Remove(RemoveOperation),
    /// Replacement of JSON Schema element
    Replace(ReplaceOperation),
}

impl JsonSchemaChange {
    /// Returns the name of the operation
    pub fn name(&self) -> &str {
        match self {
            JsonSchemaChange::Add(_) => "add",
            JsonSchemaChange::Remove(_) => "remove",
            JsonSchemaChange::Replace(_) => "replace",
        }
    }

    /// Returns the json path where the operation is applied
    pub fn path(&self) -> &str {
        match self {
            JsonSchemaChange::Add(op) => &op.path,
            JsonSchemaChange::Remove(op) => &op.path,
            JsonSchemaChange::Replace(op) => &op.path,
        }
    }
}

/// Converts a [PatchOperation] into a [JsonSchemaChange]
/// Since [PatchOperation] goes from the [json_patch::diff] function,
/// we don't expect [PatchOperation::Move], [PatchOperation::Copy] and [PatchOperation::Test] operations.
impl TryFrom<PatchOperation> for JsonSchemaChange {
    type Error = Error;

    fn try_from(value: PatchOperation) -> Result<Self, Self::Error> {
        match value {
            PatchOperation::Add(o) => Ok(Self::Add(o)),
            PatchOperation::Remove(o) => Ok(Self::Remove(o)),
            PatchOperation::Replace(o) => Ok(Self::Replace(o)),
            PatchOperation::Move(_) | PatchOperation::Copy(_) | PatchOperation::Test(_) => Err(
                Error::UnexpectedJsonPatchOperation(UnexpectedPatchOperationError(value)),
            ),
        }
    }
}

/// The trait that provides a method to get the path of the [PatchOperation]
pub(crate) trait PatchOperationPath {
    fn path(&self) -> &str;
}

impl PatchOperationPath for PatchOperation {
    fn path(&self) -> &str {
        match self {
            PatchOperation::Add(op) => &op.path,
            PatchOperation::Remove(op) => &op.path,
            PatchOperation::Replace(op) => &op.path,
            PatchOperation::Move(op) => &op.path,
            PatchOperation::Copy(op) => &op.path,
            PatchOperation::Test(op) => &op.path,
        }
    }
}
