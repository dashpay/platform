use crate::drive::defaults::CONTRACT_MAX_SERIALIZED_SIZE;
use crate::drive::identity::{
    balance_from_bytes, identity_path, identity_path_vec, IdentityRootStructure,
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
        element_flags: Option<ElementFlags>,
    ) -> DriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());
        let new_balance_bytes = balance.to_be_bytes().to_vec();
        DriveOperation::for_path_key_element(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeBalance).to_vec(),
            Element::new_item_with_flags(new_balance_bytes, element_flags),
        )
    }

    /// We can set an identities negative credit balance
    pub(super) fn set_identity_negative_credit_operation(
        &self,
        identity_id: [u8; 32],
        negative_credit: u64,
        element_flags: Option<ElementFlags>,
    ) -> DriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());
        let new_negative_credit_bytes = negative_credit.to_be_bytes().to_vec();
        DriveOperation::for_path_key_element(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeNegativeCredit).to_vec(),
            Element::new_item_with_flags(new_negative_credit_bytes, element_flags),
        )
    }

    /// Update the revision of the identity
    /// Revisions get bumped on all changes except for the balance and negative credit fields
    pub(super) fn set_revision_operation(
        &self,
        identity_id: [u8; 32],
        revision: u64,
        element_flags: ElementFlags,
    ) -> DriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());
        let revision_bytes = revision.to_be_bytes().to_vec();
        DriveOperation::for_path_key_element(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeRevision).to_vec(),
            Element::new_item_with_flags(revision_bytes, Some(element_flags)),
        )
    }

    /// Balances are stored in the identity under key 0
    pub fn add_to_identity_balance(
        &self,
        identity_id: [u8; 32],
        added_balance: u64,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        //todo ref sizes?
        let query_state_less_max_value_size = if apply {
            None
        } else {
            Some((CONTRACT_MAX_SERIALIZED_SIZE, vec![0]))
        };

        let identity_balance_element = self.grove_get(
            identity_path(identity_id.as_slice()),
            KeyRefRequest(&[0]),
            query_state_less_max_value_size,
            transaction,
            drive_operations,
        )?;
        if let Some(identity_balance_element) = identity_balance_element {
            if let Item(identity_balance_element, element_flags) = identity_balance_element {
                let balance = balance_from_bytes(identity_balance_element.as_slice())?;
                balance.checked_add(added_balance).ok_or(Error::Identity(
                    IdentityError::BalanceOverflow("identity overflow error"),
                ))?;
                drive_operations.push(self.set_identity_balance_operation(
                    identity_id,
                    new_balance,
                    element_flags,
                ));
                Ok(())
            } else {
                Err(Error::Drive(DriveError::CorruptedElementType(
                    "identity balance was present but was not identified as an item",
                )))
            }
        } else {
            Err(Error::Identity(IdentityError::IdentityNotFound(
                "identity not found while trying to modify an identity balance",
            )))
        }
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
        //todo ref sizes?
        let query_state_less_max_value_size = if apply {
            None
        } else {
            Some((CONTRACT_MAX_SERIALIZED_SIZE, vec![0]))
        };

        let identity_balance_element = self.grove_get(
            identity_path(identity_id.as_slice()),
            KeyRefRequest(&[0]),
            query_state_less_max_value_size,
            transaction,
            drive_operations,
        )?;

        if let Some(identity_balance_element) = identity_balance_element {
            if let Item(identity_balance_element, element_flags) = identity_balance_element {
                let balance = balance_from_bytes(identity_balance_element.as_slice())?;
                let (new_balance, negative_credit_amount) = if total_desired_removed_balance
                    > balance
                {
                    // we do not have enough balance
                    // there is a part we absolutely need to pay for
                    if required_removed_balance > balance {
                        return Err(Error::Identity(IdentityError::IdentityInsufficientBalance(
                            "identity does not have the required balance",
                        )));
                    }
                    (0, Some(total_desired_removed_balance - balance))
                } else {
                    // we have enough balance
                    (balance - total_desired_removed_balance, None)
                };
                drive_operations.push(self.set_identity_balance_operation(
                    identity_id,
                    new_balance,
                    element_flags.clone(),
                ));
                if let Some(negative_credit_amount) = negative_credit_amount {
                    drive_operations.push(self.set_identity_negative_credit_operation(
                        identity_id,
                        negative_credit_amount,
                        element_flags,
                    ));
                }
                Ok(())
            } else {
                Err(Error::Drive(DriveError::CorruptedElementType(
                    "identity balance was present but was not identified as an item",
                )))
            }
        } else {
            Err(Error::Identity(IdentityError::IdentityNotFound(
                "identity not found while trying to modify an identity balance",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use grovedb::Element;
    use tempfile::TempDir;

    use crate::drive::flags::StorageFlags;
    use crate::drive::Drive;
    use crate::identity::Identity;

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
