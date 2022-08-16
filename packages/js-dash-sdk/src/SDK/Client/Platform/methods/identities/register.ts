import { Platform } from "../../Platform";
import broadcastStateTransition from "../../broadcastStateTransition";

/**
 * Register identities to the platform
 *
 * @param {number} [fundingAmount=10000] - funding amount in duffs
 * @returns {Identity} identity - a register and funded identity
 */
export default async function register(
  this: Platform,
  fundingAmount : number = 100000
): Promise<any> {
    await this.initialize();

    const { client } = this;

    const account = await client.getWalletAccount();

    const {
        transaction: assetLockTransaction,
        privateKey: assetLockPrivateKey,
        outputIndex: assetLockOutputIndex
    } = await this.identities.utils.createAssetLockTransaction(fundingAmount);

    // Broadcast Asset Lock transaction
    await account.broadcastTransaction(assetLockTransaction);

    const assetLockProof = await this.identities.utils
      .createAssetLockProof(assetLockTransaction, assetLockOutputIndex);

    const { identity, identityCreateTransition, identityIndex } = await this.identities.utils
      .createIdentityCreateTransition(assetLockProof, assetLockPrivateKey);

    await broadcastStateTransition(this, identityCreateTransition, { ackFactor: 3 });

    // If state transition was broadcast without any errors, import identity to the account
   account.storage
    .getWalletStore(account.walletId)
    .insertIdentityIdAtIndex(
      identity.getId().toString(),
      identityIndex,
    );

    // Current identity object will not have metadata or balance information
    const registeredIdentity = await this.identities.get(identity.getId().toString());

    // We cannot just return registeredIdentity as we want to
    // keep additional information (assetLockProof and transaction) instance
    identity.setMetadata(registeredIdentity.getMetadata());
    identity.setBalance(registeredIdentity.getBalance());
    identity.setPublicKeys(registeredIdentity.getPublicKeys());

    return identity;
}
