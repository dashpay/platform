use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use moka::sync::Cache;
use platform_version::version::PlatformVersion;

#[derive(Hash, Eq, PartialEq, Clone)]
struct DataContractTypeAndVersion {
    contract: SystemDataContract,
    version: u32,
}

/// System contracts
pub struct SystemDataContracts {
    contracts: Cache<DataContractTypeAndVersion, DataContract>,
}

impl SystemDataContracts {
    /// Create a new SystemDataContracts
    pub fn new() -> Self {
        Self {
            contracts: Cache::new(50),
        }
    }
    /// Retrieves a reference to a `DataContract` from the cache, or loads it if not already present.
    ///
    /// This function takes a `SystemDataContract` and a reference to a `PlatformVersion` as arguments.
    /// It constructs a key using the `contract` and the `withdrawals` field of `system_data_contracts`
    /// in `platform_version`. If the `contracts` cache does not contain the key, it loads the system data contract
    /// using the `load_system_data_contract` function and inserts it into `contracts`.
    ///
    /// # Arguments
    ///
    /// * `contract` - A `SystemDataContract` that specifies the type of contract to retrieve or load.
    /// * `platform_version` - A reference to a `PlatformVersion` used to determine the version of the contract.
    ///
    /// # Returns
    ///
    /// * `Result<&DataContract, Error>` - A result that contains a reference to the `DataContract` if successful,
    /// or an `Error` if the loading operation fails.
    ///
    /// # Panics
    ///
    /// This function will panic if it tries to retrieve a `DataContract` from `contracts` that does not exist.
    /// However, this should never happen because the function ensures the `DataContract` is loaded
    pub fn get_or_load(
        &self,
        contract: SystemDataContract,
        platform_version: &PlatformVersion,
    ) -> Result<DataContract, Error> {
        let key = DataContractTypeAndVersion {
            contract,
            version: platform_version.system_data_contracts.withdrawals as u32,
        };

        if !self.contracts.contains_key(&key) {
            let data_contract = load_system_data_contract(contract, platform_version)?;
            self.contracts.insert(key.clone(), data_contract);
        }

        Ok(self.contracts.get(&key).unwrap())
    }
}
