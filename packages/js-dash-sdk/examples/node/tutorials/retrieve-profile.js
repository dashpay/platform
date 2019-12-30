const DashJS = require("../../../dist/dash.cjs.min");
const schema = require("../../schema.json");

const network = "testnet";
const opts = {
  network,
  // mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
  schemas: {dashpay: schema}
};
const sdk = new DashJS.SDK(opts);

let platform = sdk.platform;
let user = platform.identities.get('alice');
console.log(user);

