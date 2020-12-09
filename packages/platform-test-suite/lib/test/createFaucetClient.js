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

  faucetClient = new Dash.Client({
    ...clientOpts,
    wallet: {
      privateKey: process.env.FAUCET_PRIVATE_KEY,
    },
    passFakeAssetLockProofForTests: true,
  });

  return faucetClient;
}

module.exports = createFaucetClient;
