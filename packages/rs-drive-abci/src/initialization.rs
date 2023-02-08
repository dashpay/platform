use drive::dpp::identity::Identity;
use drive::drive::batch::{ContractOperationType, DriveOperationType, IdentityOperationType};
use crate::error::Error;
use crate::platform::Platform;

impl Platform {
    // fn register_initial_contract_operations(&self, path: &str, owner_id: Option<[u8;32]>, operations: &mut Vec<DriveOperationType>) {
    //     operations.push(DriveOperationType::ContractOperation(ContractOperationType::ApplyContract { contract: &Default::default(), storage_flags: None }))
    // }
    //
    // fn register_initial_identity_operations(&self, identity: Identity, operations: &mut Vec<DriveOperationType>) {
    //     operations.push(DriveOperationType::IdentityOperation(IdentityOperationType::AddNewIdentity { identity }))
    // }
    //
    // /// There are certain operations that need to be done on init chain.
    // /// These are:
    // /// Registering an Identity to hold system contracts
    // /// Registering system contracts
    // /// Registering documents for these contracts
    // pub fn register_initial_identities_and_contracts(&self) {
    //     // We first want to register an Identity
    //
    // }
}
