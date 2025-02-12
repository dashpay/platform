use crate::drive::platform_state::REDUCED_PLATFORM_STATE_KEY;
use crate::drive::system::misc_path;
use crate::drive::Drive;
use crate::error::Error;
use grovedb::{Element, TransactionArg};

impl Drive {
    pub(super) fn store_reduced_platform_state_bytes_v0(
        &self,
        reduced_state_bytes: &[u8],
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        self.grove
            .insert_if_not_exists(
                &misc_path(),
                REDUCED_PLATFORM_STATE_KEY,
                Element::Item(reduced_state_bytes.to_vec(), None),
                transaction,
                &Default::default(),
            )
            .unwrap()
            .map_err(Error::GroveDB)?;
        Ok(())
    }
}
