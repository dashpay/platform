use crate::error::fee::FeeError;
use crate::error::Error;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const EPOCH_CHANGE_TIME: i64 = 1576800000;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpochInfo {
    pub current_epoch_index: u16,
    pub is_epoch_change: bool,
}

impl EpochInfo {
    pub fn default() -> EpochInfo {
        EpochInfo {
            current_epoch_index: 0,
            is_epoch_change: true,
        }
    }

    pub fn calculate(
        genesis_time: i64,
        block_time: i64,
        previous_block_time: Option<i64>,
    ) -> Result<EpochInfo, Error> {
        let previous_block_time = match previous_block_time {
            Some(block_time) => block_time,
            None => return Ok(EpochInfo::default()),
        };

        let epoch_change_time = Decimal::from(EPOCH_CHANGE_TIME);
        let block_time = Decimal::from(block_time);
        let genesis_time = Decimal::from(genesis_time);
        let previous_block_time = Decimal::from(previous_block_time);

        let prev_epoch_index = (previous_block_time - genesis_time) / epoch_change_time;
        let prev_epoch_index_floored = prev_epoch_index.floor();

        let epoch_index = (block_time - genesis_time) / epoch_change_time;
        let epoch_index_floored = epoch_index.floor();

        let is_epoch_change = epoch_index_floored > prev_epoch_index_floored;

        let current_epoch_index: u16 = epoch_index_floored.try_into().map_err(|_| {
            Error::Fee(FeeError::DecimalConversion(
                "can't convert epoch index from Decimal to u16",
            ))
        })?;

        Ok(EpochInfo {
            current_epoch_index,
            is_epoch_change,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::fee::epoch::EpochInfo;

    mod calculate {
        #[test]
        fn test_epoch_change_to_0_epoch() {
            let genesis_time: i64 = 1655396517902;
            let block_time: i64 = 1655396517922;

            let epoch_info = super::EpochInfo::calculate(genesis_time, block_time, None)
                .expect("should calculate epoch info");

            assert_eq!(epoch_info.current_epoch_index, 0);
            assert_eq!(epoch_info.is_epoch_change, true);
        }

        #[test]
        fn test_no_epoch_change() {
            let genesis_time: i64 = 1655396517902;
            let block_time: i64 = 1655396517922;
            let prev_block_time: i64 = 1655396517912;

            let epoch_info =
                super::EpochInfo::calculate(genesis_time, block_time, Some(prev_block_time))
                    .expect("should calculate epoch info");

            assert_eq!(epoch_info.current_epoch_index, 0);
            assert_eq!(epoch_info.is_epoch_change, false);
        }

        #[test]
        fn test_epoch_change_to_epoch_1() {
            let genesis_time: i64 = 1655396517902;
            let prev_block_time: i64 = 1655396517912;
            let block_time: i64 = 1657125244561;

            let epoch_info =
                super::EpochInfo::calculate(genesis_time, block_time, Some(prev_block_time))
                    .expect("should calculate epoch info");

            assert_eq!(epoch_info.current_epoch_index, 1);
            assert_eq!(epoch_info.is_epoch_change, true);
        }
    }
}
