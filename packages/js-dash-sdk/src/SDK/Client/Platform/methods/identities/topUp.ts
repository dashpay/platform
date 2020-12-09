// @ts-ignore
import Identifier from "@dashevo/dpp/lib/Identifier";
import {Platform} from "../../Platform";

import { wait } from "../../../../../utils/wait";
import { createFakeInstantLock } from "../../../../../utils/createFakeIntantLock";
import createAssetLockTransaction from "../../createAssetLockTransaction";

/**
 * Register identities to the platform
 *
 * @param {Platform} this - bound instance class
 * @param {Identifier|string} identityId - id of the identity to top up
 * @param {number} amount - amount to top up in duffs
 * @returns {boolean}
 */
export async function topUp(this: Platform, identityId: Identifier | string, amount: number): Promise<any> {
    const { client, dpp, passFakeAssetLockProofForTests } = this;

    identityId = Identifier.from(identityId);

    const account = await client.getWalletAccount();

    const {
        transaction: assetLockTransaction,
        privateKey: assetLockPrivateKey,
        outputIndex: assetLockOutputIndex
    } = await createAssetLockTransaction(this, amount);

    // Broadcast Asset Lock transaction
    await account.broadcastTransaction(assetLockTransaction);

    // Wait some time for propagation
    await wait(1000);

    // Create ST
    // Get IS lock to proof that transaction won't be double spent
    let instantLock;
    // Create poof that the transaction won't be double spend
    if (passFakeAssetLockProofForTests) {
        instantLock = createFakeInstantLock(assetLockTransaction.hash);
    } else {
        instantLock = await account.waitForInstantLock(assetLockTransaction.hash);
    }
    // @ts-ignore
    const assetLockProof = await dpp.identity.createInstantAssetLockProof(instantLock);
    // @ts-ignore
    const identityTopUpTransition = dpp.identity.createIdentityTopUpTransition(
        identityId, assetLockTransaction, assetLockOutputIndex, assetLockProof
    );

    identityTopUpTransition.signByPrivateKey(assetLockPrivateKey);

    const result = await dpp.stateTransition.validateStructure(identityTopUpTransition);

    if (!result.isValid()) {
        throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
    }

    // Broadcast ST

    await client.getDAPIClient().platform.broadcastStateTransition(identityTopUpTransition.toBuffer());

    // Wait some time for propagation
    await wait(1000);

    return true;
}

export default topUp;
