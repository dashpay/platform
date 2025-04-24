use crate::drive::tokens::paths::token_direct_purchase_root_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Sets the direct purchase price of a token
    pub(super) fn token_set_direct_purchase_price_operations_v0(
        &self,
        token_id: [u8; 32],
        price: Option<TokenPricingSchedule>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_selling_prices(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let direct_selling_path = token_direct_purchase_root_path_vec();

        if let Some(price) = price {
            let serialized_price =
                price.serialize_consume_to_bytes_with_platform_version(platform_version)?;
            drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                direct_selling_path,
                token_id.to_vec(),
                Element::new_item(serialized_price),
            ));
        } else {
            drive_operations.push(LowLevelDriveOperation::GroveOperation(
                QualifiedGroveDbOp::delete_op(direct_selling_path, token_id.to_vec()),
            ));
        }

        Ok(drive_operations)
    }
}
