#[cfg(feature = "server")]
mod add_perpetual_distribution;
#[cfg(feature = "server")]
mod add_pre_programmed_distribution;
#[cfg(feature = "server")]
mod fetch;

mod mark_perpetual_release_as_distributed;
#[cfg(feature = "server")]
mod prove;
/// Token distribution queries
pub mod queries;
