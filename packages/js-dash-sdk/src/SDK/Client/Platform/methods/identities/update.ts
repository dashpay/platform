// @ts-ignore
import Identifier from "@dashevo/dpp/lib/Identifier";
import {Platform} from "../../Platform";

import createAssetLockTransaction from "../../createAssetLockTransaction";
import createAssetLockProof from "./internal/createAssetLockProof";
import createIdentityUpdateTransition from "./internal/createIdentityUpdateTransition";
import broadcastStateTransition from "../../broadcastStateTransition";

/**
 * Update platform identities
 *
 * @param {Platform} this - bound instance class
 * @param {Identifier|string} identityId - id of the identity to top up
 * @param {number} amount - amount to top up in duffs
 * @returns {boolean}
 */
export async function update(this: Platform, identityId: Identifier | string, amount: number): Promise<any> {
  await this.initialize();

  const { client } = this;

  identityId = Identifier.from(identityId);

  const account = await client.getWalletAccount();

  const {
    transaction: assetLockTransaction,
    privateKey: assetLockPrivateKey,
    outputIndex: assetLockOutputIndex
  } = await createAssetLockTransaction(this, amount);

  // Broadcast Asset Lock transaction
  await account.broadcastTransaction(assetLockTransaction);
  // Create a proof for the asset lock transaction
  const assetLockProof = await createAssetLockProof(this, assetLockTransaction, assetLockOutputIndex);

  const identity = await this.identities.get(identityId);

  if (identity === null) {
    throw new Error(`Identity with ID ${identityId.toString()} not found`)
  }

  // @ts-ignore
  const identityTopUpTransition = await createIdentityUpdateTransition(this, assetLockProof, assetLockPrivateKey, identity);

  // Broadcast ST
  await broadcastStateTransition(this, identityTopUpTransition);

  return true;
}

export default update;
