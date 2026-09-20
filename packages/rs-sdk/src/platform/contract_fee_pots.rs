//! The fee pots of a data contract (`getContractFeePots`): what the document action fees of
//! the contract have paid into the owner pot and the moderators pot, and the last claim of each
//! pot: the epoch and the block time it was paid out in, and the identity that claimed it.
//!
//! A document type prices its actions with the `actionFees` keyword. What a fee collects waits
//! in a pot until a [`ContractFeeClaim`](dpp::state_transition::contract_fee_claim_transition)
//! pays it out (see [`ClaimContractFees`](crate::platform::transition::contract_fee_claim)),
//! which a pot allows once per epoch. The pots tell a recipient whether a claim is worth
//! sending: [`ContractFeePotState::credits`] is what it would pay, and a
//! [`ContractFeePotState::last_claim_epoch`] equal to the current epoch means the pot was
//! already paid out in it. [`ContractFeePotState::last_claim`] also tells a member of the
//! moderation team which member last claimed for the team, and when.
//!
//! [`ContractFeePots::fetch`] takes the contract id, or a [`ContractFeePotsQuery`]. A contract
//! that charges no fees, or whose fees nobody has paid yet, reads as two empty pots. A contract
//! the network does not hold is an error on the node, not an absent result.
//!
//! The type also implements [`FetchUnproved`] for the unverified fast path.

use crate::platform::{Fetch, FetchUnproved, Identifier, Query, QuerySettings};
use crate::Error;
use dapi_grpc::platform::v0::get_contract_fee_pots_request::GetContractFeePotsRequestV0;
use dapi_grpc::platform::v0::{get_contract_fee_pots_request, GetContractFeePotsRequest};
pub use drive_proof_verifier::types::contract_moderation::{
    ContractFeePot, ContractFeePotLastClaim, ContractFeePotState, ContractFeePots,
};

/// Query for the fee pots of a contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractFeePotsQuery {
    /// The contract.
    pub contract_id: Identifier,
}

impl Query<GetContractFeePotsRequest> for ContractFeePotsQuery {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetContractFeePotsRequest, Error> {
        Ok(GetContractFeePotsRequest {
            version: Some(get_contract_fee_pots_request::Version::V0(
                GetContractFeePotsRequestV0 {
                    contract_id: self.contract_id.to_vec(),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Query<GetContractFeePotsRequest> for Identifier {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetContractFeePotsRequest, Error> {
        ContractFeePotsQuery { contract_id: *self }.query(settings)
    }
}

impl Fetch for ContractFeePots {
    type Query = GetContractFeePotsRequest;
    type Request = GetContractFeePotsRequest;
}

impl FetchUnproved for ContractFeePots {
    type Request = GetContractFeePotsRequest;
}
