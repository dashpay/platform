//! The state of an identity contender vote poll: its stored info, its contenders with their
//! tallies, and its abstain tally, from one path query over the poll's tree.

use crate::drive::votes::paths::{
    vote_identity_contender_poll_tree_path_vec, IDENTITY_CONTENDER_INFO_KEY,
    RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, RESOURCE_STORED_INFO_KEY_U8_32, VOTING_STORAGE_TREE_KEY,
};
use crate::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderWithTally;
#[cfg(feature = "server")]
use crate::drive::Drive;
use crate::error::drive::DriveError;
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
use dpp::serialization::PlatformDeserializableTrusted;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::voting::contender_structs::IdentityContenderInfo;
use dpp::voting::vote_info_storage::identity_contender_vote_poll_stored_info::IdentityContenderVotePollStoredInfo;
#[cfg(feature = "server")]
use grovedb::query_result_type::QueryResultType;
#[cfg(feature = "server")]
use grovedb::TransactionArg;
use grovedb::{Element, PathQuery, QueryItem, SizedQuery};
#[cfg(feature = "server")]
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

/// A read of the state of an identity contender vote poll, keyed by the poll's unique id.
#[derive(Debug, PartialEq, Clone, Encode, Decode)]
pub struct IdentityContenderVotePollStateQuery {
    /// The unique id of the poll: the double sha256 of the serialized vote poll
    pub vote_poll_id: Identifier,
    /// At most this many contenders
    pub limit: Option<u16>,
    /// The contender to start at, and whether it is included. When set, the stored info and
    /// the abstain tally are not read.
    pub start_at: Option<([u8; 32], bool)>,
}

/// The state of an identity contender vote poll as one read returns it.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct IdentityContenderVotePollState {
    /// The poll's stored info: None when the poll never opened, or when the read started at a
    /// contender and did not ask for it
    pub stored_info: Option<IdentityContenderVotePollStoredInfo>,
    /// The contenders in identity id order, each with its tally
    pub contenders: Vec<IdentityContenderWithTally>,
    /// The tally of abstain votes: None once the poll ended, or when the read did not ask for it
    pub abstain_vote_tally: Option<u32>,
}

impl IdentityContenderVotePollStateQuery {
    /// The path query: the stored info item and the abstain tally at the poll's level, and
    /// under each contender its record and its tally. Two elements per contender, so the
    /// limit doubles, plus one for each of the poll's own two keys the range covers.
    pub fn construct_path_query(&self) -> PathQuery {
        let path = vote_identity_contender_poll_tree_path_vec(self.vote_poll_id.as_slice());
        let mut query = Query::new_with_direction(true);
        let poll_keys = [
            RESOURCE_STORED_INFO_KEY_U8_32,
            RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32,
        ];
        let poll_keys_in_range = match &self.start_at {
            None => {
                query.insert_all();
                poll_keys.len()
            }
            Some((start_at_key, start_at_included)) => {
                let start_at_key = start_at_key.to_vec();
                let in_range = poll_keys
                    .iter()
                    .filter(|key| {
                        if *start_at_included {
                            key.as_slice() >= start_at_key.as_slice()
                        } else {
                            key.as_slice() > start_at_key.as_slice()
                        }
                    })
                    .count();
                match start_at_included {
                    true => query.insert_range_from(start_at_key..),
                    false => query.insert_range_after(start_at_key..),
                }
                in_range
            }
        };
        // The poll's own keys are read the same way whatever the range: the stored info as
        // the item it is, the abstain tree as its tally
        query.add_conditional_subquery(
            QueryItem::Key(RESOURCE_STORED_INFO_KEY_U8_32.to_vec()),
            None,
            None,
        );
        query.add_conditional_subquery(
            QueryItem::Key(RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.to_vec()),
            Some(vec![vec![VOTING_STORAGE_TREE_KEY]]),
            None,
        );
        let limit = self.limit.map(|limit| {
            limit
                .saturating_mul(2)
                .saturating_add(poll_keys_in_range as u16)
        });
        let mut contender_query = Query::new();
        contender_query.insert_keys(vec![
            vec![IDENTITY_CONTENDER_INFO_KEY],
            vec![VOTING_STORAGE_TREE_KEY],
        ]);
        query.default_subquery_branch.subquery = Some(contender_query.into());
        PathQuery {
            path,
            query: SizedQuery {
                query,
                limit,
                offset: None,
            },
        }
    }

