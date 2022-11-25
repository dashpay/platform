import { Platform } from "../../Platform";
import broadcastStateTransition from "../../broadcastStateTransition";
import {wait} from "../../../../../utils/wait";

/**
 * Register identities to the platform
 *
 * @param {number} [fundingAmount=1000000] - funding amount in duffs
 * @returns {Identity} identity - a register and funded identity
 */
export default async function register(
  this: Platform,
  fundingAmount : number = 1000000
): Promise<any> {
    this.logger.debug(`[Identity#register] Register identity with funding amount ${fundingAmount}`);
    await this.initialize();

    const { client } = this;

    const account = await client.getWalletAccount();

    const {
        transaction: assetLockTransaction,
        privateKey: assetLockPrivateKey,
        outputIndex: assetLockOutputIndex
    } = await this.identities.utils.createAssetLockTransaction(fundingAmount);

    this.logger.silly(`[Identity#register] Broadcast asset lock transaction "${assetLockTransaction.hash}"`);
    // Broadcast Asset Lock transaction
    await account.broadcastTransaction(assetLockTransaction);

    this.logger.silly(`[Identity#register] Wait for asset lock proof "${assetLockTransaction.hash}"`);
    const assetLockProof = await this.identities.utils
      .createAssetLockProof(assetLockTransaction, assetLockOutputIndex);

    const { identity, identityCreateTransition, identityIndex } = await this.identities.utils
      .createIdentityCreateTransition(assetLockProof, assetLockPrivateKey);

    this.logger.silly(`[Identity#register] Broadcast identity create ST "${identityCreateTransition.hash}"`);
    await broadcastStateTransition(this, identityCreateTransition);

    // If state transition was broadcast without any errors, import identity to the account
   account.storage
    .getWalletStore(account.walletId)
    .insertIdentityIdAtIndex(
      identity.getId().toString(),
      identityIndex,
    );

    // Fetch identity from the network because the current identity object
    // doesn't have metadata and balance information. Due to replication lag
    // some nodes couldn't have it yet, so we need to try multiple times
    const maxAttempts = 20;
    let attempt = 0;
    let registeredIdentity: any = null; // We don't have Identity type yet

    while (registeredIdentity === null && attempt < maxAttempts) {
      await wait(100);

      registeredIdentity = await this.identities.get(identity.getId());
      attempt++;
    }

    if (registeredIdentity === null) {
      throw new Error(`Can't fetch created identity with id ${identity.getId()}`);
    }

    // We cannot just return registeredIdentity as we want to
    // keep additional information (assetLockProof and transaction) instance
    identity.setMetadata(registeredIdentity.getMetadata());
    identity.setBalance(registeredIdentity.getBalance());
    identity.setPublicKeys(registeredIdentity.getPublicKeys());

    this.logger.debug(`[Identity#register] Registered identity "${identity.getId()}"`);

    return identity;
}
