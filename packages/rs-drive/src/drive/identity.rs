use crate::drive::object_size_info::PathKeyElementInfo::PathFixedSizeKeyElement;
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::InsertOperation;
use crate::identity::Identity;
use grovedb::{Element, TransactionArg};

impl Drive {
    fn insert_identity(
        &self,
        identity_key: &[u8],
        identity_bytes: Element,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let mut insert_operations: Vec<InsertOperation> = vec![];
        self.grove_insert(
            PathFixedSizeKeyElement((
                [Into::<&[u8; 1]>::into(RootTree::Identities).as_slice()],
                identity_key,
                identity_bytes,
            )),
            transaction,
            &mut insert_operations,
        )?;
        calculate_fee(None, None, Some(insert_operations), None)
    }

    pub fn insert_identity_cbor(
        &self,
        identity_id: Option<&[u8]>,
        identity_bytes: Vec<u8>,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let identity_id = match identity_id {
            None => {
                let identity = Identity::from_cbor(identity_bytes.as_slice())?;
                Vec::from(identity.id)
            }
            Some(identity_id) => Vec::from(identity_id),
        };

        self.insert_identity(
            identity_id.as_slice(),
            Element::Item(identity_bytes),
            transaction,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::identity::Identity;
    use grovedb::Element;
    use tempfile::TempDir;

    #[test]
    fn test_insert_identity() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_root_tree(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let identity_bytes = hex::decode("01000000a462696458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac6762616c616e636500687265766973696f6e006a7075626c69634b65797381a6626964006464617461582102abb64674c5df796559eb3cf92a84525cc1a6068e7ad9d4ff48a1f0b179ae29e164747970650067707572706f73650068726561644f6e6c79f46d73656375726974794c6576656c00").expect("expected to decode identity hex");

        let identity = Identity::from_cbor(identity_bytes.as_slice())
            .expect("expected to deserialize an identity");

        drive
            .insert_identity(
                &identity.id,
                Element::Item(identity_bytes),
                Some(&db_transaction),
            )
            .expect("expected to insert identity");

        drive
            .grove
            .commit_transaction(db_transaction)
            .expect("expected to be able to commit a transaction");
    }
}
