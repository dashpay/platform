# wasm-sdk query review (fetch/fetch_many, proofs, efficiency)

Context: review of query implementations to verify we use fetch/fetch_many correctly, avoid inefficient patterns, and handle proofs.

## Findings

- `src/queries/token.rs:getTokenPerpetualDistributionLastClaim`  
  - Uses a direct gRPC call with `prove: false` to avoid context/provider issues. Result is unverified and may diverge from the proof-backed variant. Should either default to the proofable path or document that this endpoint is best-effort/non-verified.
- `src/queries/protocol.rs:getProtocolVersionUpgradeVoteStatus` (and proof variant TODO)  
  - Uses `fetch_votes` without a proof-capable path; proof variant remains unimplemented. Protocol vote status cannot currently be verified.
- `src/queries/epoch.rs:getEvonodesProposedEpochBlocksBy*WithProofInfo`  
  - Proof-supporting variants are stubbed (return errors). Evonode proposed block counts cannot be fetched with proofs yet.

## Suggested follow-ups

1) Add/enable batch identity key queries (or parallelize) and return combined proofs for `getIdentitiesContractKeys*`.  
2) Decide whether `getTokenPerpetualDistributionLastClaim` should default to the proof-backed path or be clearly marked as best-effort.  
3) Implement proof-capable variants for protocol vote status and evonode proposed block counts once underlying fetch/proof traits land.
