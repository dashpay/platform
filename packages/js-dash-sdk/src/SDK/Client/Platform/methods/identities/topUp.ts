// @ts-ignore
import Identifier from "@dashevo/dpp/lib/Identifier";
import {Platform} from "../../Platform";

import { wait } from "../../../../../utils/wait";
import createAssetLockTransaction from "../../createAssetLockTransaction";
import createAssetLockProof from "./internal/createAssetLockProof";
import createIdentityTopUpTransition from "./internal/createIdnetityTopUpTransition";

/**
 * Register identities to the platform
 *
 * @param {Platform} this - bound instance class
 * @param {Identifier|string} identityId - id of the identity to top up
 * @param {number} amount - amount to top up in duffs
 * @returns {boolean}
 */
export async function topUp(this: Platform, identityId: Identifier | string, amount: number): Promise<any> {
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
    const assetLockProof = await createAssetLockProof(this, assetLockTransaction);

    // @ts-ignore
    const identityTopUpTransition = await createIdentityTopUpTransition(this, assetLockTransaction, assetLockOutputIndex, assetLockProof, assetLockPrivateKey, identityId);

    // Broadcast ST
    await client.getDAPIClient().platform.broadcastStateTransition(identityTopUpTransition.toBuffer());

    // Wait some time for propagation
    await wait(1000);

    return true;
}

export default topUp;
