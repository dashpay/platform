use crate::drive::Drive;

impl Drive {
    // TODO: We deal with it in upcoming PR
    // /// Fetches the Identity's balance as PartialIdentityInfo from the backing store
    // /// Passing apply as false get the estimated cost instead
    // pub fn prove_identity_with_balance_with_cost(
    //     &self,
    //     identity_id: [u8; 32],
    //     transaction: TransactionArg,
    // ) -> Result<Option<PartialIdentityInfo>, Error> {
    //     Ok((
    //         self.prove_identity_balance(
    //             identity_id,
    //             transaction,
    //         )?
    //             .map(|balance| PartialIdentityInfo {
    //                 id: Identifier::new(identity_id),
    //                 loaded_public_keys: Default::default(),
    //                 balance: Some(balance),
    //                 revision: None,
    //             }),
    //         FeeResult::new_from_processing_fee(balance_cost),
    //     ))
    // }
    //
    // /// Fetches the Identity's balance with keys as PartialIdentityInfo from the backing store
    // /// Passing apply as false get the estimated cost instead
    // pub fn fetch_identity_balance_with_keys(
    //     &self,
    //     identity_key_request: IdentityKeysRequest,
    //     transaction: TransactionArg,
    // ) -> Result<Option<PartialIdentityInfo>, Error> {
    //     let id = Identifier::new(identity_key_request.identity_id);
    //     let balance =
    //         self.fetch_identity_balance(identity_key_request.identity_id, true, transaction)?;
    //     let Some(balance) = balance else {
    //         return Ok(None);
    //     };
    //
    //     let loaded_public_keys = self.fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
    //         identity_key_request,
    //         transaction,
    //     )?;
    //     Ok(Some(PartialIdentityInfo {
    //         id,
    //         loaded_public_keys,
    //         balance: Some(balance),
    //         revision: None,
    //     }))
    // }
    //
    // /// Fetches the Identity's balance with keys as PartialIdentityInfo from the backing store
    // /// Passing apply as false get the estimated cost instead
    // pub fn fetch_identity_balance_with_keys_with_cost(
    //     &self,
    //     identity_key_request: IdentityKeysRequest,
    //     apply: bool,
    //     epoch: &Epoch,
    //     transaction: TransactionArg,
    // ) -> Result<(Option<PartialIdentityInfo>, FeeResult), Error> {
    //     let balance_cost = epoch.cost_for_known_cost_item(FetchIdentityBalanceProcessingCost);
    //     if apply == false {
    //         let keys_cost = identity_key_request.processing_cost(epoch)?;
    //         return Ok((
    //             None,
    //             FeeResult::new_from_processing_fee(balance_cost + keys_cost),
    //         ));
    //     }
    //     // let's start by getting the balance
    //     let id = Identifier::new(identity_key_request.identity_id);
    //     let balance =
    //         self.fetch_identity_balance(identity_key_request.identity_id, apply, transaction)?;
    //     let Some(balance) = balance else {
    //         return Ok((None, FeeResult::new_from_processing_fee(balance_cost)));
    //     };
    //
    //     let keys_cost = identity_key_request.processing_cost(epoch)?;
    //
    //     let loaded_public_keys = self.fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
    //         identity_key_request,
    //         transaction,
    //     )?;
    //     Ok((
    //         Some(PartialIdentityInfo {
    //             id,
    //             loaded_public_keys,
    //             balance: Some(balance),
    //             revision: None,
    //         }),
    //         FeeResult::new_from_processing_fee(balance_cost + keys_cost),
    //     ))
    // }
}
