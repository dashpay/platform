use crate::drive::votes::paths::vote_contested_resource_identity_votes_tree_path_for_identity_vec;
#[cfg(feature = "server")]
use crate::drive::votes::storage_form::contested_document_resource_reference_storage_form::ContestedDocumentResourceVoteReferenceStorageForm;
#[cfg(feature = "server")]
use crate::drive::votes::storage_form::contested_document_resource_storage_form::ContestedDocumentResourceVoteStorageForm;
#[cfg(feature = "server")]
use crate::drive::votes::tree_path_storage_form::TreePathStorageForm;
#[cfg(feature = "server")]
use crate::drive::Drive;
#[cfg(feature = "server")]
use crate::error::drive::DriveError;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use crate::query::GroveError;
use crate::query::Query;
#[cfg(feature = "server")]
use dpp::bincode;
#[cfg(feature = "server")]
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
#[cfg(feature = "server")]
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
#[cfg(feature = "server")]
use grovedb::TransactionArg;
use grovedb::{PathQuery, SizedQuery};
#[cfg(feature = "server")]
use platform_version::version::PlatformVersion;
#[cfg(feature = "server")]
use std::collections::BTreeMap;

/// Vote Poll Drive Query struct
#[derive(Debug, PartialEq, Clone)]
pub struct ContestedResourceVotesGivenByIdentityQuery {
    /// Which contestant do we want to get the votes for
    pub identity_id: Identifier,
    /// Offset
    pub offset: Option<u16>,
    /// Limit
    pub limit: Option<u16>,
    /// Start at vote id
    pub start_at: Option<([u8; 32], bool)>,
    /// Ascending
    pub order_ascending: bool,
}

impl ContestedResourceVotesGivenByIdentityQuery {
    #[cfg(feature = "server")]
    /// Executes a query with proof and returns the items and fee.
    pub fn execute_with_proof(
        self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, u64), Error> {
        let mut drive_operations = vec![];
        let items = self.execute_with_proof_internal(
            drive,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                drive.config.epochs_per_era,
                platform_version,
                None,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, cost))
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with proof and returns the items.
    pub(crate) fn execute_with_proof_internal(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = self.construct_path_query()?;
        drive.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }

