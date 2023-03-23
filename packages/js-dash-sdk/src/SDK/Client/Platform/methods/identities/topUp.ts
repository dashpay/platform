import Identifier from '@dashevo/dpp/lib/Identifier';
import { Platform } from '../../Platform';

import broadcastStateTransition from '../../broadcastStateTransition';

/**
 * Register identities to the platform
 *
 * @param {Platform} this - bound instance class
 * @param {Identifier|string} identityId - id of the identity to top up
 * @param {number} amount - amount to top up in duffs
 * @returns {boolean}
 */
export async function topUp(
  this: Platform,
  identityId: Identifier | string,
  amount: number,
): Promise<any> {
  this.logger.debug(`[Identity#topUp] Top up identity ${identityId.toString()} with amount ${amount}`);
  await this.initialize();

  const { client } = this;

  const account = await client.getWalletAccount();

  const {
    transaction: assetLockTransaction,
    privateKey: assetLockPrivateKey,
    outputIndex: assetLockOutputIndex,
  } = await this.identities.utils.createAssetLockTransaction(amount);

  // Broadcast Asset Lock transaction
  await account.broadcastTransaction(assetLockTransaction);
  this.logger.silly(`[Identity#topUp] Broadcasted asset lock transaction "${assetLockTransaction.hash}"`);
  // Create a proof for the asset lock transaction
  const assetLockProof = await this.identities.utils
    .createAssetLockProof(assetLockTransaction, assetLockOutputIndex);
  this.logger.silly(`[Identity#topUp] Created asset lock proof with tx "${assetLockTransaction.hash}"`);

  const identityTopUpTransition = await this.identities.utils
    .createIdentityTopUpTransition(assetLockProof, assetLockPrivateKey, identityId);
  this.logger.silly(`[Identity#register] Created IdentityTopUpTransition with asset lock tx "${assetLockTransaction.hash}"`);

  // Broadcast ST
  await broadcastStateTransition(this, identityTopUpTransition);
  this.logger.silly('[Identity#register] Broadcasted IdentityTopUpTransition');

  return true;
}

export default topUp;
