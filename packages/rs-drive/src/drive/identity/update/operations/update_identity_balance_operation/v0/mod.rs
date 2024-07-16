use crate::drive::balances::balance_path_vec;
use crate::drive::Drive;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::MAX_CREDITS;
use dpp::fee::Credits;
use grovedb::Element;

impl Drive {
    /// Updates the balance of a specific identity.
    ///
    /// This function creates a low-level drive operation to update an identity's balance in GroveDB.
    /// The function validates the balance against a predefined maximum to prevent overflow.
    ///
    /// # Parameters
    ///
    /// - `identity_id`: A 32-byte array that uniquely identifies the identity whose balance needs to be updated.
    /// - `balance`: The new balance (`Credits`) to be set for the identity.
    ///
    /// # Returns
    ///
    /// - `Result<LowLevelDriveOperation, Error>`: Returns a low-level drive operation that, when applied, will update the identity's balance in GroveDB.
    ///   Returns an error if the balance exceeds the maximum allowable value (`MAX_CREDITS`).
    ///
    /// # Errors
    ///
    /// - `IdentityError::CriticalBalanceOverflow`: Returned when attempting to set a balance that exceeds the maximum allowable value (`MAX_CREDITS`).
    ///
    /// # Usage
    ///
    /// This function is intended to be used internally within the crate's `drive::identity::update` module.
    ///
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