    #[cfg(feature = "server")]
    /// Executes a query with no proof and returns the items, skipped items, and fee.
    pub fn execute_no_proof_with_cost(
        &self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            BTreeMap<Identifier, ContestedDocumentResourceVoteStorageForm>,
            u64,
        ),
        Error,
    > {
        let mut drive_operations = vec![];
        let result =
            self.execute_no_proof(drive, transaction, &mut drive_operations, platform_version)?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                drive.config.epochs_per_era,
                platform_version,
                None,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((result, cost))
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with no proof and returns the values and skipped items.
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, ContestedDocumentResourceVoteStorageForm>, Error> {
        let path_query = self.construct_path_query()?;
        let query_result = drive.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            drive_operations,
            &platform_version.drive,
        );
        match query_result {
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    GroveError::PathKeyNotFound(_)
                        | GroveError::PathNotFound(_)
                        | GroveError::PathParentLayerNotFound(_)
                ) =>
            {
                Ok(BTreeMap::new())
            }
            Err(e) => Err(e),
            Ok((query_result_elements, _)) => {
                let voters =
                    query_result_elements
                        .to_path_key_elements()
                        .into_iter()
                        .map(|(path, key, element)| {
                            let serialized_reference = element.into_item_bytes()?;
                            let bincode_config = bincode::config::standard()
                                .with_big_endian()
                                .with_no_limit();
                            let reference: ContestedDocumentResourceVoteReferenceStorageForm =
                                bincode::decode_from_slice(&serialized_reference, bincode_config)
                                    .map_err(|e| {
                                        Error::Drive(DriveError::CorruptedSerialization(format!(
                                            "serialization of reference {} is corrupted: {}",
                                            hex::encode(serialized_reference),
                                            e
                                        )))
                                    })?
                                    .0;
                            let absolute_path = reference
                                .reference_path_type
                                .absolute_path(path.as_slice(), Some(key.as_slice()))?;
                            let vote_id = Identifier::from_vec(key)?;
                            Ok((
                                vote_id,
                                ContestedDocumentResourceVoteStorageForm::try_from_tree_path(
                                    absolute_path,
                                )?,
                            ))
                        })
                        .collect::<Result<
                            BTreeMap<Identifier, ContestedDocumentResourceVoteStorageForm>,
                            Error,
                        >>()?;

                Ok(voters)
            }
        }
    }

    #[cfg(feature = "server")]
    #[allow(unused)]
    /// Executes an internal query with no proof and returns the values and skipped items.
    pub(crate) fn execute_no_proof_internal(
        &self,
        drive: &Drive,
        result_type: QueryResultType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(QueryResultElements, u16), Error> {
        let path_query = self.construct_path_query()?;
        let query_result = drive.grove_get_path_query(
            &path_query,
            transaction,
            result_type,
            drive_operations,
            &platform_version.drive,
        );
        match query_result {
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    GroveError::PathKeyNotFound(_)
                        | GroveError::PathNotFound(_)
                        | GroveError::PathParentLayerNotFound(_)
                ) =>
            {
                Ok((QueryResultElements::new(), 0))
            }
            _ => {
                let (data, skipped) = query_result?;
                {
                    Ok((data, skipped))
                }
            }
        }
    }
    /// Operations to construct a path query.
    pub fn construct_path_query(&self) -> Result<PathQuery, Error> {
        let path = vote_contested_resource_identity_votes_tree_path_for_identity_vec(
            self.identity_id.as_bytes(),
        );

        let mut query = Query::new_with_direction(self.order_ascending);

        // this is a range on all elements
        match &self.start_at {
            None => {
                query.insert_all();
            }
            Some((starts_at_key_bytes, start_at_included)) => {
                let starts_at_key = starts_at_key_bytes.to_vec();
                match self.order_ascending {
                    true => match start_at_included {
                        true => query.insert_range_from(starts_at_key..),
                        false => query.insert_range_after(starts_at_key..),
                    },
                    false => match start_at_included {
                        true => query.insert_range_to_inclusive(..=starts_at_key),
                        false => query.insert_range_to(..starts_at_key),
                    },
                }
            }
        }

        Ok(PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: self.limit,
                offset: self.offset,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::paths::{CONTESTED_RESOURCE_TREE_KEY, IDENTITY_VOTES_TREE_KEY};
    use crate::drive::RootTree;
    use grovedb::QueryItem;

    fn expected_base_path(identity_id: &[u8; 32]) -> Vec<Vec<u8>> {
        vec![
            vec![RootTree::Votes as u8],
            vec![CONTESTED_RESOURCE_TREE_KEY as u8],
            vec![IDENTITY_VOTES_TREE_KEY as u8],
            identity_id.to_vec(),
        ]
    }

    // -----------------------------------------------------------------------
    // construct_path_query
    // -----------------------------------------------------------------------

    #[test]
    fn construct_path_query_no_start_ascending() {
        let identity_id = Identifier::from([0xAA; 32]);
        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset: None,
            limit: Some(10),
            start_at: None,
            order_ascending: true,
        };

        let pq = query
            .construct_path_query()
            .expect("should build path query");
        assert_eq!(pq.path, expected_base_path(identity_id.as_bytes()));
        assert_eq!(pq.query.limit, Some(10));
        assert_eq!(pq.query.offset, None);
        assert!(pq.query.query.left_to_right);

        // No start_at means insert_all -> RangeFull
        let items = &pq.query.query.items;
        assert_eq!(items.len(), 1);
        assert!(matches!(&items[0], QueryItem::RangeFull(..)));
    }

    #[test]
    fn construct_path_query_no_start_descending() {
        let identity_id = Identifier::from([0xBB; 32]);
        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset: None,
            limit: None,
            start_at: None,
            order_ascending: false,
        };

        let pq = query
            .construct_path_query()
            .expect("should build path query");
        assert!(!pq.query.query.left_to_right);
        assert_eq!(pq.query.limit, None);
    }

    #[test]
    fn construct_path_query_start_at_included_ascending() {
        let identity_id = Identifier::from([0xCC; 32]);
        let start_key = [0x42u8; 32];
        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset: None,
            limit: Some(5),
            start_at: Some((start_key, true)),
            order_ascending: true,
        };

        let pq = query
            .construct_path_query()
            .expect("should build path query");
        let items = &pq.query.query.items;
        assert_eq!(items.len(), 1);
        assert!(
            matches!(&items[0], QueryItem::RangeFrom(r) if r.start == start_key.to_vec()),
            "ascending + included = RangeFrom"
        );
    }

    #[test]
    fn construct_path_query_start_at_excluded_ascending() {
        let identity_id = Identifier::from([0xDD; 32]);
        let start_key = [0x42u8; 32];
        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset: None,
            limit: Some(5),
            start_at: Some((start_key, false)),
            order_ascending: true,
        };

        let pq = query
            .construct_path_query()
            .expect("should build path query");
        let items = &pq.query.query.items;
        assert_eq!(items.len(), 1);
        assert!(
            matches!(&items[0], QueryItem::RangeAfter(r) if r.start == start_key.to_vec()),
            "ascending + excluded = RangeAfter"
        );
    }

    #[test]
    fn construct_path_query_start_at_included_descending() {
        let identity_id = Identifier::from([0xEE; 32]);
        let start_key = [0x42u8; 32];
        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset: None,
            limit: Some(5),
            start_at: Some((start_key, true)),
            order_ascending: false,
        };

        let pq = query
            .construct_path_query()
            .expect("should build path query");
        let items = &pq.query.query.items;
        assert_eq!(items.len(), 1);
        assert!(
            matches!(&items[0], QueryItem::RangeToInclusive(r) if r.end == start_key.to_vec()),
            "descending + included = RangeToInclusive"
        );
    }

    #[test]
    fn construct_path_query_start_at_excluded_descending() {
        let identity_id = Identifier::from([0xFF; 32]);
        let start_key = [0x42u8; 32];
        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset: None,
            limit: Some(5),
            start_at: Some((start_key, false)),
            order_ascending: false,
        };

        let pq = query
            .construct_path_query()
            .expect("should build path query");
        let items = &pq.query.query.items;
        assert_eq!(items.len(), 1);
        assert!(
            matches!(&items[0], QueryItem::RangeTo(r) if r.end == start_key.to_vec()),
            "descending + excluded = RangeTo"
        );
    }

    #[test]
    fn construct_path_query_with_offset_and_limit() {
        let identity_id = Identifier::from([0x11; 32]);
        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset: Some(7),
            limit: Some(25),
            start_at: None,
            order_ascending: true,
        };

        let pq = query
            .construct_path_query()
            .expect("should build path query");
        assert_eq!(pq.query.limit, Some(25));
        assert_eq!(pq.query.offset, Some(7));
    }

    #[test]
    fn construct_path_query_identity_id_appears_in_path() {
        let identity_id = Identifier::from([0x99; 32]);
        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset: None,
            limit: None,
            start_at: None,
            order_ascending: true,
        };

        let pq = query
            .construct_path_query()
            .expect("should build path query");
        // The 4th path element should be the identity_id
        assert_eq!(pq.path.len(), 4);
        assert_eq!(pq.path[3], identity_id.as_bytes().to_vec());
    }
}
