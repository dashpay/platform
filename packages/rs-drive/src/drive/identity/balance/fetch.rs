use crate::drive::balances::{balance_path, balance_path_vec};
use crate::drive::block_info::BlockInfo;
use crate::drive::grove_operations::DirectQueryType;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::identity::{identity_path, IdentityRootStructure};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::credits::Credits;
use crate::fee::op::DriveOperation;
use crate::fee::result::FeeResult;
use grovedb::Element::{Item, SumItem};
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    /// Fetches the Identity's balance from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_balance(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<Option<Credits>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.fetch_identity_balance_operations(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
        )
    }

    /// Fetches the Identity's balance from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_balance_with_fees(
        &self,
        identity_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<(Option<Credits>, FeeResult), Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let value = self.fetch_identity_balance_operations(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok((value, fees))
    }

    pub(crate) fn fetch_identity_negative_balance_operations(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Option<Credits>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            // 8 is the size of a encoded u64
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: true,
                query_target: QueryTargetValue(8),
            }
        };

        let identity_path = identity_path(identity_id.as_slice());

        match self.grove_get_raw(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeNegativeCredit),
            direct_query_type,
            transaction,
            drive_operations,
        ) {
            Ok(Some(Item(encoded_balance, _))) => {
                let balance = Credits::from_be_bytes(
                    encoded_balance.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "negative balance must be u64",
                        ))
                    })?,
                );

                Ok(Some(balance as Credits))
            }

            Ok(None) | Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_))) => {
                if apply {
                    Ok(None)
                } else {
                    Ok(Some(0))
                }
            }

            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity balance was present but was not identified as an item",
            ))),

            Err(e) => Err(e),
        }
    }

    /// The query for the identity balance
    pub fn identity_balance_query(identity_id: &[u8; 32]) -> PathQuery {
        let balance_path = balance_path_vec();
        let mut query = Query::new();
        query.insert_key(identity_id.to_vec());
        PathQuery {
            path: balance_path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        }
    }

    /// Creates the operations to get Identity's balance from the backing store
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn fetch_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Option<Credits>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            // 8 is the size of a i64 used in sum trees
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: true,
                query_target: QueryTargetValue(8),
            }
        };

        let balance_path = balance_path();

        match self.grove_get_raw(
            balance_path,
            identity_id.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
        ) {
            Ok(Some(SumItem(balance, _))) if balance >= 0 => Ok(Some(balance as Credits)),

            Ok(None) | Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_))) => {
                if apply {
                    Ok(None)
                } else {
                    Ok(Some(0))
                }
            }

            Ok(Some(SumItem(..))) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity balance was present but was negative",
            ))),

            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity balance was present but was not identified as a sum item",
            ))),

            Err(e) => Err(e),
        }
    }
}