    /// The state from the elements a read or a proof returned. A verified proof may hold
    /// absent elements, which say nothing.
    pub(crate) fn state_from_elements<I>(
        &self,
        elements: I,
        trusted: bool,
    ) -> Result<IdentityContenderVotePollState, Error>
    where
        I: IntoIterator<Item = (Vec<Vec<u8>>, Vec<u8>, Option<Element>)>,
    {
        let poll_path = vote_identity_contender_poll_tree_path_vec(self.vote_poll_id.as_slice());
        let mut stored_info = None;
        let mut abstain_vote_tally = None;
        let mut contenders: BTreeMap<Identifier, (Option<IdentityContenderInfo>, Option<u32>)> =
            BTreeMap::new();
        for (path, key, element) in elements {
            let Some(element) = element else {
                continue;
            };
            if path == poll_path {
                if key.as_slice() != RESOURCE_STORED_INFO_KEY_U8_32.as_slice() {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                        "the only element at the level of an identity contender vote poll should be its stored info".to_string(),
                    )));
                }
                let bytes = element.into_item_bytes()?;
                stored_info = Some(Self::decode_stored_info(&bytes, trusted)?);
                continue;
            }
            let Some(choice_key) = path.last() else {
                return Err(Error::Drive(DriveError::CorruptedDriveState(
                    "the path must have a last element".to_string(),
                )));
            };
            if choice_key.as_slice() == RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.as_slice() {
                abstain_vote_tally = Some(Self::tally_of(&element)?);
                continue;
            }
            let identity_id = Identifier::from_vec(choice_key.clone())?;
            let entry = contenders.entry(identity_id).or_default();
            if key.as_slice() == [IDENTITY_CONTENDER_INFO_KEY] {
                let bytes = element.into_item_bytes()?;
                entry.0 = Some(Self::decode_contender_info(&bytes, trusted)?);
            } else if key.as_slice() == [VOTING_STORAGE_TREE_KEY] {
                entry.1 = Some(Self::tally_of(&element)?);
            } else {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "unexpected key {} under a contender of an identity contender vote poll",
                    hex::encode(key)
                ))));
            }
        }
        let contenders = contenders
            .into_iter()
            .map(|(identity_id, (info, vote_tally))| {
                let info = info.ok_or(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "contender {} of an identity contender vote poll has no record",
                    identity_id
                ))))?;
                Ok(IdentityContenderWithTally {
                    identity_id,
                    info,
                    vote_tally,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(IdentityContenderVotePollState {
            stored_info,
            contenders,
            abstain_vote_tally,
        })
    }

    /// A node reads its own state without the untrusted limits; a verifier always applies them.
    #[cfg(feature = "server")]
    fn decode_stored_info(
        bytes: &[u8],
        trusted: bool,
    ) -> Result<IdentityContenderVotePollStoredInfo, Error> {
        Ok(if trusted {
            IdentityContenderVotePollStoredInfo::deserialize_from_bytes_trusted(bytes)?
        } else {
            IdentityContenderVotePollStoredInfo::deserialize_from_bytes_untrusted(bytes)?
        })
    }

    #[cfg(not(feature = "server"))]
    fn decode_stored_info(
        bytes: &[u8],
        _trusted: bool,
    ) -> Result<IdentityContenderVotePollStoredInfo, Error> {
        Ok(IdentityContenderVotePollStoredInfo::deserialize_from_bytes_untrusted(bytes)?)
    }

    #[cfg(feature = "server")]
    fn decode_contender_info(bytes: &[u8], trusted: bool) -> Result<IdentityContenderInfo, Error> {
        Ok(if trusted {
            IdentityContenderInfo::deserialize_from_bytes_trusted(bytes)?
        } else {
            IdentityContenderInfo::deserialize_from_bytes_untrusted(bytes)?
        })
    }

    #[cfg(not(feature = "server"))]
    fn decode_contender_info(bytes: &[u8], _trusted: bool) -> Result<IdentityContenderInfo, Error> {
        Ok(IdentityContenderInfo::deserialize_from_bytes_untrusted(
            bytes,
        )?)
    }

    fn tally_of(element: &Element) -> Result<u32, Error> {
        match element {
            Element::SumTree(_, sum_tree_value, _) => {
                if *sum_tree_value < 0 || *sum_tree_value > u32::MAX as i64 {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                        "sum tree value for vote tally must be between 0 and u32::Max, received {} from state",
                        sum_tree_value
                    ))));
                }
                Ok(*sum_tree_value as u32)
            }
            _ => Err(Error::Drive(DriveError::CorruptedDriveState(
                "the votes of a choice of an identity contender vote poll should be a sum tree"
                    .to_string(),
            ))),
        }
    }

    /// Executes the read and returns the proof and its processing cost.
    #[cfg(feature = "server")]
    pub fn execute_with_proof(
        &self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, u64), Error> {
        let mut drive_operations = vec![];
        let path_query = self.construct_path_query();
        let proof = drive.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
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
        Ok((proof, cost))
    }

    /// Executes the read. A poll that never opened, or a branch that no poll created yet, reads
    /// as an empty state.
    #[cfg(feature = "server")]
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<IdentityContenderVotePollState, Error> {
        let path_query = self.construct_path_query();
        match drive.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            drive_operations,
            &platform_version.drive,
        ) {
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    GroveError::PathKeyNotFound(_)
                        | GroveError::PathNotFound(_)
                        | GroveError::PathParentLayerNotFound(_)
                ) =>
            {
                Ok(IdentityContenderVotePollState::default())
            }
            Err(e) => Err(e),
            Ok((query_result_elements, _)) => self.state_from_elements(
                query_result_elements
                    .to_path_key_elements()
                    .into_iter()
                    .map(|(path, key, element)| (path, key, Some(element))),
                true,
            ),
        }
    }
}
