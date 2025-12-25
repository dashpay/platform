mod address;
mod fee_strategy;
mod input_output;
mod signer;

pub use address::PlatformAddressWasm;
pub use fee_strategy::{
    default_fee_strategy, fee_strategy_from_steps, fee_strategy_from_steps_or_default,
    FeeStrategyStepWasm,
};
pub use input_output::{
    extract_addresses, extract_amounts, inputs_to_btree_map, outputs_to_btree_map,
    PlatformAddressInputWasm, PlatformAddressOutputWasm,
};
pub use signer::PlatformAddressSignerWasm;
