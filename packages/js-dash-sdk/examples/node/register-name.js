const Dash = require('dash');
const clientOpts = {
  network: 'testnet',
  mnemonic:'your mnemonic here'
};
const identityId = 'your identity id';
const client = new Dash.Client(clientOpts);

const registerName = async function () {
  let platform = client.platform;
  await client.isReady();

  const identity = await platform.identities.get(identityId);
  const nameRegistration = await platform.names.register('alice', identity);
  console.log({nameRegistration});
};
registerName();
