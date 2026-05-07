pub(crate) mod v0;
/// v1 of the batch transformer fixes issue #2867: when per-transition
/// validation produces no action, we should not synthesise an empty paid
/// action via merge_many/flatten — instead the transition becomes
/// UnpaidConsensusError so prepare_proposal removes it from the block.
/// v0 is preserved for older platform versions (≤ PROTOCOL_VERSION_11).
pub(crate) mod v1;
