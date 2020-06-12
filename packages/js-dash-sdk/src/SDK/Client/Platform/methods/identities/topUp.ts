import { Transaction } from "@dashevo/dashcore-lib";
// @ts-ignore
import {utils} from "@dashevo/wallet-lib";

import {Platform} from "../../Platform";

import { wait } from "../../../../../utils/wait";

/**
 * Register identities to the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string} identityId - id of the identity to top up
 * @param {number} amount - amount to top up in duffs
 * @returns {boolean}
 */
export async function topUp(this: Platform, identityId: string, amount: number): Promise<any> {
    const { client, dpp } = this;

    const account = await client.getWalletAccount();

    // TODO: this key needs to be incremented, but there's no agreement on how to derived it currently
    // @ts-ignore
    const identityHDPrivateKey = account.getIdentityHDKey(0);

    // @ts-ignore
    const identityPrivateKey = identityHDPrivateKey.privateKey;
    // @ts-ignore
    const identityPublicKey = identityHDPrivateKey.publicKey;

    // @ts-ignore
    const identityAddress = identityPublicKey.toAddress().toString();
    const changeAddress = account.getUnusedAddress('internal').address;

    let selection;
    // @ts-ignore
    const lockTransaction = new Transaction();

    const output = {
        satoshis: amount,
        address: identityAddress
    };

    const utxos = account.getUTXOS();
    const balance = account.getTotalBalance();

    if (balance < output.satoshis) {
        throw new Error(`Not enough balance (${balance}) to cover burn amount of ${amount}`)
    }

    selection = utils.coinSelection(utxos, [output]);

    // FIXME : Usage with a single utxo had estimated fee of 205.
    // But network failed us with 66: min relay fee not met.
    // Over-writing the value for now.
    selection.estimatedFee = 680;

    lockTransaction
        .from(selection.utxos)
        // @ts-ignore
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

    await wait(1000);

    // @ts-ignore
    const outPointBuffer = signedLockTransaction.getOutPointBuffer(0);

    const identityTopUpTransition = dpp.identity.createIdentityTopUpTransition(identityId, outPointBuffer);

    // FIXME : Need dpp to be a dependency of wallet-lib to deal with signing IdentityPublicKey (validation)
    // account.sign(identityPublicKeyModel, identityPrivateKey);

    identityTopUpTransition.signByPrivateKey(identityPrivateKey);

    const result = await dpp.stateTransition.validateStructure(identityTopUpTransition);

    if (!result.isValid()) {
        throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
    }

    const dapiClient = this.client.getDAPIClient();

    await dapiClient.applyStateTransition(identityTopUpTransition);

    // @ts-ignore
    return true;
}

export default topUp;
