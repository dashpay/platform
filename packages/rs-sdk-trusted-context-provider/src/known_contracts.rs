//! The rule of the known-contract caches the local resolver and the verified
//! provider keep.

use std::collections::HashMap;
use std::sync::Arc;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::prelude::{DataContract, Identifier};

/// Hold `contract` under its id, unless the contract held there has a higher version. A
/// registration that waited on its fetch, or a hydration from a cache saved earlier, must not
/// put an older contract back over one another operation promoted meanwhile: a page that
/// operation verified against the newer one would be checked against the older one next. An
/// equal version replaces the held body.
pub(crate) fn hold_unless_newer_held(
    known: &mut HashMap<Identifier, Arc<DataContract>>,
    contract: Arc<DataContract>,
) {
    let id = contract.id();
    if known
        .get(&id)
        .is_some_and(|held| held.version() > contract.version())
    {
        return;
    }
    known.insert(id, contract);
}
