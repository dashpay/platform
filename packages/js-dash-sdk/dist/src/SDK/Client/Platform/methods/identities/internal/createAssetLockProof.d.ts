import { Transaction } from '@dashevo/dashcore-lib';
import { Platform } from '../../../Platform';
/**
 * Creates a funding transaction for the platform identity
 *  and returns one-time key to sign the state transition
 * @param {Platform} this
 * @param {Transaction} assetLockTransaction
 * @param {number} outputIndex - index of the funding output in the asset lock transaction
 * @return {AssetLockProof} - asset lock proof to be used in the state transition
 * that can be used to sign registration/top-up state transition
 */
export declare function createAssetLockProof(this: Platform, assetLockTransaction: Transaction, outputIndex: number): Promise<any>;
export default createAssetLockProof;
