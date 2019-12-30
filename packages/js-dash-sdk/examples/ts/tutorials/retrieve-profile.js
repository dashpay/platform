import DashJS from "../../../src";

const network = "testnet";
const sdkOpts = {
  network,
};
const sdk = new DashJS.SDK(sdkOpts);

const platform = sdk.platform;

async function retrieveProfile(){
  const user = await platform.identities.get('bob');
  console.dir({user});
}
retrieveProfile();
