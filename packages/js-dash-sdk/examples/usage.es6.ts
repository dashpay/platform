import DashJS from "../src/index";
import schema from "./schema.json";
import {SDKOpts} from "../src/DashJS/SDK/SDK";

const network = "testnet";
const opts: SDKOpts = {
    network,
    mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
    schema
};
const sdk = new DashJS.SDK(opts);
const acc = sdk.wallet.getAccount();
async function sendPayment(){
    const tx = await acc.createTransaction({recipient:{address:'yLptqWxjgTxtwKJuLHoGY222NnoeqYuN8h', amount:0.12}})
    console.log(tx)
}

async function readDocument() {
    const profile = await sdk.platform.fetchDocuments('profile',  opts)
    console.log(profile);
}
