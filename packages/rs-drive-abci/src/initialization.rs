use std::borrow::Cow;
use drive::contract::DataContract;
use crate::error::Error;
use crate::platform::Platform;
use drive::dpp::identity::Identity;
use drive::drive::batch::{ContractOperationType, DriveOperationType, IdentityOperationType};

impl Platform {
    fn register_initial_contract_operations(&self, contract: DataContract, operations: &mut Vec<DriveOperationType>) {
        operations.push(DriveOperationType::ContractOperation(ContractOperationType::ApplyContract { contract: Cow::Owned(contract), storage_flags: None }))
    }

    fn register_initial_identity_operations(&self, identity: Identity, operations: &mut Vec<DriveOperationType>) {
        operations.push(DriveOperationType::IdentityOperation(IdentityOperationType::AddNewIdentity { identity }))
    }

    /// There are certain operations that need to be done on init chain.
    /// These are:
    /// Registering an Identity to hold system contracts
    /// Registering system contracts
    /// Registering documents for these contracts
    pub fn register_initial_identities_and_contracts(&self) -> Result<(), Error> {
        let mut operation_types = vec![];
        let identity = Identity::random_identity(2, Some(0));
        self.register_initial_identity_operations(identity.clone(), &mut operation_types);
        // We first want to register an Identity
        let contracts = contracts::SystemContract::load_contracts(contracts::SystemContract::all_contracts(), identity.id.clone()).map_err(Error::Protocol)?;
        for (_, contract) in contracts.into_iter() {
            self.register_initial_contract_operations(contract, &mut operation_types);
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn test_register_initial_identities() {

    }
}
