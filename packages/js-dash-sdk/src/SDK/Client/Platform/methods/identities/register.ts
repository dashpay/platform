import { Platform } from '../../Platform';
import broadcastStateTransition from '../../broadcastStateTransition';

/**
 * Register identities to the platform
 *
 * @param {number} [fundingAmount=1000000] - funding amount in duffs
 * @returns {Identity} identity - a register and funded identity
 */
export default async function register(
  this: Platform,
  fundingAmount : number = 1000000,
): Promise<any> {
  this.logger.debug(`[Identity#register] Register identity with funding amount ${fundingAmount}`);
  await this.initialize();

  const { client } = this;

  const account = await client.getWalletAccount();

  const {
    transaction: assetLockTransaction,
    privateKey: assetLockPrivateKey,
    outputIndex: assetLockOutputIndex,
  } = await this.identities.utils.createAssetLockTransaction(fundingAmount);

  // Broadcast Asset Lock transaction
  await account.broadcastTransaction(assetLockTransaction);
  this.logger.silly(`[Identity#register] Broadcasted asset lock transaction "${assetLockTransaction.hash}"`);

  const assetLockProof = await this.identities.utils
    .createAssetLockProof(assetLockTransaction, assetLockOutputIndex);
  this.logger.silly(`[Identity#register] Created asset lock proof with tx "${assetLockTransaction.hash}"`);

  const { identity, identityCreateTransition, identityIndex } = await this.identities.utils
    .createIdentityCreateTransition(assetLockProof, assetLockPrivateKey);

  this.logger.silly(`[Identity#register] Created IdentityCreateTransition with identity id "${identity.getId()}" using asset lock tx "${assetLockTransaction.hash}" `);

  // Skipping validation because it's already done in createIdentityCreateTransition
  await broadcastStateTransition(this, identityCreateTransition, {
    skipValidation: true,
  });
  this.logger.silly('[Identity#register] Broadcasted IdentityCreateTransition');

  const identityId = identity.getId();

  // If state transition was broadcast without any errors, import identity to the account
  account.storage
    .getWalletStore(account.walletId)
    .insertIdentityIdAtIndex(
      identityId.toString(),
      identityIndex,
    );

  // Acknowledge identifier to handle retry attempts to mitigate
  // state transition propagation lag
  this.fetcher.acknowledgeIdentifier(identityId);

  const registeredIdentity = await this.identities.get(identityId);

  if (registeredIdentity === null) {
    throw new Error(`Can't fetch created identity with id ${identityId}`);
  }

  // We cannot just return registeredIdentity as we want to
  // keep additional information (assetLockProof and transaction) instance
  identity.setMetadata(registeredIdentity.getMetadata());
  identity.setBalance(registeredIdentity.getBalance());
  identity.setPublicKeys(registeredIdentity.getPublicKeys());

  this.logger.debug(`[Identity#register] Registered identity "${identityId}"`);

  return identity;
}
