const Dash = require('dash');
const { NodeForage } = require('nodeforage');

const { contractId } = require('@dashevo/dpns-contract/lib/systemIds');

const getDAPISeeds = require('./getDAPISeeds');

let faucetClient;

const forage = new NodeForage({ name: 'faucet-wallet' });
console.log(process.env);
function createFaucetClient() {
  const seeds = getDAPISeeds();

  const clientOpts = {
    seeds,
    network: process.env.NETWORK,
    apps: {
      dpns: {
        contractId,
      },
    },
  };

  // TODO: Consider implementing .env flag that will enable/disable storage adapter
  const walletOptions = {
    privateKey: process.env.FAUCET_PRIVATE_KEY,
    adapter: forage,
  };

  if (process.env.SKIP_SYNC_BEFORE_HEIGHT) {
    walletOptions.unsafeOptions = {
      skipSynchronizationBeforeHeight: process.env.SKIP_SYNC_BEFORE_HEIGHT,
    };
  }

  faucetClient = new Dash.Client({
    ...clientOpts,
    wallet: walletOptions,
  });

  return faucetClient;
}

module.exports = createFaucetClient;
