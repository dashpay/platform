import DashJS from "../../../src";

const network = "testnet";
const sdkOpts = {
  network,
  mnemonic: "bring pledge solid dance age arena raise recycle orbit mango lyrics gorilla",
};
const sdk = new DashJS.SDK(sdkOpts);

const createIdentity = async function () {
  await sdk.isReady();

  let platform = sdk.platform;

  platform
      .identities
      .register('user')
      .then((identityId) => {
        console.log({identityId});
      });

};
createIdentity();
