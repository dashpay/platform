use std::collections::BTreeMap;
use tokio::sync::Mutex;
use dpp::prelude;
use dpp::prelude::IdentityContractNonce;
use crate::platform::Identifier;
use crate::sdk::LastQueryTimestamp;

mod identity_contract_nonce_counter;

/// This is a cache that is internal to the SDK that the user does not have to worry about
pub struct InternalSdkCache {
    /// This is the identity contract nonce counter for the sdk
    /// The sdk will automatically manage this counter for the user.
    /// When the sdk user requests to put documents this will be automatically updated
    /// This update can involve querying Platform for the current identity contract nonce
    /// If the sdk user requests to put a state transition the counter is checked and either
    /// returns an error or is updated.
    pub(crate) identity_contract_nonce_counter: tokio::sync::Mutex<
        BTreeMap<(Identifier, Identifier), (prelude::IdentityContractNonce, LastQueryTimestamp)>,
    >,
}

impl Default for InternalSdkCache {
    fn default() -> Self {
        InternalSdkCache {
            identity_contract_nonce_counter: Mutex::new(BTreeMap::<(Identifier, Identifier), (IdentityContractNonce, LastQueryTimestamp)>::new()),
        }
    }
}