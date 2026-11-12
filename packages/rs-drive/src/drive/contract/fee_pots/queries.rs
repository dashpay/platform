use crate::drive::contract::paths::{
    contract_fee_pots_path_vec, contract_last_fee_claim_key, contract_other_path_vec,
};
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use grovedb::{PathQuery, Query, SizedQuery};
use grovedb_version::version::GroveVersion;

impl Drive {
    /// The query for one fee pot of a contract: `[40, 64 | 192] -> contract id`.
    pub fn contract_fee_pot_query(contract_id: [u8; 32], pot: ContractFeePot) -> PathQuery {
        let mut query = Query::new();
        query.insert_key(contract_id.to_vec());
        PathQuery::new(
            contract_fee_pots_path_vec(pot),
            SizedQuery::new(query, None, None),
        )
    }

    /// The query for the last claims of the given pots of a contract:
    /// `[64, contract id, 2] -> 32 | 96`.
    pub fn contract_last_fee_claims_query(
        contract_id: [u8; 32],
        pots: &[ContractFeePot],
    ) -> PathQuery {
        let mut query = Query::new();
        for pot in pots {
            query.insert_key(contract_last_fee_claim_key(*pot).to_vec());
        }
        PathQuery::new(
            contract_other_path_vec(&contract_id),
            SizedQuery::new(query, None, None),
        )
    }

    /// The query for the given pots of a contract and the epochs they were last claimed in,
    /// merged into one proof.
    pub fn contract_fee_pots_query(
        contract_id: [u8; 32],
        pots: &[ContractFeePot],
        grove_version: &GroveVersion,
    ) -> Result<PathQuery, Error> {
        let mut queries: Vec<PathQuery> = pots
            .iter()
            .map(|pot| Self::contract_fee_pot_query(contract_id, *pot))
            .collect();
        queries.push(Self::contract_last_fee_claims_query(contract_id, pots));
        Ok(PathQuery::merge(queries.iter().collect(), grove_version)?)
    }
}
