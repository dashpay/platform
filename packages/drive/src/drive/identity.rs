use dpp::identity::Identity;
use grovedb::{Element, TransactionArg};
use std::ops::Deref;

use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::KeyInfo;
use crate::drive::object_size_info::PathKeyElementInfo::PathFixedSizeKeyElement;
use crate::drive::storage::batch::Batch;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;

const IDENTITY_KEY: [u8; 1] = [0];

impl Drive {
    pub fn add_insert_identity_operations(
        &self,
        batch: &mut Batch,
        identity: Identity,
    ) -> Result<(), Error> {
        let block_execution_context = self.block_execution_context.borrow();
        let epoch = match block_execution_context.deref() {
            Some(block_execution_context) => block_execution_context.epoch_info.current_epoch_index,
            None => 0,
        };

        let storage_flags = StorageFlags { epoch };

        let identity_bytes = identity.to_buffer().map_err(|_| {
            Error::Identity(IdentityError::IdentitySerialization(
                "failed to serialize identity to CBOR",
            ))
        })?;

        let identities_tree_key = Into::<&[u8; 1]>::into(RootTree::Identities).as_slice();

        batch.insert_empty_tree(
            [identities_tree_key],
            KeyInfo::KeyRef(&identity.id.buffer),
            Some(&storage_flags),
        )?;

        batch.insert(PathFixedSizeKeyElement((
            [identities_tree_key, &identity.id.buffer],
            &IDENTITY_KEY,
            Element::Item(identity_bytes, storage_flags.to_element_flags()),
        )))
    }

    pub fn insert_identity(
        &self,
        identity: Identity,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let mut batch = Batch::new(self);

        self.add_insert_identity_operations(&mut batch, identity)?;

        let mut drive_operations: Vec<DriveOperation> = vec![];

        self.apply_batch_operations(apply, transaction, batch.operations, &mut drive_operations)?;

        calculate_fee(None, Some(drive_operations))
    }

    pub fn fetch_identity(
        &self,
        id: &[u8],
        transaction: TransactionArg,
    ) -> Result<Identity, Error> {
        let element = self
            .grove
            .get(
                [Into::<&[u8; 1]>::into(RootTree::Identities).as_slice(), id],
                &IDENTITY_KEY,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(identity_cbor, _) = element {
            let identity = Identity::from_buffer(identity_cbor.as_slice()).map_err(|_| {
                Error::Identity(IdentityError::IdentitySerialization(
                    "failed to de-serialize identity from CBOR",
                ))
            })?;

            Ok(identity)
        } else {
            Err(Error::Drive(DriveError::CorruptedIdentityNotItem(
                "identity must be an item",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use dpp::identity::Identity;

    use crate::fee::pools::tests::helpers::setup::setup_drive;

    #[test]
    fn test_insert_and_fetch_identity() {
        let drive = setup_drive();

        let transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&transaction))
            .expect("expected to create root tree successfully");

        let identity_bytes = hex::decode("01000000a462696458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac6762616c616e636500687265766973696f6e006a7075626c69634b65797381a6626964006464617461582102abb64674c5df796559eb3cf92a84525cc1a6068e7ad9d4ff48a1f0b179ae29e164747970650067707572706f73650068726561644f6e6c79f46d73656375726974794c6576656c00").expect("expected to decode identity hex");

        let identity = Identity::from_buffer(identity_bytes.as_slice())
            .expect("expected to deserialize an identity");

        drive
            .insert_identity(identity.clone(), true, Some(&transaction))
            .expect("expected to insert identity");

        let fetched_identity = drive
            .fetch_identity(&identity.id.buffer, Some(&transaction))
            .expect("should fetch an identity");

        assert_eq!(
            fetched_identity.to_buffer().expect("should serialize"),
            identity.to_buffer().expect("should serialize")
        );
    }
}
