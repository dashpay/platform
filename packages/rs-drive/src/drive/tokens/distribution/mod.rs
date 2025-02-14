#[cfg(feature = "server")]
mod add_perpetual_distribution;
#[cfg(feature = "server")]
mod add_pre_programmed_distribution;
#[cfg(feature = "server")]
mod fetch;

#[cfg(feature = "server")]
mod prove;
/// Token distribution queries
pub mod queries;
