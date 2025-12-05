use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::identity::core_script::CoreScript;
use crate::withdrawal::Pooling;

pub trait AddressCreditWithdrawalTransitionAccessorsV0 {
    /// Get optional output (for change)
    fn output(&self) -> Option<&(PlatformAddress, Credits)>;
    /// Set optional output
    fn set_output(&mut self, output: Option<(PlatformAddress, Credits)>);

    /// Get core fee per byte
    fn core_fee_per_byte(&self) -> u32;
    /// Set core fee per byte
    fn set_core_fee_per_byte(&mut self, core_fee_per_byte: u32);

    /// Get pooling
    fn pooling(&self) -> Pooling;
    /// Set pooling
    fn set_pooling(&mut self, pooling: Pooling);

    /// Get output script
    fn output_script(&self) -> &CoreScript;
    /// Set output script
    fn set_output_script(&mut self, output_script: CoreScript);
}
