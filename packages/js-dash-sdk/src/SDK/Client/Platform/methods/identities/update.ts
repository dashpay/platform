// @ts-ignore
import Identifier from "@dashevo/dpp/lib/Identifier";
import { Platform } from "../../Platform";
import IdentityPublicKey from "@dashevo/dpp/lib/identity/IdentityPublicKey"

import createAssetLockTransaction from "../../createAssetLockTransaction";
import createAssetLockProof from "./internal/createAssetLockProof";
import createIdentityUpdateTransition from "./internal/createIdentityUpdateTransition";
import broadcastStateTransition from "../../broadcastStateTransition";

/**
 * Update platform identities
 *
 * @param {Platform} this - bound instance class
 * @param {Identifier|string} identityId - id of the identity to top up
 * @param {IdentityPublicKey[]} addPublicKeys - public keys to add
 * @param {number[]} disablePublicKeys - public key IDs to disable
 * @param {number} publicKeysDisabledAt - timestamp to disable at
 *
 * @returns {boolean}
 */
export async function update(
  this: Platform,
  identityId: Identifier | string,
  addPublicKeys?: IdentityPublicKey[],
  disablePublicKeys?: number[],
  publicKeysDisabledAt?: number,
  ): Promise<any> {
  await this.initialize();

  const { client } = this;

  identityId = Identifier.from(identityId);

  const account = await client.getWalletAccount();

  const identityIndex = await account.getUnusedIdentityIndex();

  // @ts-ignore
  const { privateKey: identityPrivateKey } = account.identities.getIdentityHDKeyByIndex(identityIndex, 0);

  const identity = await this.identities.get(identityId);

  if (identity === null) {
    throw new Error(`Identity with ID ${identityId.toString()} not found`)
  }

  // @ts-ignore
  const identityTopUpTransition = await createIdentityUpdateTransition(
    this,
    identityPrivateKey,
    identity,
    addPublicKeys,
    disablePublicKeys,
    publicKeysDisabledAt,
  );

  // Broadcast ST
  await broadcastStateTransition(this, identityTopUpTransition);

  return true;
}

export default update;
