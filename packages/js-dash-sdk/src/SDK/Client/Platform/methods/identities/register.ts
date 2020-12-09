import { Platform } from "../../Platform";
import { wait } from "../../../../../utils/wait";
import { createFakeInstantLock } from "../../../../../utils/createFakeIntantLock";
import createAssetLockTransaction from "../../createAssetLockTransaction";

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
    const { client, dpp, passFakeAssetLockProofForTests } = this;

    const account = await client.getWalletAccount();

    const {
        transaction: assetLockTransaction,
        privateKey: assetLockPrivateKey,
        outputIndex: assetLockOutputIndex
    } = await createAssetLockTransaction(this, fundingAmount);

    // Broadcast Asset Lock transaction
    await account.broadcastTransaction(assetLockTransaction);

    // Wait some time for propagation
    await wait(1000);

    const identityIndex = await account.getUnusedIdentityIndex();

    // @ts-ignore
    const { privateKey: identityPrivateKey } = account.getIdentityHDKeyByIndex(identityIndex, 0);
    const identityPublicKey = identityPrivateKey.toPublicKey();

    let instantLock;
    // Create poof that the transaction won't be double spend
    if (passFakeAssetLockProofForTests) {
        instantLock = createFakeInstantLock(assetLockTransaction.hash);
    } else {
        instantLock = await account.waitForInstantLock(assetLockTransaction.hash);
    }
    // @ts-ignore
    const assetLockProof = await dpp.identity.createInstantAssetLockProof(instantLock);

    // Create Identity
    // @ts-ignore
    const identity = dpp.identity.create(
        assetLockTransaction, assetLockOutputIndex, assetLockProof, [identityPublicKey]
    );

    // Create ST
    const identityCreateTransition = dpp.identity.createIdentityCreateTransition(identity);

    identityCreateTransition.signByPrivateKey(assetLockPrivateKey);

    const result = await dpp.stateTransition.validateStructure(identityCreateTransition);

    if (!result.isValid()) {
        throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
    }

    // Broadcast ST
    await client.getDAPIClient().platform.broadcastStateTransition(identityCreateTransition.toBuffer());

    account.storage.insertIdentityIdAtIndex(
        account.walletId,
        identity.getId().toString(),
        identityIndex,
    );

    // Wait some time for propagation
    await wait(6000);

    let fetchedIdentity;
    do {
        await wait(1000);

        fetchedIdentity = await this.client.getDAPIClient().platform.getIdentity(identity.getId());
    } while (!fetchedIdentity);

    return identity;
}
