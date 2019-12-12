import DashJS from "../../src";
import schema from "../schema.json";
import {SDKOpts} from "../../src/DashJS/SDK/SDK";

const network = "testnet";
const opts: SDKOpts = {
    network,
    mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
    schemas: {
        dashpay: schema
    }
};
const sdk = new DashJS.SDK(opts);
const {wallet, platform} = sdk;

readDocument();

async function sendPayment() {
    if (!wallet) throw new Error('Missing wallet');
    const acc = wallet.getAccount();
    const tx = await acc.createTransaction({recipient: {address: 'yLptqWxjgTxtwKJuLHoGY222NnoeqYuN8h', amount: 0.12}})
    console.log(tx)
}

async function readDocument() {
    if (platform===undefined) throw new Error('Missing platform');
    const profile = await platform.documents.fetch('dashpay.profile', opts)
    console.log(profile);
}
