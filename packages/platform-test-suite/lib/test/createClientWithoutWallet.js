const Dash = require('dash');

const { contractId } = require('@dashevo/dpns-contract/lib/systemIds');

const getDAPISeeds = require('./getDAPISeeds');

function createClientWithoutWallet() {
  return new Dash.Client({
    // seeds: getDAPISeeds(),
    dapiAddresses: [
      '34.210.237.116',
      '54.69.65.231',
      // '54.185.90.95',
      // '54.186.234.0',
      // '35.87.212.139',
      // '34.212.52.44',
      '34.217.47.197',
      '34.220.79.131',
      '18.237.212.176',
      '54.188.17.188',
      '34.210.1.159',
    ],
    network: process.env.NETWORK,
    apps: {
      dpns: {
        contractId,
      },
    },
  });
}

module.exports = createClientWithoutWallet;
