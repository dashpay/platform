use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::Drive;
use crate::error::Error;
use crate::error::Error::GroveDB;
use grovedb::PathQuery;

impl Drive {
    pub fn fetch_identities_all_keys_query(
        &self,
        identity_ids: Vec<[u8; 32]>,
        key_request: IdentityKeysRequest,
    ) -> Result<PathQuery, Error> {
        // let allIdentitiesKeys = Self::fetch_identities_all_keys(&self, identity_ids, transaction);

        let mut queries = Vec::new();
        for identity_id in identity_ids {
            queries.push(Self::fetch_identity_keys(&self, key_request.clone(), None));
        }

        PathQuery::merge(queries.iter().collect()).map_err(GroveDB)
    }
}
