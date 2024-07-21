use std::collections::BTreeMap;
use platform_value::{Value};
use crate::fee::Credits;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransition;
use crate::state_transition::documents_batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;

impl DocumentCreateTransitionV0Methods for DocumentCreateTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentCreateTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            DocumentCreateTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        match self {
            DocumentCreateTransition::V0(v0) => v0.base = base,
        }
    }

    fn entropy(&self) -> [u8; 32] {
        match self {
            DocumentCreateTransition::V0(v0) => v0.entropy,
        }
    }

    fn set_entropy(&mut self, entropy: [u8; 32]) {
        match self {
            DocumentCreateTransition::V0(v0) => v0.entropy = entropy,
        }
    }

    fn data(&self) -> &BTreeMap<String, Value> {
        match self {
            DocumentCreateTransition::V0(v0) => &v0.data,
        }
    }

    fn data_mut(&mut self) -> &mut BTreeMap<String, Value> {
        match self {
            DocumentCreateTransition::V0(v0) => &mut v0.data,
        }
    }

    fn set_data(&mut self, data: BTreeMap<String, Value>) {
        match self {
            DocumentCreateTransition::V0(v0) => v0.data = data,
        }
    }

    fn prefunded_voting_balance(&self) -> &Option<(String, Credits)> {
        match self {
            DocumentCreateTransition::V0(v0) => v0.prefunded_voting_balance(),
        }
    }

    fn prefunded_voting_balances_mut(&mut self) -> &mut Option<(String, Credits)> {
        match self {
            DocumentCreateTransition::V0(v0) => v0.prefunded_voting_balances_mut(),
        }
    }

    fn set_prefunded_voting_balance(&mut self, index_name: String, amount: Credits) {
        match self {
            DocumentCreateTransition::V0(v0) => v0.set_prefunded_voting_balance(index_name, amount),
        }
    }

    fn clear_prefunded_voting_balance(&mut self) {
        match self {
            DocumentCreateTransition::V0(v0) => v0.clear_prefunded_voting_balance(),
        }
    }
}
