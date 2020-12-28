const Dash = require('dash');

const getDAPISeeds = require('./getDAPISeeds');

function createClientWithoutWallet() {
  return new Dash.Client({
    seeds: getDAPISeeds(),
    passFakeAssetLockProofForTests: process.env.NETWORK === 'regtest',
  });
}

module.exports = createClientWithoutWallet;
