// @ts-ignore
import Identifier from "@dashevo/dpp/lib/Identifier";
import {Platform} from "../../Platform";

import createAssetLockTransaction from "../../createAssetLockTransaction";
import createAssetLockProof from "./internal/createAssetLockProof";
import createIdentityTopUpTransition from "./internal/createIdnetityTopUpTransition";
import broadcastStateTransition from "../../broadcastStateTransition";

/**
 * Register identities to the platform
 *
 * @param {Platform} this - bound instance class
 * @param {Identifier|string} identityId - id of the identity to top up
 * @param {number} amount - amount to top up in duffs
 * @returns {boolean}
 */
export async function topUp(this: Platform, identityId: Identifier | string, amount: number): Promise<any> {
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

    // @ts-ignore
    const identityTopUpTransition = await createIdentityTopUpTransition(this, assetLockProof, assetLockPrivateKey, identityId);

    // Broadcast ST
    await broadcastStateTransition(this, identityTopUpTransition);

    return true;
}

export default topUp;
