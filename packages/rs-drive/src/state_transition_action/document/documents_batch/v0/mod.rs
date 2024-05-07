use dpp::fee::Credits;
use crate::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::identifier::Identifier;
use dpp::prelude::UserFeeIncrease;
use crate::state_transition_action::document::documents_batch::document_transition::document_purchase_transition_action::DocumentPurchaseTransitionActionAccessorsV0;

/// action v0
#[derive(Default, Debug, Clone)]
pub struct DocumentsBatchTransitionActionV0 {
    /// The owner making the transitions
    pub owner_id: Identifier,
    /// The inner transitions
    pub transitions: Vec<DocumentTransitionAction>,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}

impl DocumentsBatchTransitionActionV0 {
    pub(super) fn all_purchases_amount(&self) -> Option<Credits> {
        let (total, any_purchases) = self
            .transitions
            .iter()
            .filter_map(|transition| match transition {
                DocumentTransitionAction::PurchaseAction(purchase) => Some(purchase.price()),
                _ => None,
            })
            .fold((0, false), |(acc, _), price| (acc + price, true));

        if any_purchases {
            Some(total) // Return the sum as Some(Credits) if there were any purchases
        } else {
            None // Return None if no purchases were found
        }
    }
}
