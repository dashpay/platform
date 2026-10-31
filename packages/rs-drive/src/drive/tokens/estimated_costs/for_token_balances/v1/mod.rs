use crate::drive::tokens::paths::tokens_root_path;
use crate::drive::Drive;
use crate::util::estimated_layers::register_keeping_deeper_estimated_layers;
use crate::util::estimated_layers::EstimatedLayers;
use grovedb::batch::KeyInfoPath;

impl Drive {
    /// Adds estimation costs for token balances (protocol version 15): v0, keeping a deeper
    /// description of the root and the Tokens layer that another write of the batch registered.
    /// The escrow of the token marketplace's listings sits deeper in the Tokens layer than the
    /// balances key v0 describes, and the offers' escrow deeper in the root tree, under the
    /// prefunded specialized balances; a batch moving tokens or credits into or out of escrow
    /// writes both, whichever registers first.
    pub(super) fn add_estimation_costs_for_token_balances_v1(
        token_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut EstimatedLayers,
    ) {
        register_keeping_deeper_estimated_layers(
            estimated_costs_only_with_layer_info,
            &[
                KeyInfoPath::from_known_path([]),
                KeyInfoPath::from_known_path(tokens_root_path()),
            ],
            |estimated_costs_only_with_layer_info| {
                Self::add_estimation_costs_for_token_balances_v0(
                    token_id,
                    estimated_costs_only_with_layer_info,
                )
            },
        );
    }
}
