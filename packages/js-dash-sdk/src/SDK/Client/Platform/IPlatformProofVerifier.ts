import type {
  GetDataContractHistoryResponse,
} from '@dashevo/dapi-client/lib/methods/platform/getDataContractHistory/GetDataContractHistoryResponse';
import type { IStateTransitionResult } from './IStateTransitionResult';

export interface VerifiedDataContractHistoryEntry {
  date: bigint;
  value: Uint8Array;
}

/**
 * Trust boundary for legacy JavaScript Platform operations.
 *
 * Implementations must verify GroveDB proofs and authenticate their roots with
 * Tenderdash quorum signatures for the supplied network and metadata.
 * Presence or structural decoding of proof bytes is not sufficient.
 */
export interface IPlatformProofVerifier {
  /**
   * Verify either the transition execution result or an authenticated,
   * height-pinned snapshot of its affected state. A snapshot does not prove
   * that this exact transition executed. Callers must reject consensus errors
   * from the original response before invoking this method.
   */
  verifyStateTransitionResult(input: {
    serializedStateTransition: Uint8Array;
    response: IStateTransitionResult;
    network: string;
    protocolVersion: number;
  }): Promise<void>;

  /**
   * Verify the returned contract-history data and its complete query binding.
   */
  verifyDataContractHistory(input: {
    contractId: Uint8Array;
    startAtMs: bigint;
    limit: number;
    offset: number;
    response: GetDataContractHistoryResponse;
    network: string;
    protocolVersion: number;
  }): Promise<VerifiedDataContractHistoryEntry[]>;
}
