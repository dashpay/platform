use crate::drive::platform_state::PlatformStateEntryKind;
use crate::drive::Drive;
use crate::error::Error;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn delete_platform_state_entry_v0(
        &self,
        kind: PlatformStateEntryKind,
        key: &[u8],
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        self.grove
            .delete_aux(kind.entry_key(key), None, transaction)
            .unwrap()
            .map_err(Error::from)
    }
}
