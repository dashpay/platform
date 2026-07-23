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
 * Implementations must verify the GroveDB query/result and authenticate its
 * root with the Tenderdash quorum signature for the supplied network and
 * metadata. Returning successfully means the complete request binding was
 * verified; presence or structural decoding of proof bytes is not sufficient.
 */
export interface IPlatformProofVerifier {
  verifyStateTransitionResult(input: {
    serializedStateTransition: Uint8Array;
    response: IStateTransitionResult;
    network: string;
    protocolVersion: number;
  }): Promise<void>;

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
