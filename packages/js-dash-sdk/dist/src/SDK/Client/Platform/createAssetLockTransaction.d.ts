import { PrivateKey, Transaction } from '@dashevo/dashcore-lib';
import { Platform } from './Platform';
/**
 * Creates a funding transaction for the platform identity
 *  and returns one-time key to sign the state transition
 * @param {Platform} this
 * @param {number} fundingAmount - amount of dash to fund the identity's credits
 * @return {Promise<{transaction: Transaction, privateKey: PrivateKey}>}
 *  - transaction and one time private key
 * that can be used to sign registration/top-up state transition
 */
export declare function createAssetLockTransaction(this: Platform, fundingAmount: any): Promise<{
    transaction: Transaction;
    privateKey: PrivateKey;
    outputIndex: number;
}>;
export default createAssetLockTransaction;
