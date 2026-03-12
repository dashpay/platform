use dpp::state_transition::StateTransitionType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=StateTransitionTypes)]
#[allow(non_camel_case_types)]
pub enum StateTransitionTypeWasm {
    DataContractCreate = 0,
    DocumentsBatch = 1,
    IdentityCreate = 2,
    IdentityTopUp = 3,
    DataContractUpdate = 4,
    IdentityUpdate = 5,
    IdentityCreditWithdrawal = 6,
    IdentityCreditTransfer = 7,
    MasternodeVote = 8,
    IdentityCreditTransferToAddresses = 9,
    IdentityCreateFromAddresses = 10,
    IdentityTopUpFromAddresses = 11,
    AddressFundsTransfer = 12,
    AddressFundingFromAssetLock = 13,
    AddressCreditWithdrawal = 14,
}

impl From<StateTransitionType> for StateTransitionTypeWasm {
    fn from(state_transition_type: StateTransitionType) -> Self {
        match state_transition_type {
            StateTransitionType::DataContractCreate => StateTransitionTypeWasm::DataContractCreate,
            StateTransitionType::Batch => StateTransitionTypeWasm::DocumentsBatch,
            StateTransitionType::IdentityCreate => StateTransitionTypeWasm::IdentityCreate,
            StateTransitionType::IdentityTopUp => StateTransitionTypeWasm::IdentityTopUp,
            StateTransitionType::DataContractUpdate => StateTransitionTypeWasm::DataContractUpdate,
            StateTransitionType::IdentityUpdate => StateTransitionTypeWasm::IdentityUpdate,
            StateTransitionType::IdentityCreditWithdrawal => {
                StateTransitionTypeWasm::IdentityCreditWithdrawal
            }
            StateTransitionType::IdentityCreditTransfer => {
                StateTransitionTypeWasm::IdentityCreditTransfer
            }
            StateTransitionType::MasternodeVote => StateTransitionTypeWasm::MasternodeVote,
            StateTransitionType::IdentityCreditTransferToAddresses => {
                StateTransitionTypeWasm::IdentityCreditTransferToAddresses
            }
            StateTransitionType::IdentityCreateFromAddresses => {
                StateTransitionTypeWasm::IdentityCreateFromAddresses
            }
            StateTransitionType::IdentityTopUpFromAddresses => {
                StateTransitionTypeWasm::IdentityTopUpFromAddresses
            }
            StateTransitionType::AddressFundsTransfer => {
                StateTransitionTypeWasm::AddressFundsTransfer
            }
            StateTransitionType::AddressFundingFromAssetLock => {
                StateTransitionTypeWasm::AddressFundingFromAssetLock
            }
            StateTransitionType::AddressCreditWithdrawal => {
                StateTransitionTypeWasm::AddressCreditWithdrawal
            }
            _ => todo!("shielded state transition types not yet implemented in wasm"),
        }
    }
}
