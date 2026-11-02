import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

/**
 * Contract groups: identity-owned sets of contracts, contract document types and contract
 * tokens. A group's id derives from the registering identity and the identity nonce of the
 * contract create transition that registered it.
 */
export class ContractGroupsFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  /** The group's owner, admins, name and description, or `undefined` when no group has the id. */
  async info(contractGroupId: wasm.IdentifierLike): Promise<wasm.ContractGroupInfo | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractGroupInfo(contractGroupId);
  }

  async infoWithProof(contractGroupId: wasm.IdentifierLike):
    Promise<wasm.ProofMetadataResponseTyped<wasm.ContractGroupInfo | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractGroupInfoWithProofInfo(contractGroupId);
  }

  /**
   * One page of the group's members of one kind, in key order. Pass the page's
   * `nextStartAfter` as the next query's `startAfter`; a page without one is the last.
   */
  async members(query: wasm.ContractGroupMembersQuery): Promise<wasm.ContractGroupMembersPage> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractGroupMembers(query);
  }

  async membersWithProof(
    query: wasm.ContractGroupMembersQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.ContractGroupMembersPage>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractGroupMembersWithProofInfo(query);
  }

  /**
   * The groups a contract belongs to: as a whole, through its document types and through its
   * tokens. Every entry is empty when it belongs to no group.
   */
  async forContract(contractId: wasm.IdentifierLike): Promise<wasm.ContractGroupMemberships> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractGroupsForContract(contractId);
  }

  async forContractWithProof(
    contractId: wasm.IdentifierLike,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.ContractGroupMemberships>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractGroupsForContractWithProofInfo(contractId);
  }
}
