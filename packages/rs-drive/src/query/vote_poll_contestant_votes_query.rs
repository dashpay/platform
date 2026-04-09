#[cfg(feature = "verify")]
use super::ContractLookupFn;
use crate::drive::votes::paths::VotePollPaths;
#[cfg(any(feature = "server", feature = "verify"))]
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
#[cfg(feature = "server")]
use crate::drive::Drive;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use crate::query::GroveError;
use crate::query::Query;
use bincode::{Decode, Encode};
#[cfg(feature = "server")]
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
#[cfg(feature = "server")]
use dpp::platform_value;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice::TowardsIdentity;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
#[cfg(feature = "server")]
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
#[cfg(feature = "server")]
use grovedb::TransactionArg;
use grovedb::{PathQuery, SizedQuery};
use platform_version::version::PlatformVersion;

/// Vote Poll Drive Query struct
#[derive(Debug, PartialEq, Clone, Encode, Decode)]
pub struct ContestedDocumentVotePollVotesDriveQuery {
    /// What vote poll are we asking for?
    pub vote_poll: ContestedDocumentResourceVotePoll,
    /// Which contestant do we want to get the votes for
    pub contestant_id: Identifier,
    /// Offset
    pub offset: Option<u16>,
    /// Limit
    pub limit: Option<u16>,
    /// Start at identity id
    pub start_at: Option<([u8; 32], bool)>,
    /// Ascending
    pub order_ascending: bool,
}

impl ContestedDocumentVotePollVotesDriveQuery {
    #[cfg(feature = "server")]
    /// Resolves the contested document vote poll drive query.
    ///
    /// This method processes the query by interacting with the drive, using the provided
    /// transaction and platform version to ensure consistency and compatibility.
    ///
    /// # Parameters
    ///
    /// * `drive`: A reference to the `Drive` object used for database interactions.
    /// * `transaction`: The transaction argument used to ensure consistency during the resolve operation.
    /// * `platform_version`: The platform version to ensure compatibility.
    ///
    /// # Returns
    ///
    /// * `Ok(ResolvedContestedDocumentVotePollDriveQuery)` - The resolved query information.
    /// * `Err(Error)` - An error if the resolution process fails.
    ///
    /// # Errors
    ///
    /// This method returns an `Error` variant if there is an issue resolving the query.
    /// The specific error depends on the underlying problem encountered during resolution.
    pub fn resolve(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedContestedDocumentVotePollVotesDriveQuery<'_>, Error> {
        let ContestedDocumentVotePollVotesDriveQuery {
            vote_poll,
            contestant_id,
            offset,
            limit,
            start_at,
            order_ascending,
        } = self;
        Ok(ResolvedContestedDocumentVotePollVotesDriveQuery {
            vote_poll: vote_poll.resolve_allow_borrowed(drive, transaction, platform_version)?,
            contestant_id: *contestant_id,
            offset: *offset,
            limit: *limit,
            start_at: *start_at,
            order_ascending: *order_ascending,
        })
    }

    /// Resolves the contested document vote poll drive query.
    ///
    /// See [ContestedDocumentVotePollVotesDriveQuery::resolve](ContestedDocumentVotePollVotesDriveQuery::resolve) for more information.
    #[cfg(feature = "verify")]
    pub fn resolve_with_known_contracts_provider<'a>(
        &self,
        known_contracts_provider: &ContractLookupFn,
    ) -> Result<ResolvedContestedDocumentVotePollVotesDriveQuery<'a>, Error> {
        let ContestedDocumentVotePollVotesDriveQuery {
            vote_poll,
            contestant_id,
            offset,
            limit,
            start_at,
            order_ascending,
        } = self;
        Ok(ResolvedContestedDocumentVotePollVotesDriveQuery {
            vote_poll: vote_poll.resolve_with_known_contracts_provider(known_contracts_provider)?,
            contestant_id: *contestant_id,
            offset: *offset,
            limit: *limit,
            start_at: *start_at,
            order_ascending: *order_ascending,
        })
    }

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
        let resolved = self.resolve(drive, transaction, platform_version)?;
        let path_query = resolved.construct_path_query(platform_version)?;
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
    ) -> Result<(Vec<Identifier>, u64), Error> {
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
    ) -> Result<Vec<Identifier>, Error> {
        let resolved = self.resolve(drive, transaction, platform_version)?;
        let path_query = resolved.construct_path_query(platform_version)?;
        let query_result = drive.grove_get_path_query(
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
                Ok(vec![])
            }
            Err(e) => Err(e),
            Ok((query_result_elements, _skipped)) => {
                let voters = query_result_elements
                    .to_keys()
                    .into_iter()
                    .map(Identifier::try_from)
                    .collect::<Result<Vec<Identifier>, platform_value::Error>>()?;

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
        let resolved = self.resolve(drive, transaction, platform_version)?;
        let path_query = resolved.construct_path_query(platform_version)?;
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
}
/// Vote Poll Drive Query struct
#[derive(Debug, PartialEq, Clone)]
pub struct ResolvedContestedDocumentVotePollVotesDriveQuery<'a> {
    /// What vote poll are we asking for?
    pub vote_poll: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed<'a>,
    /// Who's votes are we looking for
    pub contestant_id: Identifier,
    /// Offset
    pub offset: Option<u16>,
    /// Limit
    pub limit: Option<u16>,
    /// Start at identity id, the bool is if it is also included
    pub start_at: Option<([u8; 32], bool)>,
    /// Ascending
    pub order_ascending: bool,
}

