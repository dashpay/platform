use crate::drive::Drive;
use crate::util::estimated_layers::register_keeping_deeper_estimated_layers;
use crate::util::estimated_layers::EstimatedLayers;
use grovedb::batch::KeyInfoPath;

impl Drive {
    /// Adds estimation costs for identity balances (protocol version 15): v0, keeping a deeper
    /// description of the root layer that another write of the batch registered. An offer of the
    /// token marketplace debits the buyer's balance and credits its escrow under the prefunded
    /// specialized balances, which sit deeper in the root tree than the balances, whichever
    /// registers first.
    pub(super) fn add_estimation_costs_for_balances_v1(
        estimated_costs_only_with_layer_info: &mut EstimatedLayers,
    ) {
        register_keeping_deeper_estimated_layers(
            estimated_costs_only_with_layer_info,
            &[KeyInfoPath::from_known_path([])],
            Self::add_estimation_costs_for_balances_v0,
        );
    }
}
