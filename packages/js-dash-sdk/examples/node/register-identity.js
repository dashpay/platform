const Dash = require('dash');
const clientOpts = {
  network: 'testnet',
  mnemonic:'your mnemonic here'
};
const client = new Dash.Client(clientOpts);

const createIdentity = async function () {
  await client.isReady();

  let platform = client.platform;

  platform
      .identities
      .register()
      .then((identityId) => {
        console.log({identityId});
      });

};
createIdentity();
