use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use std::sync::Arc;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionActionV0;

impl TokenTransferTransitionActionV0 {
    /// Converts a `TokenTransferTransitionV0` into a `TokenTransferTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token transfer transition, looks up the necessary data contracts, performs required
    /// checks, and applies the relevant logic for token transfer. It also calculates the fees associated with the transaction.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the transfer.
    /// * `value` - The `TokenTransferTransitionV0` struct containing the transition data, including token amount,
    ///   recipient details, and encrypted notes.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing transaction cost calculations when full state is not needed.
    /// * `transaction` - The transaction context, which includes state details and necessary operations for the transition.
    /// * `block_info` - Information about the current block to calculate the fees for the transition.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns the associated `DataContractFetchInfo`.
    /// * `platform_version` - A reference to the platform version, ensuring that the transition respects version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<(TokenTransferTransitionActionV0, FeeResult), Error>` - Returns a tuple containing the constructed
    ///   `TokenTransferTransitionActionV0` and the calculated `FeeResult` if successful, or an error if the transition cannot
    ///   be created or an issue arises with the provided state or data.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_token_transfer_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenTransferTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, FeeResult), Error> {
        let TokenTransferTransitionV0 {
            base,
            amount,
            recipient_id,
            public_note,
            shared_encrypted_note,
            private_encrypted_note,
        } = value;

        let mut drive_operations = vec![];

        // Lookup the base action using the base transition data and contract information
        // There is no change note for transfer tokens
        let (base_action, _change_note) =
            TokenBaseTransitionAction::try_from_base_transition_with_contract_lookup(
                drive,
                owner_id,
                base,
                approximate_without_state_for_costs,
                transaction,
                &mut drive_operations,
                get_data_contract,
                platform_version,
            )?
            .into_data()?;

