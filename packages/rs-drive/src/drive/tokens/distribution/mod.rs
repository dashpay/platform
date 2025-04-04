#[cfg(feature = "server")]
mod add_perpetual_distribution;
#[cfg(feature = "server")]
mod add_pre_programmed_distribution;
#[cfg(feature = "server")]
mod fetch;
#[cfg(feature = "server")]
mod mark_perpetual_release_as_distributed;
#[cfg(feature = "server")]
mod mark_pre_programmed_release_as_distributed;
#[cfg(feature = "server")]
mod prove;
/// Token distribution queries
pub mod queries;
// TODO: Disabled module
// #[cfg(feature = "server")]
// mod set_perpetual_distribution_next_event_for_identity_id;
