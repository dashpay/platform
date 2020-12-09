const Dash = require('dash');

const getDAPISeeds = require('./getDAPISeeds');

function createClientWithoutWallet() {
  return new Dash.Client({
    seeds: getDAPISeeds(),
    passFakeAssetLockProofForTests: true,
  });
}

module.exports = createClientWithoutWallet;
