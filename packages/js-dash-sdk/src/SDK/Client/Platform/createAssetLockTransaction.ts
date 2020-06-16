import {PrivateKey, Transaction} from "@dashevo/dashcore-lib";
import {utils} from "@dashevo/wallet-lib";
import {Platform} from "./Platform";

export default async function createAssetLockTransaction(platform : Platform, fundingAmount): Promise<{ transaction: Transaction, privateKey: PrivateKey }> {
    const account = await platform.client.getWalletAccount();

    const identityAddressInfo = account.getUnusedAddress();
    const [identityHDPrivateKey] = account.getPrivateKeys([identityAddressInfo.address]);

    // @ts-ignore
    const assetLockPrivateKey = identityHDPrivateKey.privateKey;
    const assetLockPublicKey = assetLockPrivateKey.toPublicKey();

    const identityAddress = assetLockPublicKey.toAddress(platform.client.network).toString();
    const changeAddress = account.getUnusedAddress('internal').address;

    const lockTransaction = new Transaction(undefined);

    const output = {
        satoshis: fundingAmount,
        address: identityAddress
    };

    // TODO: Find another way to mark the address as used
    const outputToMarkItUsed = {
        satoshis: 10000,
        address: identityAddress
    };

    const utxos = account.getUTXOS();
    const balance = account.getTotalBalance();

    if (balance < output.satoshis) {
        throw new Error(`Not enough balance (${balance}) to cover burn amount of ${fundingAmount}`);
    }

    const selection = utils.coinSelection(utxos, [output, outputToMarkItUsed]);

    lockTransaction
        .from(selection.utxos)
        // @ts-ignore
        .addBurnOutput(output.satoshis, assetLockPublicKey._getID())
        // @ts-ignore
        .to(identityAddressInfo.address, 10000)
        // @ts-ignore
        .change(changeAddress)

    const utxoAddresses = selection.utxos.map((utxo: any) => utxo.address.toString());

    // @ts-ignore
    const utxoHDPrivateKey = account.getPrivateKeys(utxoAddresses);

    // @ts-ignore
    const signingKeys = utxoHDPrivateKey.map((hdprivateKey) => hdprivateKey.privateKey);

    const transaction = lockTransaction.sign(signingKeys);

    return {
        transaction,
        privateKey: assetLockPrivateKey
    };
}
