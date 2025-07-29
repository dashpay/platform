import { Identifier } from '@dashevo/wasm-dpp';
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

  // Create asset lock transaction
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

  // If wasm-sdk is available, delegate to it
  if (this.wasmSdk && this.getAdapter()) {
    const adapter = this.getAdapter()!;
    
    // Convert identity ID to string
    const identityIdString = typeof identityId === 'string' ? identityId : identityId.toString();
    
    // Convert asset lock proof to hex format for wasm-sdk
    const assetLockProofHex = adapter.convertAssetLockProof(assetLockProof);
    
    // Convert private key to WIF format
    const assetLockPrivateKeyWIF = adapter.convertPrivateKeyToWIF(assetLockPrivateKey);
    
    // Call wasm-sdk identityTopUp
    const result = await this.wasmSdk.identityTopUp(
      identityIdString,
      assetLockProofHex,
      assetLockPrivateKeyWIF
    );
    
    this.logger.debug(`[Identity#topUp] Topped up identity "${identityIdString}"`);
    
    return result.success !== false;
  }

  // Legacy implementation - will be removed once migration is complete
  identityId = Identifier.from(identityId);

  const identityTopUpTransition = await this.identities.utils
    .createIdentityTopUpTransition(assetLockProof, assetLockPrivateKey, identityId);
  this.logger.silly(`[Identity#topUp] Created IdentityTopUpTransition with asset lock tx "${assetLockTransaction.hash}"`);

  // Skipping validation because it's already done in createIdentityTopUpTransition
  await broadcastStateTransition(this, identityTopUpTransition, {
    skipValidation: true,
  });
  this.logger.silly('[Identity#topUp] Broadcasted IdentityTopUpTransition');

  return true;
}

export default topUp;