impl ResolvedContestedDocumentVotePollVotesDriveQuery<'_> {
    /// Operations to construct a path query.
    pub fn construct_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let path = self
            .vote_poll
            .contender_voting_path(&TowardsIdentity(self.contestant_id), platform_version)?;

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
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
    use crate::util::object_size_info::DataContractResolvedInfo;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use grovedb::QueryItem;

    /// Helper to construct a resolved contestant votes query using the DPNS
    /// "domain" contested index.
    fn build_resolved_query(
        contract: &dpp::data_contract::DataContract,
        contestant_id: Identifier,
        offset: Option<u16>,
        limit: Option<u16>,
        start_at: Option<([u8; 32], bool)>,
        order_ascending: bool,
    ) -> ResolvedContestedDocumentVotePollVotesDriveQuery<'_> {
        let document_type_name = "domain".to_string();
        let index_name = "parentNameAndLabel".to_string();

        let parent_domain_value = dpp::platform_value::Value::Text("dash".to_string());
        let label_value = dpp::platform_value::Value::Text("test-name".to_string());

        let index_values = vec![parent_domain_value, label_value];

        let vote_poll = ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
            contract: DataContractResolvedInfo::BorrowedDataContract(contract),
            document_type_name,
            index_name,
            index_values,
        };

        ResolvedContestedDocumentVotePollVotesDriveQuery {
            vote_poll,
            contestant_id,
            offset,
            limit,
            start_at,
            order_ascending,
        }
    }

    // -----------------------------------------------------------------------
    // construct_path_query tests
    // -----------------------------------------------------------------------

    #[test]
    fn construct_path_query_no_start_ascending() {
        let platform_version = PlatformVersion::latest();
        let dpns = get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let contract = dpns.data_contract_owned();

        let contestant_id = Identifier::from([0xAA; 32]);
        let query = build_resolved_query(
            &contract,
            contestant_id,
            None,     // offset
            Some(10), // limit
            None,     // start_at
            true,     // ascending
        );

        let pq = query
            .construct_path_query(platform_version)
            .expect("should build path query");

        // Path should end with the contestant identifier and voting storage key
        assert!(!pq.path.is_empty());
        assert_eq!(pq.query.limit, Some(10));
        assert_eq!(pq.query.offset, None);
        assert!(pq.query.query.left_to_right);

        // No start -> RangeFull
        let items = &pq.query.query.items;
        assert_eq!(items.len(), 1);
        assert!(matches!(&items[0], QueryItem::RangeFull(..)));
    }

    #[test]
    fn construct_path_query_no_start_descending() {
        let platform_version = PlatformVersion::latest();
        let dpns = get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let contract = dpns.data_contract_owned();

        let contestant_id = Identifier::from([0xBB; 32]);
        let query = build_resolved_query(&contract, contestant_id, None, None, None, false);

        let pq = query
            .construct_path_query(platform_version)
            .expect("should build path query");

        assert!(!pq.query.query.left_to_right);
        assert_eq!(pq.query.limit, None);
    }

    #[test]
    fn construct_path_query_start_at_included_ascending() {
        let platform_version = PlatformVersion::latest();
        let dpns = get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let contract = dpns.data_contract_owned();

        let contestant_id = Identifier::from([0xCC; 32]);
        let start_key = [0x42u8; 32];
        let query = build_resolved_query(
            &contract,
            contestant_id,
            None,
            Some(5),
            Some((start_key, true)),
            true,
        );

        let pq = query
            .construct_path_query(platform_version)
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
        let platform_version = PlatformVersion::latest();
        let dpns = get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let contract = dpns.data_contract_owned();

        let contestant_id = Identifier::from([0xDD; 32]);
        let start_key = [0x42u8; 32];
        let query = build_resolved_query(
            &contract,
            contestant_id,
            None,
            Some(5),
            Some((start_key, false)),
            true,
        );

        let pq = query
            .construct_path_query(platform_version)
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
        let platform_version = PlatformVersion::latest();
        let dpns = get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let contract = dpns.data_contract_owned();

        let contestant_id = Identifier::from([0xEE; 32]);
        let start_key = [0x42u8; 32];
        let query = build_resolved_query(
            &contract,
            contestant_id,
            None,
            Some(5),
            Some((start_key, true)),
            false,
        );

        let pq = query
            .construct_path_query(platform_version)
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
        let platform_version = PlatformVersion::latest();
        let dpns = get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let contract = dpns.data_contract_owned();

        let contestant_id = Identifier::from([0xFF; 32]);
        let start_key = [0x42u8; 32];
        let query = build_resolved_query(
            &contract,
            contestant_id,
            None,
            Some(5),
            Some((start_key, false)),
            false,
        );

        let pq = query
            .construct_path_query(platform_version)
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
        let platform_version = PlatformVersion::latest();
        let dpns = get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let contract = dpns.data_contract_owned();

        let contestant_id = Identifier::from([0x11; 32]);
        let query = build_resolved_query(
            &contract,
            contestant_id,
            Some(3),  // offset
            Some(20), // limit
            None,
            true,
        );

        let pq = query
            .construct_path_query(platform_version)
            .expect("should build path query");

        assert_eq!(pq.query.limit, Some(20));
        assert_eq!(pq.query.offset, Some(3));
    }
}
