const Dash = require('dash');

const clientOpts = {
  network: 'testnet'
};
const client = new Dash.Client(clientOpts);

const platform = client.platform;

async function retrieveName(){
  const user = await platform.names.get('alice');
  console.dir({user}, {depth:5});
}
retrieveName();
