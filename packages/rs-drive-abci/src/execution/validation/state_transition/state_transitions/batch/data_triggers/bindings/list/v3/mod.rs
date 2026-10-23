use crate::execution::validation::state_transition::batch::data_triggers::bindings::data_trigger_binding::DataTriggerBindingV0;
use crate::execution::validation::state_transition::batch::data_triggers::bindings::list::v2::data_trigger_bindings_list_v2;
use crate::execution::validation::state_transition::batch::data_triggers::triggers::reject::reject_data_trigger;

use dpp::errors::ProtocolError;
use dpp::system_data_contracts::token_market_contract;
use dpp::system_data_contracts::token_market_contract::v1::document_types::{ask, bid};
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionActionType;

/// Retrieves a list of data triggers binding with matching params.
///
/// v3 (PROTOCOL_VERSION_15): v2 plus a reject of every replace and delete of a token market
/// contract order (`ask` and `bid`) sent as a document transition. Only the token marketplace
/// transitions write orders: creating one as a document is refused by
/// `creationRestrictionMode` 2, and the types stay mutable and deletable, so documents carry a
/// revision and storage flags for the partial fills and closes the platform writes.
#[inline(always)]
pub(super) fn data_trigger_bindings_list_v3() -> Result<Vec<DataTriggerBindingV0>, ProtocolError> {
    let mut data_triggers = data_trigger_bindings_list_v2()?;
    for document_type in [ask::NAME, bid::NAME] {
        for transition_action_type in [
            DocumentTransitionActionType::Replace,
            DocumentTransitionActionType::Delete,
        ] {
            data_triggers.push(DataTriggerBindingV0 {
                data_contract_id: token_market_contract::ID,
                document_type: document_type.to_string(),
                transition_action_type,
                data_trigger: reject_data_trigger,
            });
        }
    }
    Ok(data_triggers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_reject_every_replace_and_delete_of_an_order_and_keep_the_v2_bindings() {
        let v2 = data_trigger_bindings_list_v2().expect("expected the v2 bindings");
        let v3 = data_trigger_bindings_list_v3().expect("expected the v3 bindings");
        assert_eq!(v3.len(), v2.len() + 4);

        let token_market_bindings: Vec<(&str, DocumentTransitionActionType)> = v3
            .iter()
            .filter(|binding| binding.data_contract_id == token_market_contract::ID)
            .map(|binding| {
                (
                    binding.document_type.as_str(),
                    binding.transition_action_type,
                )
            })
            .collect();
        assert_eq!(
            token_market_bindings,
            vec![
                (ask::NAME, DocumentTransitionActionType::Replace),
                (ask::NAME, DocumentTransitionActionType::Delete),
                (bid::NAME, DocumentTransitionActionType::Replace),
                (bid::NAME, DocumentTransitionActionType::Delete),
            ]
        );
    }
}
