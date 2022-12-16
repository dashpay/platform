use crate::drive::block_info::BlockInfo;
use crate::drive::defaults::CONTRACT_MAX_SERIALIZED_SIZE;
use crate::drive::flags::StorageFlags;
use crate::drive::identity::{
    balance_from_bytes, balance_path_vec, identity_path, identity_path_vec, IdentityRootStructure,
};
use crate::drive::object_size_info::KeyValueInfo::KeyRefRequest;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::fee::{calculate_fee, FeeResult};
use grovedb::Element::Item;
use grovedb::{Element, ElementFlags, TransactionArg};

impl Drive {
    /// We can set an identities balance
    pub(super) fn set_identity_balance_operation(
        &self,
        identity_id: [u8; 32],
        balance: u64,
    ) -> Result<DriveOperation, Error> {
        let balance_path = balance_path_vec();
        // while i64::MAX could potentially work, best to avoid it.
        if balance >= i64::MAX as u64 {
            Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set balance to over i64::Max",
            )))
        } else {
            Ok(DriveOperation::for_known_path_key_element(
                balance_path,
                identity_id.to_vec(),
                Element::new_sum_item(balance as i64),
            ))
        }
    }

    /// We can set an identities negative credit balance
    pub(super) fn set_identity_negative_credit_operation(
        &self,
        identity_id: [u8; 32],
        negative_credit: u64,
    ) -> DriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());
        let new_negative_credit_bytes = negative_credit.to_be_bytes().to_vec();
        DriveOperation::for_known_path_key_element(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeNegativeCredit).to_vec(),
            Element::new_item(new_negative_credit_bytes),
        )
    }

    /// Update the revision of the identity
    /// Revisions get bumped on all changes except for the balance and negative credit fields
    pub(super) fn set_revision_operation(
        &self,
        identity_id: [u8; 32],
        revision: u64,
    ) -> DriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());
        let revision_bytes = revision.to_be_bytes().to_vec();
        DriveOperation::for_known_path_key_element(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeRevision).to_vec(),
            Element::new_item(revision_bytes),
        )
    }

    /// Balances are stored in the balance tree under the identity's id
    pub(crate) fn add_to_identity_balance(
        &self,
        identity_id: [u8; 32],
        added_balance: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.add_to_identity_balance_operations(
            identity_id,
            added_balance,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok(fees)
    }

    /// Balances are stored in the balance tree under the identity's id
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn add_to_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        added_balance: u64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let previous_balance = self.fetch_identity_balance_operations(
            identity_id,
            apply,
            transaction,
            drive_operations,
        )?;

        let new_balance = previous_balance
            .checked_add(added_balance)
            .ok_or(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "identity overflow error",
            )))?;
        drive_operations.push(self.set_identity_balance_operation(identity_id, new_balance)?);
        Ok(())
    }

    /// Balances are stored in the balance tree under the identity's id
    pub(crate) fn remove_from_identity_balance(
        &self,
        identity_id: [u8; 32],
        required_removed_balance: u64,
        total_desired_removed_balance: u64,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.remove_from_identity_balance_operations(
            identity_id,
            required_removed_balance,
            total_desired_removed_balance,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok(fees)
    }

    /// Balances are stored in the identity under key 0
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn remove_from_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        required_removed_balance: u64,
        total_desired_removed_balance: u64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let previous_balance = self.fetch_identity_balance_operations(
            identity_id,
            apply,
            transaction,
            drive_operations,
        )?;

        let (new_balance, negative_credit_amount) =
            if total_desired_removed_balance > previous_balance {
                // we do not have enough balance
                // there is a part we absolutely need to pay for
                if required_removed_balance > previous_balance {
                    return Err(Error::Identity(IdentityError::IdentityInsufficientBalance(
                        "identity does not have the required balance",
                    )));
                }
                (0, Some(total_desired_removed_balance - previous_balance))
            } else {
                // we have enough balance
                (previous_balance - total_desired_removed_balance, None)
            };
        drive_operations.push(self.set_identity_balance_operation(identity_id, new_balance)?);
        if let Some(negative_credit_amount) = negative_credit_amount {
            drive_operations.push(
                self.set_identity_negative_credit_operation(identity_id, negative_credit_amount),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::block_info::BlockInfo;
    use dpp::identity::Identity;
    use grovedb::Element;
    use tempfile::TempDir;

    use crate::drive::flags::StorageFlags;
    use crate::drive::Drive;
    use crate::fee_pools::epochs::Epoch;

    #[test]
    fn test_update_identity_balance() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345));

        let old_balance = identity.balance;

        let block = BlockInfo::default_with_epoch(Epoch::new(0));

        drive
            .add_new_identity(identity.clone(), &block, true, None)
            .expect("expected to insert identity");

        let db_transaction = drive.grove.start_transaction();

        drive
            .add_to_identity_balance(
                identity.id.to_buffer(),
                300,
                &block,
                true,
                Some(&db_transaction),
            )
            .expect("expected to add to identity balance");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("expected to be able to commit a transaction");

        let (balance, fee_cost) = drive
            .fetch_identity_balance_with_fees(identity.id.to_buffer(), &block, true, None)
            .expect("expected to get balance");

        assert_eq!(balance, old_balance + 300);
    }
}
