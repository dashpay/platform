use dpp::balances::credits::Creditable;
use grovedb::{Element, TransactionArg};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;
use dpp::fee::Credits;

use crate::fee_pools::epochs::epoch_key_constants;
use crate::fee_pools::epochs::paths::EpochProposers;

impl Drive {
    /// Gets the amount of processing fees to be distributed for the Epoch.
    pub(super) fn get_epoch_processing_credits_for_distribution_v0(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<Credits, Error> {
        let element = self
            .grove
            .get(
                &epoch_tree.get_path(),
                epoch_key_constants::KEY_POOL_PROCESSING_FEES.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        let Element::SumItem(credits, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType(
                "epochs processing fee must be an item",
            )));
        };

        Ok(credits.to_unsigned())
    }
}

#[cfg(test)]
mod tests {
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::fee_pools::epochs::epoch_key_constants;
    use crate::fee_pools::epochs::paths::EpochProposers;
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::epoch::Epoch;
    use grovedb::Element;

    #[test]
    fn test_error_if_value_has_wrong_element_type() {
        let drive = setup_drive_with_initial_state_structure();
        let transaction = drive.grove.start_transaction();

        let epoch = Epoch::new(0).unwrap();

        drive
            .grove
            .insert(
                &epoch.get_path(),
                epoch_key_constants::KEY_POOL_PROCESSING_FEES.as_slice(),
                Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                None,
                Some(&transaction),
            )
            .unwrap()
            .expect("should insert invalid data");

        let result =
            drive.get_epoch_processing_credits_for_distribution_v0(&epoch, Some(&transaction));

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::UnexpectedElementType(_)))
        ));
    }
}
