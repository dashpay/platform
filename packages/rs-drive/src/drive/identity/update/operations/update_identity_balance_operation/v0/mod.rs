use crate::drive::balances::balance_path_vec;
use crate::drive::Drive;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::balances::credits::MAX_CREDITS;
use dpp::fee::Credits;
use grovedb::Element;

impl Drive {
    pub(in crate::drive::identity::update) fn update_identity_balance_operation_v0(
        &self,
        identity_id: [u8; 32],
        balance: Credits,
    ) -> Result<LowLevelDriveOperation, Error> {
        // while i64::MAX could potentially work, best to avoid it.
        if balance >= MAX_CREDITS {
            return Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set balance to over max credits amount (i64::MAX)",
            )));
        }

        let balance_path = balance_path_vec();

        Ok(LowLevelDriveOperation::replace_for_known_path_key_element(
            balance_path,
            identity_id.to_vec(),
            Element::new_sum_item(balance as i64),
        ))
    }
}
