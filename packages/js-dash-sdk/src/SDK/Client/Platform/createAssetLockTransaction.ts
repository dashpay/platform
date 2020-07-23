import { PrivateKey, Transaction } from "@dashevo/dashcore-lib";
import { utils } from "@dashevo/wallet-lib";
import { Platform } from "./Platform";

/**
 * Creates a funding transaction for the platform identity and returns one-time key to sign the state transition
 * @param {Platform} platform
 * @param {number} fundingAmount - amount of dash to fund the identity's credits
 * @return {{transaction: Transaction, privateKey: PrivateKey}} - transaction and one time private key
 * that can be used to sign registration/top-up state transition
 */
export default async function createAssetLockTransaction(platform : Platform, fundingAmount): Promise<{ transaction: Transaction, privateKey: PrivateKey }> {
    const account = await platform.client.getWalletAccount();

    // @ts-ignore
    const assetLockOneTimePrivateKey = new PrivateKey();
    const assetLockOneTimePublicKey = assetLockOneTimePrivateKey.toPublicKey();

    const identityAddress = assetLockOneTimePublicKey.toAddress(platform.client.network).toString();
    const changeAddress = account.getUnusedAddress('internal').address;

    const lockTransaction = new Transaction(undefined);

    const output = {
        satoshis: fundingAmount,
        address: identityAddress
    };

    const utxos = account.getUTXOS();
    const balance = account.getTotalBalance();

    if (balance < output.satoshis) {
        throw new Error(`Not enough balance (${balance}) to cover burn amount of ${fundingAmount}`);
    }

    const selection = utils.coinSelection(utxos, [output]);

    lockTransaction
        .from(selection.utxos)
        // @ts-ignore
        .addBurnOutput(output.satoshis, assetLockOneTimePublicKey._getID())
        .change(changeAddress);

    const utxoAddresses = selection.utxos.map((utxo: any) => utxo.address.toString());

    // @ts-ignore
    const utxoHDPrivateKey = account.getPrivateKeys(utxoAddresses);

    // @ts-ignore
    const signingKeys = utxoHDPrivateKey.map((hdprivateKey) => hdprivateKey.privateKey);

    const transaction = lockTransaction.sign(signingKeys);

    return {
        transaction,
        privateKey: assetLockOneTimePrivateKey
    };
}
