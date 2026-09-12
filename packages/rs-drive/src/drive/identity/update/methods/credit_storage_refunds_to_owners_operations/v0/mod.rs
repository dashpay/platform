use crate::drive::identity::update::storage_refund_credit_outcome::StorageRefundCreditOutcome;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::fee_result::refunds::FeeRefunds;
use dpp::fee::Credits;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::{BTreeMap, HashMap};

impl Drive {
    /// Credits each recorded refund owner that has a balance element and
    /// reports the rest as routed to the processing pool. See the dispatcher.
    #[inline(always)]
    pub(super) fn credit_storage_refunds_to_owners_operations_v0(
        &self,
        fee_refunds: &FeeRefunds,
        skip_owner: Option<[u8; 32]>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<StorageRefundCreditOutcome, Error> {
        let mut credited: BTreeMap<Identifier, Credits> = BTreeMap::new();
        let mut routed_to_processing_pool: Credits = 0;

        for (owner_id, credits_per_epoch) in fee_refunds.iter() {
            if skip_owner.as_ref() == Some(owner_id) {
                continue;
            }

            let credits = credits_per_epoch
                .values()
                .try_fold(0u64, |sum, epoch_credits| sum.checked_add(*epoch_credits))
                .ok_or(ProtocolError::Overflow(
                    "storage refund credits for one owner overflow",
                ))?;

            if credits == 0 {
                continue;
            }

            // A stateful read: `None` means the balance element does not exist, which
            // is the only signal Drive has today that the owner is gone.
            let existing_balance = self.fetch_identity_balance_operations(
                *owner_id,
                true,
                transaction,
                drive_operations,
                platform_version,
            )?;

            if existing_balance.is_some() {
                let mut estimated_costs_only_with_layer_info =
                    None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>;

                drive_operations.extend(self.add_to_identity_balance_operations(
                    *owner_id,
                    credits,
                    &mut estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?);

                credited.insert(Identifier::from(*owner_id), credits);
            } else {
                routed_to_processing_pool = routed_to_processing_pool.checked_add(credits).ok_or(
                    ProtocolError::Overflow(
                        "storage refund credits routed to the processing pool overflow",
                    ),
                )?;
            }
        }

        Ok(StorageRefundCreditOutcome {
            credited,
            routed_to_processing_pool,
        })
    }
}
