use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_path;
use crate::drive::Drive;
use crate::util::estimated_layers::register_keeping_deeper_estimated_layers;
use crate::util::estimated_layers::EstimatedLayers;
use grovedb::batch::KeyInfoPath;

impl Drive {
    /// Adds estimation costs for a prefunded specialized balance update (protocol version 15):
    /// v1, keeping a deeper description of the root and the prefunded balances layer that another
    /// write of the batch registered: the escrow of the token marketplace's offers sits two levels
    /// below the voting balances' key in that layer.
    #[inline(always)]
    pub(super) fn add_estimation_costs_for_prefunded_specialized_balance_update_v2(
        estimated_costs_only_with_layer_info: &mut EstimatedLayers,
    ) {
        register_keeping_deeper_estimated_layers(
            estimated_costs_only_with_layer_info,
            &[
                KeyInfoPath::from_known_path([]),
                KeyInfoPath::from_known_path(prefunded_specialized_balances_path()),
            ],
            Self::add_estimation_costs_for_prefunded_specialized_balance_update_v1,
        );
    }
}
