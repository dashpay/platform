import { Platform } from "../../Platform";
import createAssetLockTransaction from "../../createAssetLockTransaction";
import createIdentityCreateTransition from "./internal/createIdentityCreateTransition";
import createAssetLockProof from "./internal/createAssetLockProof";
import broadcastStateTransition from "../../broadcastStateTransition";

/**
 * Register identities to the platform
 *
 * @param {number} [fundingAmount=10000] - funding amount in duffs
 * @returns {Identity} identity - a register and funded identity
 */
export default async function register(
  this: Platform,
  fundingAmount : number = 10000
): Promise<any> {
    await this.initialize();

    const { client } = this;

    const account = await client.getWalletAccount();

    const {
        transaction: assetLockTransaction,
        privateKey: assetLockPrivateKey,
        outputIndex: assetLockOutputIndex
    } = await createAssetLockTransaction(this, fundingAmount);

    // Broadcast Asset Lock transaction
    await account.broadcastTransaction(assetLockTransaction);
    const assetLockProof = await createAssetLockProof(this, assetLockTransaction, assetLockOutputIndex);

    const { identity, identityCreateTransition, identityIndex } = await createIdentityCreateTransition(
        this, assetLockProof, assetLockPrivateKey
    );
    await broadcastStateTransition(this, identityCreateTransition);

    // If state transition was broadcast without any errors, import identity to the account
   account.storage
    .getWalletStore(account.walletId)
    .insertIdentityIdAtIndex(
      identity.getId().toString(),
      identityIndex,
    );

    // Current identity object will not have metadata or balance information
    const registeredIdentity = await this.identities.get(identity.id.toString());

    // We cannot just return registeredIdentity as we want to
    // keep additional information (assetLockProof and transaction) instance
    identity.setMetadata(registeredIdentity.metadata);
    identity.setBalance(registeredIdentity.balance);

    return identity;
}
