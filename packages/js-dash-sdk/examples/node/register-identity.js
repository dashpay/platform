const DashJS = require('dash');
const sdkOpts = {
  network: 'testnet',
  mnemonic:'your mnemonic here'
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
