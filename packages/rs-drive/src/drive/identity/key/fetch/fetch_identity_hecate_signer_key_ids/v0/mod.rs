use crate::drive::identity::identity_query_keys_tree_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::{KeyID, Purpose};
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery, TransactionArg};
use integer_encoding::VarInt;

impl Drive {
    /// Reads the key ids of the signer index without following its references: the index
    /// sits under the identity's key references, keyed by key id. The query starts at the key
    /// references tree, which every identity has, so an identity without a signer index reads
    /// no entries rather than a missing path.
    pub(super) fn fetch_identity_hecate_signer_key_ids_operations_v0(
        &self,
        identity_id: [u8; 32],
        limit: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<KeyID>, Error> {
        let mut query =
            Query::new_single_query_item(QueryItem::Key(vec![Purpose::HECATE_SIGNER as u8]));
        query.set_subquery(Query::new_range_full());
        let path_query = PathQuery::new(
            identity_query_keys_tree_path_vec(identity_id),
            SizedQuery::new(query, Some(limit), None),
        );

        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            drive_operations,
            &platform_version.drive,
        )?;

        results
            .to_keys()
            .into_iter()
            .map(|key| {
                KeyID::decode_var(key.as_slice())
                    .filter(|(_, read)| *read == key.len())
                    .map(|(key_id, _)| key_id)
                    .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                        "a Hecate signer index entry is not keyed by a key id".to_string(),
                    )))
            })
            .collect()
    }
}
