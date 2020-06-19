const Dash = require('dash');

const getDAPISeeds = require('./getDAPISeeds');

function createClientWithoutWallet() {
  return new Dash.Client({
    seeds: getDAPISeeds(),
  });
}

module.exports = createClientWithoutWallet;
