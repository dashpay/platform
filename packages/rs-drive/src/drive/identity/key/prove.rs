use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::Drive;
use crate::error::Error;
use crate::error::Error::GroveDB;
use grovedb::{PathQuery, TransactionArg};

impl Drive {
    pub fn prove_identities_all_keys(
        &self,
        identity_ids: Vec<[u8; 32]>,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let identity_query =
            Self::fetch_identities_all_keys_query(&self, identity_ids, key_request)?;
        self.grove_get_proved_path_query(&identity_query, false, transaction, &mut vec![])
    }
}
