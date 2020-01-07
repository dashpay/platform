const DashJS = require('dash');
const sdkOpts = {
  network: 'testnet',
  mnemonic:'your mnemonic here'
};
const identityId = 'your identity id';
const sdk = new DashJS.SDK(sdkOpts);

const registerName = async function () {
  let platform = sdk.platform;
  await sdk.isReady();

  const identity = await platform.identities.get(identityId);
  const nameRegistration = await platform.names.register('alice', identity);
  console.log({nameRegistration});
};
registerName();
