use crate::drive::Drive;
use crate::error::Error;
use grovedb::TransactionArg;

impl Drive {
    /// Proves an Identity's balance from the backing store
    pub fn prove_contract(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let contract_query = Self::fetch_contract_query(contract_id);
        self.grove_get_proved_path_query(&contract_query, false, transaction, &mut vec![])
    }
}
