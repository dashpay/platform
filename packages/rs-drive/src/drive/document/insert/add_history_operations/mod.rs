mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use dpp::block::block_info::BlockInfo;
use dpp::document::document_event::DocumentEvent;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Adds a document event to the document history system contract, for
    /// document types that subscribed to history via the
    /// `keepsTransferHistory`, `keepsPurchaseHistory` and
    /// `keepsPricingHistory` configuration flags.
    ///
    /// # Parameters
    ///
    /// * `source_data_contract_id`: The contract of the document the event happened to.
    /// * `source_document_type_name`: The type of that document.
    /// * `source_document_id`: That document's id.
    /// * `owner_id`: The identity that acted, which owns the history document.
    /// * `owner_nonce`: The acting identity's contract nonce, which derives the history
    ///   document's id.
    /// * `event`: The event: a transfer, a purchase or a price update.
    /// * `block_info`: The block being executed.
    /// * `estimated_costs_only_with_layer_info`: The estimation map, when only estimating costs.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LowLevelDriveOperation>)` with the operations inserting the history document.
    /// * `Err(Error)` when the method version is unknown, the document history contract or its
    ///   document type for the event cannot be loaded, or building the insert fails.
    #[allow(clippy::too_many_arguments)]
    pub fn add_document_history_operations(
        &self,
        source_data_contract_id: Identifier,
        source_document_type_name: &str,
        source_document_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        event: DocumentEvent,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .insert
            .add_history_operations
        {
            0 => self.add_document_history_operations_v0(
                source_data_contract_id,
                source_document_type_name,
                source_document_id,
                owner_id,
                owner_nonce,
                event,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_document_history_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::system_data_contracts::SystemDataContract;
    use grovedb::Element;

    const SOURCE_CONTRACT: [u8; 32] = [9; 32];

    /// A drive holding the document history contract, as a chain's genesis stores it.
    fn setup_drive_with_document_history(platform_version: &PlatformVersion) -> Drive {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = drive
            .cache
            .system_data_contracts
            .load_document_history(platform_version)
            .expect("expected the document history contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to store the document history contract");
        drive
    }

    /// Records `event` for a document of the source contract, at its own block time so
    /// every record has its own `$createdAt`.
    fn record(
        drive: &Drive,
        event: DocumentEvent,
        nonce: IdentityNonce,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let operations = drive.add_document_history_operations(
            Identifier::new(SOURCE_CONTRACT),
            "card",
            Identifier::new([nonce as u8; 32]),
            Identifier::new([1; 32]),
            nonce,
            event,
            &BlockInfo::default_with_time(1_000 * nonce),
            &mut None,
            None,
            platform_version,
        )?;
        drive.apply_batch_low_level_drive_operations(
            None,
            None,
            operations,
            &mut vec![],
            &platform_version.drive,
        )
    }

    fn element(
        drive: &Drive,
        path: Vec<Vec<u8>>,
        key: &[u8],
        platform_version: &PlatformVersion,
    ) -> Element {
        let path: Vec<&[u8]> = path.iter().map(Vec::as_slice).collect();
        drive
            .grove
            .get_raw(
                path.as_slice().into(),
                key,
                None,
                &platform_version.drive.grove_version,
            )
            .value
            .expect("expected the element")
    }

    /// The tree keeping the count and the sum of the prices recorded for the source
    /// contract under `byContract`: the `$createdAt` tree below the contract's id.
    fn prices_by_contract(
        drive: &Drive,
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Element {
        let mut path = contract_document_type_path_vec(
            SystemDataContract::DocumentHistory.id().as_bytes(),
            document_type_name,
        );
        path.push(b"dataContractId".to_vec());
        path.push(SOURCE_CONTRACT.to_vec());
        element(drive, path, b"$createdAt", platform_version)
    }

    /// The prices of a contract's listings are summed in 128 bits: listings go on being
    /// recorded once their prices pass what 64 bits hold.
    #[test]
    fn should_record_listings_whose_prices_sum_past_64_bits() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_document_history(platform_version);
        let highest = i64::MAX as u64;

        for nonce in 1..=3 {
            record(
                &drive,
                DocumentEvent::PriceUpdate { price: highest },
                nonce,
                platform_version,
            )
            .expect("expected to record the listing");
        }

        let Element::ProvableCountProvableBigSumTree(_, count, sum, _) =
            prices_by_contract(&drive, "priceUpdate", platform_version)
        else {
            panic!("expected the prices of a contract's listings in a 128-bit tree");
        };
        assert_eq!((count, sum), (3, 3 * i128::from(i64::MAX)));
    }

    /// A purchase's price also enters the total the `purchase` type keeps of every price,
    /// in the type's primary tree.
    #[test]
    fn should_record_purchases_whose_prices_sum_past_64_bits() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_document_history(platform_version);
        let highest = i64::MAX as u64;

        for nonce in 1..=2 {
            record(
                &drive,
                DocumentEvent::Purchase {
                    seller_id: Identifier::new([2; 32]),
                    price: highest,
                },
                nonce,
                platform_version,
            )
            .expect("expected to record the purchase");
        }

        let Element::ProvableCountProvableBigSumTree(_, count, sum, _) =
            prices_by_contract(&drive, "purchase", platform_version)
        else {
            panic!("expected the prices of a contract's purchases in a 128-bit tree");
        };
        assert_eq!((count, sum), (2, 2 * i128::from(i64::MAX)));

        let Element::CountBigSumTree(_, count, sum, _) = element(
            &drive,
            contract_document_type_path_vec(
                SystemDataContract::DocumentHistory.id().as_bytes(),
                "purchase",
            ),
            &[0],
            platform_version,
        ) else {
            panic!("expected the purchase type to keep its total in a 128-bit tree");
        };
        assert_eq!((count, sum), (2, 2 * i128::from(i64::MAX)));
    }

    /// Protocol version 14 keeps the prices in a 64-bit tree, which the second listing at
    /// the highest price does not fit.
    #[test]
    fn should_refuse_a_listing_past_64_bits_at_protocol_version_14() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let drive = setup_drive_with_document_history(platform_version);
        let highest = i64::MAX as u64;

        record(
            &drive,
            DocumentEvent::PriceUpdate { price: highest },
            1,
            platform_version,
        )
        .expect("expected to record the first listing");
        assert!(matches!(
            prices_by_contract(&drive, "priceUpdate", platform_version),
            Element::ProvableCountProvableSumTree(_, 1, i64::MAX, _)
        ));

        let error = record(
            &drive,
            DocumentEvent::PriceUpdate { price: 1 },
            2,
            platform_version,
        )
        .expect_err("expected the second listing to be refused");
        assert!(
            error.to_string().contains("overflow"),
            "expected an overflow, got {error}"
        );
    }
}
