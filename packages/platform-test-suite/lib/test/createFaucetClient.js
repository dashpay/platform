const Dash = require('dash');

const getDAPISeeds = require('./getDAPISeeds');

let faucetClient;

function createFaucetClient() {
  if (faucetClient) {
    return faucetClient;
  }

  const seeds = getDAPISeeds();

  const clientOpts = {
    seeds,
    network: process.env.NETWORK,
    apps: {
      dpns: {
        contractId: process.env.DPNS_CONTRACT_ID,
      },
    },
  };

  return new Dash.Client({
    ...clientOpts,
    wallet: {
      privateKey: process.env.FAUCET_PRIVATE_KEY,
    },
  });
}

module.exports = createFaucetClient;
