#[cfg(feature = "full")]
use crate::drive::balances::balance_path;
#[cfg(any(feature = "full", feature = "verify"))]
use crate::drive::balances::balance_path_vec;
#[cfg(feature = "full")]
use crate::drive::grove_operations::DirectQueryType;
#[cfg(feature = "full")]
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
#[cfg(feature = "full")]
use crate::drive::identity::{identity_path, IdentityRootStructure};
#[cfg(any(feature = "full", feature = "verify"))]
use crate::drive::Drive;
#[cfg(feature = "full")]
use crate::error::drive::DriveError;
#[cfg(feature = "full")]
use crate::error::Error;
#[cfg(feature = "full")]
use crate::fee::calculate_fee;
#[cfg(feature = "full")]
use crate::fee::credits::{Creditable, Credits, SignedCredits};
#[cfg(feature = "full")]
use crate::fee::op::LowLevelDriveOperation;
#[cfg(feature = "full")]
use crate::fee::result::FeeResult;
use crate::query::QueryResultEncoding;
#[cfg(feature = "full")]
use dpp::block::block_info::BlockInfo;
use dpp::platform_value::platform_value;
#[cfg(feature = "full")]
use grovedb::Element::{Item, SumItem};
#[cfg(feature = "full")]
use grovedb::TransactionArg;
#[cfg(any(feature = "full", feature = "verify"))]
use grovedb::{PathQuery, Query, SizedQuery};

impl Drive {
    #[cfg(feature = "full")]
    /// Fetches the Identity's balance from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_balance(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Option<Credits>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_balance_operations(
            identity_id,
            true,
            transaction,
            &mut drive_operations,
        )
    }

    #[cfg(feature = "full")]
    /// Fetches the Identity's balance from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_serialized_identity_balance(
        &self,
        identity_id: [u8; 32],
        encoding: QueryResultEncoding,
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let balance = self.fetch_identity_balance(identity_id, transaction)?;
        let value = platform_value!({ "balance": balance });
        encoding.encode_value(&value)
    }

    #[cfg(feature = "full")]
    /// Fetches the Identity's balance from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_serialized_identity_balance_and_revision(
        &self,
        identity_id: [u8; 32],
        encoding: QueryResultEncoding,
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let balance = self.fetch_identity_balance(identity_id, transaction)?;

        let revision = self.fetch_identity_revision(identity_id, true, transaction)?;
        let value = platform_value!({
            "balance" : balance,
            "revision" : revision,
        });
        encoding.encode_value(&value)
    }

    #[cfg(feature = "full")]
    /// Fetches the Identity's balance from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_balance_with_costs(
        &self,
        identity_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<(Option<Credits>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_identity_balance_operations(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok((value, fees))
    }

    #[cfg(feature = "full")]
    /// Fetches the Identity's balance from the backing store
    /// If the balance is 0, then also provide debt
    pub fn fetch_identity_balance_include_debt(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Option<SignedCredits>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_balance_include_debt_operations(
            identity_id,
            true,
            transaction,
            &mut drive_operations,
        )
    }

    #[cfg(feature = "full")]
    /// Fetches the Identity's balance from the backing store
    /// If the balance is 0, then also provide debt
    pub fn fetch_identity_balance_include_debt_with_costs(
        &self,
        identity_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<(Option<SignedCredits>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_identity_balance_include_debt_operations(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok((value, fees))
    }

    #[cfg(feature = "full")]
    /// Fetches the Identity's balance from the backing store
    /// If the balance is 0, then also provide debt
    pub(crate) fn fetch_identity_balance_include_debt_operations(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Option<SignedCredits>, Error> {
        Ok(self
            .fetch_identity_balance_operations(identity_id, apply, transaction, drive_operations)?
            .map(|credits| {
                if credits > 0 {
                    Ok::<Option<SignedCredits>, Error>(Some(credits.to_signed()?))
                } else {
                    self.fetch_identity_negative_balance_operations(
                        identity_id,
                        apply,
                        transaction,
                        drive_operations,
                    )
                    .map(|negative_credits| {
                        let negative_credits = negative_credits.ok_or(Error::Drive(
                            DriveError::CorruptedDriveState(
                                "Identity has balance but no negative credit holder".to_string(),
                            ),
                        ))?;
                        Ok(Some(-negative_credits.to_signed()?))
                    })?
                }
            })
            .transpose()?
            .flatten())
    }

    #[cfg(feature = "full")]
    pub(crate) fn fetch_identity_negative_balance_operations(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
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

    #[cfg(any(feature = "full", feature = "verify"))]
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

    #[cfg(feature = "full")]
    /// Creates the operations to get Identity's balance from the backing store
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn fetch_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
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