        let fee_result = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            drive.config.epochs_per_era,
            platform_version,
            None,
        )?;

        // Return the TokenTransferTransitionActionV0 with the relevant data
        Ok((
            TokenTransferTransitionActionV0 {
                base: base_action,
                amount,
                recipient_id,
                public_note,
                shared_encrypted_note,
                private_encrypted_note,
            },
            fee_result,
        ))
    }

    /// Converts a borrowed `TokenTransferTransitionV0` into a `TokenTransferTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token transfer transition similarly to the `try_from_token_transfer_transition_with_contract_lookup`
    /// method but operates on a borrowed reference of the `TokenTransferTransitionV0`. It performs the same checks and applies
    /// the token transfer logic, while avoiding copying the `TokenTransferTransitionV0` struct.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the transfer.
    /// * `value` - A reference to the `TokenTransferTransitionV0` struct containing the transition data, including token amount,
    ///   recipient details, and encrypted notes.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing transaction cost calculations when full state is not needed.
    /// * `transaction` - The transaction context, which includes state details and necessary operations for the transition.
    /// * `block_info` - Information about the current block to calculate the fees for the transition.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns the associated `DataContractFetchInfo`.
    /// * `platform_version` - A reference to the platform version, ensuring that the transition respects version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<(TokenTransferTransitionActionV0, FeeResult), Error>` - Returns a tuple containing the constructed
    ///   `TokenTransferTransitionActionV0` and the calculated `FeeResult` if successful, or an error if the transition cannot
    ///   be created or an issue arises with the provided state or data.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_transfer_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenTransferTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, FeeResult), Error> {
        let TokenTransferTransitionV0 {
            base,
            amount,
            recipient_id,
            public_note,
            shared_encrypted_note,
            private_encrypted_note,
        } = value;

        let mut drive_operations = vec![];

        // Lookup the base action using the borrowed base transition data and contract information
        // We can never change the note
        let (base_action, _change_note) =
            TokenBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                drive,
                owner_id,
                base,
                approximate_without_state_for_costs,
                transaction,
                &mut drive_operations,
                get_data_contract,
                platform_version,
            )?
            .into_data()?;

        let fee_result = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            drive.config.epochs_per_era,
            platform_version,
            None,
        )?;

        // Return the TokenTransferTransitionActionV0 with the relevant data
        Ok((
            TokenTransferTransitionActionV0 {
                base: base_action,
                amount: *amount,
                recipient_id: *recipient_id,
                public_note: public_note.clone(),
                shared_encrypted_note: shared_encrypted_note.clone(),
                private_encrypted_note: private_encrypted_note.clone(),
            },
            fee_result,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
        TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
        TokenBaseTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::{
        TokenTransferTransitionAction, TokenTransferTransitionActionAccessorsV0,
        TokenTransferTransitionActionV0,
    };
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use dpp::tokens::{PrivateEncryptedNote, SharedEncryptedNote};
    use platform_version::version::PlatformVersion;
    use std::sync::Arc;

    fn make_base() -> TokenBaseTransitionAction {
        let fetch_info = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::new([21u8; 32]),
            identity_contract_nonce: 11,
            token_contract_position: 0,
            data_contract: Arc::new(fetch_info),
            store_in_group: None,
            perform_action: true,
        })
    }

    fn sample_shared() -> SharedEncryptedNote {
        // SharedEncryptedNote = (SenderKeyIndex, RecipientKeyIndex, Vec<u8>)
        (1, 2, vec![0xaa, 0xbb])
    }

    fn sample_private() -> PrivateEncryptedNote {
        // PrivateEncryptedNote = (RootEncryptionKeyIndex, DerivationEncryptionKeyIndex, Vec<u8>)
        (3, 4, vec![0xee, 0xff])
    }

    fn make_action_v0(
        amount: u64,
        recipient: Identifier,
        public_note: Option<&str>,
        shared: Option<SharedEncryptedNote>,
        private_: Option<PrivateEncryptedNote>,
    ) -> TokenTransferTransitionActionV0 {
        TokenTransferTransitionActionV0 {
            base: make_base(),
            amount,
            recipient_id: recipient,
            public_note: public_note.map(|s| s.to_string()),
            shared_encrypted_note: shared,
            private_encrypted_note: private_,
        }
    }

    #[test]
    fn v0_amount_and_recipient_return_stored_values() {
        let v0 = make_action_v0(1_000, Identifier::new([2u8; 32]), None, None, None);
        assert_eq!(v0.amount(), 1_000);
        assert_eq!(v0.recipient_id(), Identifier::new([2u8; 32]));
    }

    #[test]
    fn v0_public_note_accessors_work() {
        let v0 = make_action_v0(1, Identifier::new([2u8; 32]), Some("tx"), None, None);
        assert_eq!(v0.public_note(), Some(&"tx".to_string()));
        assert_eq!(v0.public_note_owned(), Some("tx".to_string()));
    }

    #[test]
    fn v0_set_public_note_updates_field() {
        let mut v0 = make_action_v0(1, Identifier::new([2u8; 32]), None, None, None);
        v0.set_public_note(Some("added".to_string()));
        assert_eq!(v0.public_note(), Some(&"added".to_string()));
        v0.set_public_note(None);
        assert!(v0.public_note().is_none());
    }

    #[test]
    fn v0_shared_encrypted_note_accessors_work() {
        let shared = sample_shared();
        let v0 = make_action_v0(
            1,
            Identifier::new([2u8; 32]),
            None,
            Some(shared.clone()),
            None,
        );
        assert_eq!(v0.shared_encrypted_note(), Some(&shared));
        assert_eq!(v0.shared_encrypted_note_owned(), Some(shared));
    }

    #[test]
    fn v0_set_shared_encrypted_note_updates_and_clears() {
        let mut v0 = make_action_v0(1, Identifier::new([2u8; 32]), None, None, None);
        assert!(v0.shared_encrypted_note().is_none());
        let shared = sample_shared();
        v0.set_shared_encrypted_note(Some(shared.clone()));
        assert_eq!(v0.shared_encrypted_note(), Some(&shared));
        v0.set_shared_encrypted_note(None);
        assert!(v0.shared_encrypted_note().is_none());
    }

    #[test]
    fn v0_private_encrypted_note_accessors_work() {
        let private_ = sample_private();
        let v0 = make_action_v0(
            1,
            Identifier::new([2u8; 32]),
            None,
            None,
            Some(private_.clone()),
        );
        assert_eq!(v0.private_encrypted_note(), Some(&private_));
        assert_eq!(v0.private_encrypted_note_owned(), Some(private_));
    }

    #[test]
    fn v0_set_private_encrypted_note_updates_and_clears() {
        let mut v0 = make_action_v0(1, Identifier::new([2u8; 32]), None, None, None);
        assert!(v0.private_encrypted_note().is_none());
        let private_ = sample_private();
        v0.set_private_encrypted_note(Some(private_.clone()));
        assert_eq!(v0.private_encrypted_note(), Some(&private_));
        v0.set_private_encrypted_note(None);
        assert!(v0.private_encrypted_note().is_none());
    }

    #[test]
    fn v0_notes_ref_returns_cloned_triple() {
        let shared = sample_shared();
        let private_ = sample_private();
        let v0 = make_action_v0(
            1,
            Identifier::new([2u8; 32]),
            Some("pub"),
            Some(shared.clone()),
            Some(private_.clone()),
        );
        let (public_n, shared_n, private_n) = v0.notes();
        assert_eq!(public_n, Some("pub".to_string()));
        assert_eq!(shared_n, Some(shared));
        assert_eq!(private_n, Some(private_));

        // Ensure the v0 is still usable after notes() (non-consuming)
        assert_eq!(v0.amount(), 1);
    }

    #[test]
    fn v0_notes_owned_returns_all_three_owned() {
        let shared = sample_shared();
        let private_ = sample_private();
        let v0 = make_action_v0(
            1,
            Identifier::new([2u8; 32]),
            Some("pub"),
            Some(shared.clone()),
            Some(private_.clone()),
        );
        let (public_n, shared_n, private_n) = v0.notes_owned();
        assert_eq!(public_n, Some("pub".to_string()));
        assert_eq!(shared_n, Some(shared));
        assert_eq!(private_n, Some(private_));
    }

    #[test]
    fn v0_notes_none_triple_when_all_none() {
        let v0 = make_action_v0(1, Identifier::new([2u8; 32]), None, None, None);
        let (public_n, shared_n, private_n) = v0.notes();
        assert!(public_n.is_none());
        assert!(shared_n.is_none());
        assert!(private_n.is_none());
    }

    #[test]
    fn v0_default_base_accessors_delegate() {
        let v0 = make_action_v0(1, Identifier::new([2u8; 32]), None, None, None);
        assert_eq!(v0.token_id(), Identifier::new([21u8; 32]));
        assert_eq!(v0.token_position(), 0);
        assert_eq!(v0.identity_contract_nonce(), 11);
        let fetch = v0.data_contract_fetch_info();
        assert_eq!(v0.data_contract_id(), fetch.contract.id());
        assert_eq!(
            v0.data_contract_fetch_info_ref().contract.id(),
            fetch.contract.id()
        );
    }

    #[test]
    fn v0_base_owned_yields_same_token_id() {
        let v0 = make_action_v0(1, Identifier::new([2u8; 32]), None, None, None);
        let base = v0.base_owned();
        assert_eq!(base.token_id(), Identifier::new([21u8; 32]));
    }

    #[test]
    fn enum_from_v0_preserves_fields() {
        let shared = sample_shared();
        let private_ = sample_private();
        let v0 = make_action_v0(
            555,
            Identifier::new([3u8; 32]),
            Some("x"),
            Some(shared.clone()),
            Some(private_.clone()),
        );
        let wrapped: TokenTransferTransitionAction = v0.into();

        assert_eq!(wrapped.amount(), 555);
        assert_eq!(wrapped.recipient_id(), Identifier::new([3u8; 32]));
        assert_eq!(wrapped.public_note(), Some(&"x".to_string()));
        assert_eq!(wrapped.shared_encrypted_note(), Some(&shared));
        assert_eq!(wrapped.private_encrypted_note(), Some(&private_));
    }

    #[test]
    fn enum_setters_mutate_underlying_fields() {
        let mut wrapped: TokenTransferTransitionAction =
            make_action_v0(1, Identifier::new([1u8; 32]), None, None, None).into();

        wrapped.set_public_note(Some("pub".to_string()));
        assert_eq!(wrapped.public_note(), Some(&"pub".to_string()));

        let shared = sample_shared();
        wrapped.set_shared_encrypted_note(Some(shared.clone()));
        assert_eq!(wrapped.shared_encrypted_note(), Some(&shared));

        let private_ = sample_private();
        wrapped.set_private_encrypted_note(Some(private_.clone()));
        assert_eq!(wrapped.private_encrypted_note(), Some(&private_));
    }

    #[test]
    fn enum_clearing_setters_yield_none() {
        let shared = sample_shared();
        let private_ = sample_private();
        let mut wrapped: TokenTransferTransitionAction = make_action_v0(
            1,
            Identifier::new([1u8; 32]),
            Some("p"),
            Some(shared),
            Some(private_),
        )
        .into();

        wrapped.set_public_note(None);
        wrapped.set_shared_encrypted_note(None);
        wrapped.set_private_encrypted_note(None);

        assert!(wrapped.public_note().is_none());
        assert!(wrapped.shared_encrypted_note().is_none());
        assert!(wrapped.private_encrypted_note().is_none());
    }

    #[test]
    fn enum_notes_and_notes_owned_match_inputs() {
        let shared = sample_shared();
        let private_ = sample_private();
        let wrapped: TokenTransferTransitionAction = make_action_v0(
            1,
            Identifier::new([1u8; 32]),
            Some("pn"),
            Some(shared.clone()),
            Some(private_.clone()),
        )
        .into();

        let (p, s, pr) = wrapped.notes();
        assert_eq!(p, Some("pn".to_string()));
        assert_eq!(s, Some(shared.clone()));
        assert_eq!(pr, Some(private_.clone()));

        let (p2, s2, pr2) = wrapped.notes_owned();
        assert_eq!(p2, Some("pn".to_string()));
        assert_eq!(s2, Some(shared));
        assert_eq!(pr2, Some(private_));
    }

    #[test]
    fn enum_public_note_owned_consumes_and_returns_note() {
        let wrapped: TokenTransferTransitionAction =
            make_action_v0(1, Identifier::new([1u8; 32]), Some("taken"), None, None).into();
        let note = wrapped.public_note_owned();
        assert_eq!(note, Some("taken".to_string()));
    }

    #[test]
    fn enum_shared_and_private_note_owned_consume_self() {
        let shared = sample_shared();
        let wrapped: TokenTransferTransitionAction = make_action_v0(
            1,
            Identifier::new([1u8; 32]),
            None,
            Some(shared.clone()),
            None,
        )
        .into();
        assert_eq!(wrapped.shared_encrypted_note_owned(), Some(shared));

        let private_ = sample_private();
        let wrapped2: TokenTransferTransitionAction = make_action_v0(
            1,
            Identifier::new([1u8; 32]),
            None,
            None,
            Some(private_.clone()),
        )
        .into();
        assert_eq!(wrapped2.private_encrypted_note_owned(), Some(private_));
    }

    #[test]
    fn enum_base_and_base_owned_expose_same_token_id() {
        let wrapped: TokenTransferTransitionAction =
            make_action_v0(1, Identifier::new([1u8; 32]), None, None, None).into();
        let ref_id = wrapped.base().token_id();
        let owned_id = wrapped.base_owned().token_id();
        assert_eq!(ref_id, owned_id);
        assert_eq!(ref_id, Identifier::new([21u8; 32]));
    }
}
