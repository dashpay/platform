use crate::drive::identity::key::fetch::{IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::default_costs::KnownCostItem::FetchIdentityBalanceProcessingCost;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::result::FeeResult;
use crate::fee_pools::epochs::Epoch;
use dpp::identifier::Identifier;
use dpp::identity::PartialIdentity;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the Identity's balance as PartialIdentityInfo from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_with_balance(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Option<PartialIdentity>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        Ok(self
            .fetch_identity_balance_operations(
                identity_id,
                true,
                transaction,
                &mut drive_operations,
            )?
            .map(|balance| PartialIdentity {
                id: Identifier::new(identity_id),
                loaded_public_keys: Default::default(),
                balance: Some(balance),
                revision: None,
            }))
    }

    /// Fetches the Identity's balance as PartialIdentityInfo from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_with_balance_with_cost(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        epoch: &Epoch,
        transaction: TransactionArg,
    ) -> Result<(Option<PartialIdentity>, FeeResult), Error> {
        let balance_cost = epoch.cost_for_known_cost_item(FetchIdentityBalanceProcessingCost);
        if !apply {
            return Ok((None, FeeResult::new_from_processing_fee(balance_cost)));
        }
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        Ok((
            self.fetch_identity_balance_operations(
                identity_id,
                apply,
                transaction,
                &mut drive_operations,
            )?
            .map(|balance| PartialIdentity {
                id: Identifier::new(identity_id),
                loaded_public_keys: Default::default(),
                balance: Some(balance),
                revision: None,
            }),
            FeeResult::new_from_processing_fee(balance_cost),
        ))
    }

    /// Fetches the Identity's balance with keys as PartialIdentityInfo from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_balance_with_keys(
        &self,
        identity_key_request: IdentityKeysRequest,
        transaction: TransactionArg,
    ) -> Result<Option<PartialIdentity>, Error> {
        let id = Identifier::new(identity_key_request.identity_id);
        let balance = self.fetch_identity_balance(identity_key_request.identity_id, transaction)?;
        let Some(balance) = balance else {
            return Ok(None);
        };

        let loaded_public_keys = self.fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
            identity_key_request,
            transaction,
        )?;
        Ok(Some(PartialIdentity {
            id,
            loaded_public_keys,
            balance: Some(balance),
            revision: None,
        }))
    }

    /// Fetches the Identity's balance with keys as PartialIdentityInfo from the backing store
    /// Passing apply as false get the estimated cost instead
    pub fn fetch_identity_balance_with_keys_with_cost(
        &self,
        identity_key_request: IdentityKeysRequest,
        apply: bool,
        epoch: &Epoch,
        transaction: TransactionArg,
    ) -> Result<(Option<PartialIdentity>, FeeResult), Error> {
        let balance_cost = epoch.cost_for_known_cost_item(FetchIdentityBalanceProcessingCost);
        if !apply {
            let keys_cost = identity_key_request.processing_cost(epoch)?;
            return Ok((
                None,
                FeeResult::new_from_processing_fee(balance_cost + keys_cost),
            ));
        }
        // let's start by getting the balance
        let id = Identifier::new(identity_key_request.identity_id);
        let balance = self.fetch_identity_balance(identity_key_request.identity_id, transaction)?;
        let Some(balance) = balance else {
            return Ok((None, FeeResult::new_from_processing_fee(balance_cost)));
        };

        let keys_cost = identity_key_request.processing_cost(epoch)?;

        let loaded_public_keys = self.fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
            identity_key_request,
            transaction,
        )?;
        Ok((
            Some(PartialIdentity {
                id,
                loaded_public_keys,
                balance: Some(balance),
                revision: None,
            }),
            FeeResult::new_from_processing_fee(balance_cost + keys_cost),
        ))
    }
}
