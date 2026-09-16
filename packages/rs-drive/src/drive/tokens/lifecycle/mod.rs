//! Per-issuer token lifecycle.
//!
//! The ledger at `[Tokens] / TOKEN_CONTRACT_LIFECYCLES_KEY` holds one
//! [`ContractTokenLifecycle`](dpp::tokens::contract_lifecycle::ContractTokenLifecycle) per
//! contract that issues tokens, keyed by contract id, the destroyed supply scalar at
//! `TOKEN_DESTROYED_SUPPLY_KEY` and the cleanup queue tree at
//! `TOKEN_LIFECYCLE_CLEANUP_QUEUE_KEY`. Every native supply write moves the issuer's rollup
//! in the same batch; destroying an issuer marks its record and moves the rollup into the
//! scalar without visiting a holder or a token.

/// Moves a token's issuer rollup with a supply write.
#[cfg(feature = "server")]
pub mod add_to_contract_issued_supply;
#[cfg(feature = "server")]
mod destroy_token_issuer;
/// Layer estimation for writes under the lifecycle ledger.
#[cfg(feature = "server")]
pub mod estimated_costs;
#[cfg(feature = "server")]
mod fetch_contract_token_lifecycle;
#[cfg(feature = "server")]
mod fetch_token_lifecycles;
#[cfg(feature = "server")]
mod insert_token_contract_lifecycles_structure;
mod queries;
#[cfg(all(feature = "server", any(test, feature = "fixtures-and-mocks")))]
mod test_helpers;

#[cfg(feature = "server")]
use crate::drive::tokens::paths::{
    token_contract_lifecycles_root_path, TOKEN_DESTROYED_SUPPLY_KEY,
};
#[cfg(feature = "server")]
use crate::drive::Drive;
#[cfg(feature = "server")]
use crate::error::drive::DriveError;
#[cfg(feature = "server")]
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use crate::util::grove_operations::DirectQueryType;
#[cfg(feature = "server")]
use dpp::version::drive_versions::DriveVersion;
#[cfg(feature = "server")]
use grovedb::TransactionArg;

/// Byte length of the destroyed supply scalar: a `u128` in big endian.
pub const TOKEN_DESTROYED_SUPPLY_SIZE: usize = 16;

/// Encodes the destroyed supply scalar.
pub fn encode_destroyed_supply(value: u128) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

/// Decodes the destroyed supply scalar, refusing any other length.
pub fn decode_destroyed_supply(bytes: &[u8]) -> Option<u128> {
    let bytes: [u8; TOKEN_DESTROYED_SUPPLY_SIZE] = bytes.try_into().ok()?;
    Some(u128::from_be_bytes(bytes))
}

#[cfg(feature = "server")]
impl Drive {
    /// Reads the destroyed supply scalar, accumulating the cost of the read. `None` when the
    /// ledger does not exist yet (a state below the version that created it).
    pub(crate) fn fetch_token_destroyed_supply_operations(
        &self,
        direct_query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<u128>, Error> {
        let lifecycles_path = token_contract_lifecycles_root_path();
        let bytes = match self.grove_get_raw_optional_item(
            (&lifecycles_path).into(),
            &TOKEN_DESTROYED_SUPPLY_KEY,
            direct_query_type,
            transaction,
            drive_operations,
            drive_version,
        ) {
            Ok(bytes) => bytes,
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_)) => {
                None
            }
            Err(e) => return Err(e),
        };
        bytes
            .map(|bytes| {
                decode_destroyed_supply(&bytes).ok_or_else(|| {
                    Error::Drive(DriveError::CorruptedDriveState(format!(
                        "the destroyed token supply scalar has {} bytes instead of {}",
                        bytes.len(),
                        TOKEN_DESTROYED_SUPPLY_SIZE
                    )))
                })
            })
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_round_trip_the_destroyed_supply_scalar() {
        for value in [0, 1, u64::MAX as u128 + 1, u128::MAX] {
            let bytes = encode_destroyed_supply(value);
            assert_eq!(bytes.len(), TOKEN_DESTROYED_SUPPLY_SIZE);
            assert_eq!(decode_destroyed_supply(&bytes), Some(value));
        }
        assert_eq!(decode_destroyed_supply(&[0; 8]), None);
        assert_eq!(decode_destroyed_supply(&[0; 17]), None);
    }
}
