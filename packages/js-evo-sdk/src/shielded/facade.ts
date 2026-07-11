import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

/**
 * Read-only access to the shielded (Orchard) pool: total balance, encrypted
 * notes, anchors, nullifier statuses. Each query has a `*WithProof` variant
 * that additionally returns proof + metadata for cryptographic verification.
 *
 * Building / signing / broadcasting shielded transitions is intentionally
 * out of scope here — that requires the Orchard prover and is tracked
 * separately. See PR #3235 for the deferral rationale.
 */
export class ShieldedFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  // ── Pool state ────────────────────────────────────────────────────────

  async poolState(): Promise<bigint | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getShieldedPoolState();
  }

  async poolStateWithProof(): Promise<wasm.ProofMetadataResponseTyped<bigint | null>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getShieldedPoolStateWithProofInfo();
  }

  // ── Encrypted notes ──────────────────────────────────────────────────

  async encryptedNotes(startIndex: bigint, count: number): Promise<wasm.ShieldedEncryptedNote[]> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getShieldedEncryptedNotes(startIndex, count);
  }

  async encryptedNotesWithProof(
    startIndex: bigint,
    count: number,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.ShieldedEncryptedNote[]>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getShieldedEncryptedNotesWithProofInfo(startIndex, count);
  }

  // ── Anchors ──────────────────────────────────────────────────────────

  async anchors(): Promise<Uint8Array[]> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getShieldedAnchors();
  }

  async anchorsWithProof(): Promise<wasm.ProofMetadataResponseTyped<Uint8Array[]>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getShieldedAnchorsWithProofInfo();
  }

  async mostRecentAnchor(): Promise<Uint8Array | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMostRecentShieldedAnchor();
  }

  async mostRecentAnchorWithProof(): Promise<wasm.ProofMetadataResponseTyped<Uint8Array | null>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMostRecentShieldedAnchorWithProofInfo();
  }

  // ── Nullifiers ───────────────────────────────────────────────────────

  /**
   * Each nullifier must be a `Uint8Array` of exactly 32 bytes; otherwise
   * the underlying call rejects with `InvalidArgument`.
   */
  async nullifiers(nullifiers: Uint8Array[]): Promise<wasm.ShieldedNullifierStatus[]> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getShieldedNullifiers(nullifiers);
  }

  async nullifiersWithProof(
    nullifiers: Uint8Array[],
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.ShieldedNullifierStatus[]>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getShieldedNullifiersWithProofInfo(nullifiers);
  }
}
