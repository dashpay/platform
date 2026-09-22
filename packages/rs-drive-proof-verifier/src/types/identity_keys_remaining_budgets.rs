use crate::types::RetrievedObjects;
use dpp::fee::Credits;
use dpp::identity::KeyID;
use std::ops::Deref;

/// What is left of the budgets of several keys of one identity.
///
/// One entry per requested key id: `Some(credits)` for a budgeted key (zero means the key can no
/// longer sign), `None` for a key without a budget or a key that does not exist.
#[derive(Debug, Default, Clone, PartialEq, Eq, derive_more::From)]
pub struct IdentityKeysRemainingBudgets(
    /// Key id to remaining budget
    #[from]
    pub RetrievedObjects<KeyID, Credits>,
);

impl Deref for IdentityKeysRemainingBudgets {
    type Target = RetrievedObjects<KeyID, Credits>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromIterator<(KeyID, Option<Credits>)> for IdentityKeysRemainingBudgets {
    fn from_iter<T: IntoIterator<Item = (KeyID, Option<Credits>)>>(iter: T) -> Self {
        iter.into_iter()
            .collect::<RetrievedObjects<KeyID, Credits>>()
            .into()
    }
}
