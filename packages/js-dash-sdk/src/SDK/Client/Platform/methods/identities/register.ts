import { Platform } from "../../Platform";
import { wait } from "../../../../../utils/wait";
import createAssetLockTransaction from "../../createAssetLockTransaction";
import createIdentityCreateTransition from "./internal/createIdentityCreateTransition";
import createAssetLockProof from "./internal/createAssetLockProof";

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
    const { client } = this;

    const account = await client.getWalletAccount();

    const {
        transaction: assetLockTransaction,
        privateKey: assetLockPrivateKey,
        outputIndex: assetLockOutputIndex
    } = await createAssetLockTransaction(this, fundingAmount);

    // Broadcast Asset Lock transaction
    await account.broadcastTransaction(assetLockTransaction);
    const assetLockProof = await createAssetLockProof(this, assetLockTransaction);

    const { identity, identityCreateTransition, identityIndex } = await createIdentityCreateTransition(
        this, assetLockTransaction, assetLockOutputIndex, assetLockProof, assetLockPrivateKey
    );

    await client.getDAPIClient().platform.broadcastStateTransition(identityCreateTransition.toBuffer());

    // If state transition was broadcast without any errors, import identity to the account
    account.storage.insertIdentityIdAtIndex(
        account.walletId,
        identity.getId().toString(),
        identityIndex,
    );

    let fetchedIdentity;
    do {
        await wait(1000);

        fetchedIdentity = await this.client.getDAPIClient().platform.getIdentity(identity.getId());
    } while (!fetchedIdentity);

    return identity;
}
