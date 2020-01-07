const DashJS = require('dash');

const sdkOpts = {
  network: 'testnet'
};
const sdk = new DashJS.SDK(sdkOpts);

const platform = sdk.platform;

async function retrieveName(){
  const user = await platform.names.get('alice');
  console.dir({user}, {depth:5});
}
retrieveName();
