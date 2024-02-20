use crate::platform::Identifier;
use crate::sdk::LastQueryTimestamp;
use dpp::prelude;
use dpp::prelude::IdentityNonce;
use std::collections::BTreeMap;
use tokio::sync::Mutex;

/// This is a cache that is internal to the SDK that the user does not have to worry about
pub struct InternalSdkCache {
    /// This is the identity nonce counter for the sdk
    /// The sdk will automatically manage this counter for the user.
    /// When the sdk user requests to update identities, withdraw or transfer
    /// this will be automatically updated
    /// This update can involve querying Platform for the current identity nonce
    /// If the sdk user requests to put a state transition the counter is checked and either
    /// returns an error or is updated.
    pub(crate) identity_nonce_counter:
        tokio::sync::Mutex<BTreeMap<Identifier, (prelude::IdentityNonce, LastQueryTimestamp)>>,

    /// This is the identity contract nonce counter for the sdk
    /// The sdk will automatically manage this counter for the user.
    /// When the sdk user requests to put documents this will be automatically updated
    /// This update can involve querying Platform for the current identity contract nonce
    /// If the sdk user requests to put a state transition the counter is checked and either
    /// returns an error or is updated.
    pub(crate) identity_contract_nonce_counter: tokio::sync::Mutex<
        BTreeMap<(Identifier, Identifier), (prelude::IdentityNonce, LastQueryTimestamp)>,
    >,
}

impl Default for InternalSdkCache {
    fn default() -> Self {
        InternalSdkCache {
            identity_nonce_counter: Mutex::new(BTreeMap::<
                Identifier,
                (IdentityNonce, LastQueryTimestamp),
            >::new()),
            identity_contract_nonce_counter: Mutex::new(BTreeMap::<
                (Identifier, Identifier),
                (IdentityNonce, LastQueryTimestamp),
            >::new()),
        }
    }
}
