import { Transaction } from '@dashevo/dashcore-lib';
import { Platform } from '../../../Platform';

const { InstantLockTimeoutError, TxMetadataTimeoutError } = require('@dashevo/wallet-lib/src/errors');

/**
 * Creates a funding transaction for the platform identity
 *  and returns one-time key to sign the state transition
 * @param {Platform} this
 * @param {Transaction} assetLockTransaction
 * @param {number} outputIndex - index of the funding output in the asset lock transaction
 * @return {AssetLockProof} - asset lock proof to be used in the state transition
 * that can be used to sign registration/top-up state transition
 */
export async function createAssetLockProof(
  this : Platform,
  assetLockTransaction: Transaction,
  outputIndex: number,
): Promise<any> {
  const platform = this;
  await platform.initialize();

  const account = await platform.client.getWalletAccount();
  const { wasmDpp } = platform;

  // Create poof that the transaction won't be double spend

  const {
    promise: instantLockPromise,
    cancel: cancelInstantLock,
  } = account.waitForInstantLock(assetLockTransaction.hash);

  const {
    promise: txMetadataPromise,
    cancel: cancelTxMetadata,
  } = account.waitForTxMetadata(assetLockTransaction.hash);

  let cancelObtainCoreChainLockedHeight;

  let rejectTimer;

  // @ts-ignore
  const rejectionTimeout = account.waitForTxMetadataTimeout > account.waitForInstantLockTimeout
  // wait for platform to sync core chain locked height
  // @ts-ignore
    ? account.waitForTxMetadataTimeout + 360000
  // @ts-ignore
    : account.waitForInstantLockTimeout;

  return Promise.race([
    // Wait for Instant Lock
    instantLockPromise
      .then((instantLock) => {
        clearTimeout(rejectTimer);

        cancelTxMetadata();

        if (cancelObtainCoreChainLockedHeight) {
          cancelObtainCoreChainLockedHeight();
        }

        // @ts-ignore
        return wasmDpp.identity.createInstantAssetLockProof(
          instantLock.toBuffer(),
          assetLockTransaction.toBuffer(),
          outputIndex,
        );
      })
      .catch((error) => {
        if (error instanceof InstantLockTimeoutError) {
          // Instant Lock is timed out.
          // Allow chain proof to win the race
          return new Promise(() => {});
        }

        return Promise.reject(error);
      }),

    // Wait for transaction is mined and platform chain synced core height to the transaction height
    txMetadataPromise
      .then((assetLockMetadata) => platform.identities.utils
      // @ts-ignore
        .waitForCoreChainLockedHeight(assetLockMetadata.height)
        .then(({ promise, cancel }) => {
          cancelObtainCoreChainLockedHeight = cancel;

          return promise;
        })
        .then(() => {
          clearTimeout(rejectTimer);
          cancelInstantLock();

          // Change endianness of raw txId bytes in outPoint to match expectations of dashcore-rust
          let outPointBuffer = assetLockTransaction.getOutPointBuffer(outputIndex);
          const txIdBuffer = outPointBuffer.slice(0, 32);
          const outputIndexBuffer = outPointBuffer.slice(32);
          txIdBuffer.reverse();
          outPointBuffer = Buffer.concat([txIdBuffer, outputIndexBuffer]);

          // @ts-ignore
          return wasmDpp.identity.createChainAssetLockProof(
            // @ts-ignore
            assetLockMetadata.height,
            outPointBuffer,
          );
        }))
      .catch((error) => {
        if (error instanceof TxMetadataTimeoutError) {
          // Instant Lock is timed out.
          // Allow instant proof to win the race
          return new Promise(() => {});
        }

        return Promise.reject(error);
      }),

    // Common timeout for getting proofs
    new Promise((_, reject) => {
      rejectTimer = setTimeout(() => {
        cancelTxMetadata();

        if (cancelObtainCoreChainLockedHeight) {
          cancelObtainCoreChainLockedHeight();
        }

        cancelInstantLock();

        reject(new Error('Asset Lock Proof creation timeout'));
      }, rejectionTimeout);
    }),
  ]);
}

export default createAssetLockProof;
