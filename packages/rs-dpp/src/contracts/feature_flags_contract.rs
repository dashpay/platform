use super::SystemIDs;

pub mod types {
    pub const UPDATE_CONSENSUS_PARAMS: &str = "updateConsensusParams";
}

pub fn system_ids() -> SystemIDs {
    SystemIDs {
        contract_id: "H9sjb2bHG8t7gq5SwNdqzMWG7KR6sf3CbziFzthCkDD6".to_string(),
        owner_id: "HY1keaRK5bcDmujNCQq5pxNyvAiHHpoHQgLN5ppiu4kh".to_string(),
    }
}
