use dpp::identity::Identity;
use grovedb::query_result_type::QueryResultType::QueryElementResultType;
use grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery, TransactionArg};

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::flags::StorageFlags;
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
        identity: Identity,
        storage_flags: StorageFlags,
        batch: &mut GroveDbOpBatch,
    ) -> Result<(), Error> {
        let identity_bytes = identity.to_buffer().map_err(|_| {
            Error::Identity(IdentityError::IdentitySerialization(
                "failed to serialize identity to CBOR",
            ))
        })?;

        batch.add_insert_empty_tree_with_flags(
            vec![vec![RootTree::Identities as u8]],
            identity.id.buffer.to_vec(),
            &storage_flags,
        );

        batch.add_insert(
            vec![
                vec![RootTree::Identities as u8],
                identity.id.buffer.to_vec(),
            ],
            IDENTITY_KEY.to_vec(),
            Element::Item(identity_bytes, storage_flags.to_element_flags()),
        );

        Ok(())
    }

    pub fn insert_identity(
        &self,
        identity: Identity,
        apply: bool,
        storage_flags: StorageFlags,
        transaction: TransactionArg,
    ) -> Result<(i64, u64), Error> {
        let mut batch = GroveDbOpBatch::new();

        self.add_insert_identity_operations(identity, storage_flags, &mut batch)?;

        let mut drive_operations: Vec<DriveOperation> = vec![];

        self.apply_batch_grovedb_operations(apply, transaction, batch, &mut drive_operations)?;

        calculate_fee(None, Some(drive_operations))
    }

    pub fn fetch_identity(
        &self,
        id: &[u8],
        transaction: TransactionArg,
    ) -> Result<(Identity, StorageFlags), Error> {
        let element = self
            .grove
            .get(
                [Into::<&[u8; 1]>::into(RootTree::Identities).as_slice(), id],
                &IDENTITY_KEY,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(identity_cbor, element_flags) = element {
            let identity = Identity::from_buffer(identity_cbor.as_slice()).map_err(|_| {
                Error::Identity(IdentityError::IdentitySerialization(
                    "failed to de-serialize identity from CBOR",
                ))
            })?;

            Ok((identity, StorageFlags::from_element_flags(element_flags)?))
        } else {
            Err(Error::Drive(DriveError::CorruptedIdentityNotItem(
                "identity must be an item",
            )))
        }
    }

    pub fn fetch_identities(
        &self,
        ids: &Vec<[u8; 32]>,
        transaction: TransactionArg,
    ) -> Result<Vec<Identity>, Error> {
        Ok(self
            .fetch_identities_with_flags(ids, transaction)?
            .into_iter()
            .map(|(identity, _)| identity)
            .collect())
    }

    pub fn fetch_identities_with_flags(
        &self,
        ids: &Vec<[u8; 32]>,
        transaction: TransactionArg,
    ) -> Result<Vec<(Identity, StorageFlags)>, Error> {
        let mut query = Query::new();
        query.set_subquery_key(IDENTITY_KEY.to_vec());
        for id in ids {
            query.insert_item(QueryItem::Key(id.to_vec()));
        }
        let path_query = PathQuery {
            path: vec![vec![RootTree::Identities as u8]],
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        };
        let (result_items, _) = self
            .grove
            .query_raw(&path_query, QueryElementResultType, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?;

        result_items
            .to_elements()
            .into_iter()
            .map(|element| {
                if let Element::Item(identity_cbor, element_flags) = element {
                    let identity =
                        Identity::from_buffer(identity_cbor.as_slice()).map_err(|_| {
                            Error::Identity(IdentityError::IdentitySerialization(
                                "failed to de-serialize identity from CBOR",
                            ))
                        })?;

                    Ok((identity, StorageFlags::from_element_flags(element_flags)?))
                } else {
                    Err(Error::Drive(DriveError::CorruptedIdentityNotItem(
                        "identity must be an item",
                    )))
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::setup_drive;
    use crate::drive::flags::StorageFlags;
    use dpp::identity::Identity;

    #[test]
    fn test_insert_and_fetch_identity() {
        let drive = setup_drive(None);

        let transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&transaction))
            .expect("expected to create root tree successfully");

        let identity_bytes = hex::decode("01000000a462696458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac6762616c616e636500687265766973696f6e006a7075626c69634b65797381a6626964006464617461582102abb64674c5df796559eb3cf92a84525cc1a6068e7ad9d4ff48a1f0b179ae29e164747970650067707572706f73650068726561644f6e6c79f46d73656375726974794c6576656c00").expect("expected to decode identity hex");

        let identity = Identity::from_buffer(identity_bytes.as_slice())
            .expect("expected to deserialize an identity");

        drive
            .insert_identity(
                identity.clone(),
                true,
                StorageFlags::default(),
                Some(&transaction),
            )
            .expect("expected to insert identity");

        let (fetched_identity, _) = drive
            .fetch_identity(&identity.id.buffer, Some(&transaction))
            .expect("should fetch an identity");

        assert_eq!(
            fetched_identity.to_buffer().expect("should serialize"),
            identity.to_buffer().expect("should serialize")
        );
    }
}
