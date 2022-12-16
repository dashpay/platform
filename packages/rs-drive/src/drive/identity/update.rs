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
    pub fn add_to_identity_balance(
        &self,
        identity_id: [u8; 32],
        added_balance: u64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let previous_balance =
            self.fetch_identity_balance(identity_id, apply, transaction, drive_operations)?;

        let new_balance = previous_balance
            .checked_add(added_balance)
            .ok_or(Error::Identity(IdentityError::BalanceOverflow(
                "identity overflow error",
            )))?;
        drive_operations.push(self.set_identity_balance_operation(identity_id, new_balance)?);
        Ok(())
    }

    /// Balances are stored in the identity under key 0
    pub fn remove_from_identity_balance(
        &self,
        identity_id: [u8; 32],
        required_removed_balance: u64,
        total_desired_removed_balance: u64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let previous_balance =
            self.fetch_identity_balance(identity_id, apply, transaction, drive_operations)?;

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
    use dpp::identity::Identity;
    use grovedb::Element;
    use tempfile::TempDir;

    use crate::drive::flags::StorageFlags;
    use crate::drive::Drive;

    #[test]
    fn test_insert_identity() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let identity_bytes = hex::decode("01000000a462696458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac6762616c616e636500687265766973696f6e006a7075626c69634b65797381a6626964006464617461582102abb64674c5df796559eb3cf92a84525cc1a6068e7ad9d4ff48a1f0b179ae29e164747970650067707572706f73650068726561644f6e6c79f46d73656375726974794c6576656c00").expect("expected to decode identity hex");

        let identity = Identity::from_cbor(identity_bytes.as_slice())
            .expect("expected to deserialize an identity");

        let storage_flags = StorageFlags::SingleEpoch(0);

        drive
            .insert_identity(
                &identity.id,
                Element::Item(identity_bytes, Some(storage_flags.to_element_flags())),
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert identity");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("expected to be able to commit a transaction");
    }
}
