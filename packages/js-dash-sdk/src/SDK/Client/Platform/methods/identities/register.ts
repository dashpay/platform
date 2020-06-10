import { Transaction } from "@dashevo/dashcore-lib";
// @ts-ignore
import {utils} from "@dashevo/wallet-lib";

import {Platform} from "../../Platform";

/**
 * Register identities to the platform
 *
 * @param {Platform} this - bound instance class
 * @returns {Identity}
 */
export async function register(this: Platform): Promise<any> {
    const { client, dpp } = this;

    const account = await client.getWalletAccount();

    const burnAmount = 10000;

    //TODO : Here, we always use index 0. We might want to increment.
    // @ts-ignore
    const identityHDPrivateKey = account.getIdentityHDKey(0);

    // @ts-ignore
    const identityPrivateKey = identityHDPrivateKey.privateKey;
    // @ts-ignore
    const identityPublicKey = identityHDPrivateKey.publicKey;

    const identityAddress = identityPublicKey.toAddress().toString();
    const changeAddress = account.getUnusedAddress('internal').address;

    // @ts-ignore
    const lockTransaction = new Transaction();

    const output = {
        satoshis: burnAmount,
        address: identityAddress
    };

    const utxos = account.getUTXOS();
    const balance = account.getTotalBalance();

        if (balance < output.satoshis) {
            throw new Error(`Not enough balance (${balance}) to cover burn amount of ${burnAmount}`);
        }

    const selection = utils.coinSelection(utxos, [output]);

    // FIXME : Usage with a single utxo had estimated fee of 205.
    // But network failed us with 66: min relay fee not met.
    // Over-writing the value for now.
    selection.estimatedFee = 680;

    lockTransaction
        .from(selection.utxos)
        .addBurnOutput(output.satoshis, identityPublicKey._getID())
        // @ts-ignore
        .change(changeAddress)
        .fee(selection.estimatedFee);

    const UTXOHDPrivateKey = account.getPrivateKeys(selection.utxos.map((utxo: any) => utxo.address.toString()));

    // @ts-ignore
    const signingKeys = UTXOHDPrivateKey.map((hdprivateKey) => hdprivateKey.privateKey);

    // @ts-ignore
    // FIXME : Seems to fail with addBurnOutput ?
    // const signedLockTransaction = account.sign(lockTransaction, signingKeys);
    const signedLockTransaction = lockTransaction.sign(signingKeys);

    // @ts-ignore
    await account.broadcastTransaction(signedLockTransaction);

    // @ts-ignore
    const outPoint = signedLockTransaction.getOutPointBuffer(0);

    const identity = dpp.identity.create(outPoint, [identityPublicKey]);

    const identityCreateTransition = dpp.identity.createIdentityCreateTransition(identity);

    // FIXME : Need dpp to be a dependency of wallet-lib to deal with signing IdentityPublicKey (validation)
    // account.sign(identityPublicKeyModel, identityPrivateKey);

    identityCreateTransition.signByPrivateKey(identityPrivateKey);

    const result = await dpp.stateTransition.validateStructure(identityCreateTransition);

    if (!result.isValid()) {
        throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
    }

    await this.client.getDAPIClient().applyStateTransition(identityCreateTransition);

    // @ts-ignore
    return identity;
}

export default register;
