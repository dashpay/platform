import { Transaction } from "@dashevo/dashcore-lib";
import { Platform } from "../../../Platform";

import { createFakeInstantLock } from "../../../../../../utils/createFakeIntantLock";

/**
 * Creates a funding transaction for the platform identity and returns one-time key to sign the state transition
 * @param {Platform} platform
 * @param {Transaction} assetLockTransaction
 * @param {number} outputIndex - index of the funding output in the asset lock transaction
 * @return {AssetLockProof} - asset lock proof to be used in the state transition
 * that can be used to sign registration/top-up state transition
 */
export default async function createAssetLockProof(platform : Platform, assetLockTransaction: Transaction, outputIndex: number): Promise<any> {
    await platform.initialize();

    const account = await platform.client.getWalletAccount();
    const { dpp } = platform;

    // Create poof that the transaction won't be double spend

    const {
      promise: instantLockPromise,
      cancel: cancelInstantLock
    } = account.waitForInstantLock(assetLockTransaction.hash);

    const {
      promise: txMetadataPromise,
      cancel: cancelTxMetadata,
    } = account.waitForTxMetadata(assetLockTransaction.hash);

    let instantLock;

    try {
        instantLock = await ;
    } catch (e) {
        // if block is mined before the transaction is instant locked instant lock won't be sent
        if (!e.message || !e.message.startsWith('InstantLock waiting period for transaction')) {
          throw e;
        }
    }

    // TODO: We should fallback to chainlock proofs instead
    instantLock = createFakeInstantLock(assetLockTransaction.hash)

    // @ts-ignore
    return dpp.identity.createInstantAssetLockProof(instantLock, assetLockTransaction, outputIndex);
}
