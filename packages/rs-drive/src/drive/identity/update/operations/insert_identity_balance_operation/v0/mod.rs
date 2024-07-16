use crate::drive::balances::balance_path_vec;
use crate::drive::Drive;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::MAX_CREDITS;
use dpp::fee::Credits;
use grovedb::Element;

impl Drive {
    /// Creates a balance key-value with specified amount
    /// Must be used only to create initial key-value. To update balance
    /// use `add_to_identity_balance`, `remove_from_identity_balance`,
    /// and `apply_balance_change_from_fee_to_identity`
    pub(crate) fn insert_identity_balance_operation_v0(
        &self,
        identity_id: [u8; 32],
        balance: Credits,
    ) -> Result<LowLevelDriveOperation, Error> {
        // while i64::MAX could potentially work, best to avoid it.
        if balance >= MAX_CREDITS {
            return Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set balance to over max credits amount (i64::MAX)",
            )));
        };

        let balance_path = balance_path_vec();

        Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
            balance_path,
            identity_id.to_vec(),
            Element::new_sum_item(balance as i64),
        ))
    }
}
