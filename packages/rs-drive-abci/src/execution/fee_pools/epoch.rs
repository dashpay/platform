use crate::block::BlockInfo;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const EPOCH_CHANGE_TIME_MS: u64 = 1576800000;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpochInfo {
    pub current_epoch_index: u16,
    // Available only on epoch change
    pub previous_epoch_index: Option<u16>,
    pub is_epoch_change: bool,
}

impl EpochInfo {
    pub fn default() -> EpochInfo {
        EpochInfo {
            current_epoch_index: 0,
            previous_epoch_index: None,
            is_epoch_change: true,
        }
    }

    pub fn calculate(
        genesis_time_ms: u64,
        block_time_ms: u64,
        previous_block_time_ms: Option<u64>,
    ) -> Result<Self, Error> {
        let previous_block_time = match previous_block_time_ms {
            Some(block_time) => block_time,
            None => return Ok(EpochInfo::default()),
        };

        let epoch_change_time = Decimal::from(EPOCH_CHANGE_TIME_MS);
        let block_time = Decimal::from(block_time_ms);
        let genesis_time = Decimal::from(genesis_time_ms);
        let previous_block_time = Decimal::from(previous_block_time);

        let previous_epoch_index = (previous_block_time - genesis_time) / epoch_change_time;
        let previous_epoch_index_floored = previous_epoch_index.floor();

        let epoch_index = (block_time - genesis_time) / epoch_change_time;
        let epoch_index_floored = epoch_index.floor();

        let is_epoch_change = epoch_index_floored > previous_epoch_index_floored;

        let current_epoch_index: u16 = epoch_index_floored.try_into().map_err(|_| {
            Error::Execution(ExecutionError::Conversion(
                "can't convert epochs index from Decimal to u16",
            ))
        })?;

        let previous_epoch_index: Option<u16> = if epoch_index_floored
            != previous_epoch_index_floored
        {
            let previous_epoch_index = previous_epoch_index_floored.try_into().map_err(|_| {
                Error::Execution(ExecutionError::Conversion(
                    "can't convert epochs index from Decimal to u16",
                ))
            })?;

            Some(previous_epoch_index)
        } else {
            None
        };

        Ok(Self {
            current_epoch_index,
            previous_epoch_index,
            is_epoch_change,
        })
    }

    pub fn from_genesis_time_and_block_info(
        genesis_time_ms: u64,
        block_info: &BlockInfo,
    ) -> Result<Self, Error> {
        Self::calculate(
            genesis_time_ms,
            block_info.block_time_ms,
            block_info.previous_block_time_ms,
        )
    }
}

#[cfg(test)]
mod test {

    mod calculate {
        use crate::execution::fee_pools::epoch::EpochInfo;

        #[test]
        fn test_epoch_change_to_0_epoch() {
            let genesis_time_ms: u64 = 1655396517902;
            let block_time_ms: u64 = 1655396517922;

            let epoch_info = EpochInfo::calculate(genesis_time_ms, block_time_ms, None)
                .expect("should calculate epochs info");

            assert_eq!(epoch_info.current_epoch_index, 0);
            assert_eq!(epoch_info.is_epoch_change, true);
        }

        #[test]
        fn test_no_epoch_change() {
            let genesis_time_ms: u64 = 1655396517902;
            let block_time_ms: u64 = 1655396517922;
            let prev_block_time_ms: u64 = 1655396517912;

            let epoch_info =
                EpochInfo::calculate(genesis_time_ms, block_time_ms, Some(prev_block_time_ms))
                    .expect("should calculate epochs info");

            assert_eq!(epoch_info.current_epoch_index, 0);
            assert_eq!(epoch_info.is_epoch_change, false);
        }

        #[test]
        fn test_epoch_change_to_epoch_1() {
            let genesis_time_ms: u64 = 1655396517902;
            let prev_block_time_ms: u64 = 1655396517912;
            let block_time_ms: u64 = 1657125244561;

            let epoch_info =
                EpochInfo::calculate(genesis_time_ms, block_time_ms, Some(prev_block_time_ms))
                    .expect("should calculate epochs info");

            assert_eq!(epoch_info.current_epoch_index, 1);
            assert_eq!(epoch_info.is_epoch_change, true);
        }
    }
}
