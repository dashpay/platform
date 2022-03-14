const Dash = require('dash');

const { contractId } = require('@dashevo/dpns-contract/lib/systemIds');

const getDAPISeeds = require('./getDAPISeeds');

function createClientWithoutWallet() {
  return new Dash.Client({
    seeds: getDAPISeeds(),
    network: process.env.NETWORK,
    apps: {
      dpns: {
        contractId,
      },
    },
  });
}

module.exports = createClientWithoutWallet;
