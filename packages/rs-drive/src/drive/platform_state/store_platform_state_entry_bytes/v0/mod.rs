use crate::drive::platform_state::PlatformStateEntryKind;
use crate::drive::Drive;
use crate::error::Error;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn store_platform_state_entry_bytes_v0(
        &self,
        kind: PlatformStateEntryKind,
        key: &[u8],
        bytes: &[u8],
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        self.grove
            .put_aux(kind.entry_key(key), bytes, None, transaction)
            .unwrap()
            .map_err(Error::from)
    }
}
