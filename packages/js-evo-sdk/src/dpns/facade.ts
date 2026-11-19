import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class DpnsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  async convertToHomographSafe(input: string): Promise<string> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.dpnsConvertToHomographSafe(input);
  }

  async isValidUsername(label: string): Promise<boolean> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.dpnsIsValidUsername(label);
  }

  async isContestedUsername(label: string): Promise<boolean> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.dpnsIsContestedUsername(label);
  }

  async isNameAvailable(label: string): Promise<boolean> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.dpnsIsNameAvailable(label);
  }

  async resolveName(name: string): Promise<string | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.dpnsResolveName(name);
  }

  /**
   * Registers a DPNS name. A contested name joins a contest: pass
   * `options.contestFund` as the most, in credits, the registration pays into
   * it, or leave it out to state the fund to join read just before the domain
   * is submitted.
   */
  async registerName(options: wasm.DpnsRegisterNameOptions): Promise<wasm.RegisterDpnsNameResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.dpnsRegisterName(options);
  }

  /**
   * Whether a DPNS name is tombstoned: its owner gave it up for good, and
   * nobody can register it while the tombstone exists. `isNameAvailable` is
   * false for such a name. False for a registered name and for a free one, and
   * always false before protocol version 15.
   */
  async isNameTombstoned(name: string): Promise<boolean> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.dpnsIsNameTombstoned(name);
  }

  /**
   * Deletes a DPNS name its owner no longer wants (protocol version 15). The
   * name is free again: anybody may register it. Resolves to the id of the
   * deleted domain document.
   *
   * A contested name (`isContestedUsername`) can not be deleted, only
   * tombstoned with `tombstoneName`: the call rejects before anything is
   * broadcast.
   */
  async deleteName(options: wasm.DpnsDeleteNameOptions): Promise<wasm.Identifier> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.dpnsDeleteName(options);
  }

  /**
   * Tombstones a DPNS name its owner no longer wants (protocol version 15):
   * the name is deleted and nobody can register it again while the tombstone
   * exists. Resolves to the tombstone document the network stored.
   *
   * A tombstone can not be undone by anyone: neither its owner nor anybody
   * else can delete it. The DPNS contract says only that a tombstoned name may
   * later be brought back and redistributed through a direct or indirect will
   * of the DAO.
   *
   * Any name can be tombstoned. It is the only way to give up a contested name.
   */
  async tombstoneName(options: wasm.DpnsTombstoneNameOptions): Promise<wasm.Document> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.dpnsTombstoneName(options);
  }

  async usernames(query: wasm.DpnsUsernamesQuery): Promise<string[]> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernames(query);
  }

  async username(identityId: wasm.IdentifierLike): Promise<string | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsername(identityId);
  }

  async usernamesWithProof(query: wasm.DpnsUsernamesQuery): Promise<wasm.ProofMetadataResponseTyped<Array<string>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernamesWithProofInfo(query);
  }

  async usernameWithProof(identityId: wasm.IdentifierLike):
    Promise<wasm.ProofMetadataResponseTyped<string | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernameWithProofInfo(identityId);
  }

  async getUsernameByName(username: string): Promise<wasm.DpnsUsernameInfo | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernameByName(username);
  }

  async getUsernameByNameWithProof(
    username: string,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.DpnsUsernameInfo | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernameByNameWithProofInfo(username);
  }
}
