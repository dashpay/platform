//! Pricing for the balance work an erase's refunds cause outside its own fee.
//!
//! Refund credits are applied after the fee result is formed, as separate
//! balance operations against identities that had nothing to do with the
//! transition. A bound on how many revisions one erase removes limits that work
//! but does not pay for it, so it is priced here and billed to the submitter
//! through the execution context, which feeds both the admission estimate and
//! the fee actually charged.

use std::collections::HashMap;

use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

/// Beneficiaries an erase can credit beyond the revisions themselves: the
/// lifecycle record's writer, the per-document history subtree's creator, and
/// the current pointer's, all of which a terminal chunk removes.
const STRUCTURAL_REFUND_RECIPIENTS: u64 = 3;

impl Drive {
    /// Prices the worst-case number of third-party balance updates one erase
    /// can cause.
    ///
    /// Every recipient does the same shape of work — read the balance, read the
    /// debt, write one or both — so the cost of one is measured through the
    /// ordinary estimation path and multiplied by how many there can be. The
    /// measurement performs no balance mutation and touches no real identity:
    /// it runs the balance update in estimation mode, where the reads are
    /// average-case costs rather than state.
    pub fn erase_refund_recipient_cost(
        &self,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let chunk = platform_version
            .system_limits
            .max_document_revisions_erased_per_transition
            .ok_or(Error::Drive(DriveError::NotSupported(
                "erasing retained revisions is not available at this protocol version",
            )))?;
        let recipients = chunk as u64 + STRUCTURAL_REFUND_RECIPIENTS;

        let mut layers: Option<HashMap<KeyInfoPath, EstimatedLayerInformation>> =
            Some(HashMap::new());
        let batch = self.add_to_identity_balance_operations(
            [0u8; 32],
            1,
            &mut layers,
            None,
            platform_version,
        )?;
        let mut operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            layers,
            None,
            batch,
            &mut operations,
            &platform_version.drive,
        )?;
        let one = Drive::calculate_fee(
            None,
            Some(operations),
            epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;

        Ok(FeeResult {
            storage_fee: one.storage_fee.saturating_mul(recipients),
            processing_fee: one.processing_fee.saturating_mul(recipients),
            fee_refunds: Default::default(),
            removed_bytes_from_system: 0,
        })
    }
}
