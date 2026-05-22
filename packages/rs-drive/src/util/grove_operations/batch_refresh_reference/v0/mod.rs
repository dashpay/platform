use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use grovedb::Element;

impl Drive {
    /// Pushes a "refresh reference" operation to `drive_operations`.
    ///
    /// Accepts both reference-shaped elements:
    /// - [`Element::Reference`] → emits a plain `RefreshReference` op.
    /// - [`Element::ReferenceWithSumItem`] → emits a
    ///   `RefreshReference` op in sum-item override mode, so the
    ///   carried sum is rewritten to the value in the supplied element
    ///   and ancestor sum-tree aggregates pick up the delta.
    ///
    /// The summable variant is the path needed by document-update
    /// callers on `summable` indexes when only the summed property's
    /// value changes (no index-key shift): the reference body is
    /// stable, but the sum contribution embedded alongside it must be
    /// rewritten in place. Without this branch, a benign no-op update
    /// to the summed field would error out as
    /// `CorruptedCodeExecution` because the element type doesn't
    /// match the plain-reference shape.
    ///
    /// Any other element variant remains a corruption signal.
    pub(crate) fn batch_refresh_reference_v0(
        &self,
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        document_reference: Element,
        trust_refresh_reference: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        match document_reference {
            Element::Reference(reference_path_type, max_reference_hop, flags) => {
                drive_operations.push(
                    LowLevelDriveOperation::refresh_reference_for_known_path_key_reference_info(
                        path,
                        key,
                        reference_path_type,
                        max_reference_hop,
                        flags,
                        trust_refresh_reference,
                    ),
                );
                Ok(())
            }
            Element::ReferenceWithSumItem(
                reference_path_type,
                max_reference_hop,
                sum_value,
                flags,
            ) => {
                drive_operations.push(
                    LowLevelDriveOperation::refresh_reference_with_sum_item_for_known_path_key_reference_info(
                        path,
                        key,
                        reference_path_type,
                        max_reference_hop,
                        sum_value,
                        flags,
                        trust_refresh_reference,
                    ),
                );
                Ok(())
            }
            _ => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "expected a plain Reference or ReferenceWithSumItem on refresh",
            ))),
        }
    }
}
