const Dash = require('dash');

const { contractId } = require('@dashevo/dpns-contract/lib/systemIds');

const getDAPISeeds = require('./getDAPISeeds');

function createClientWithoutWallet() {
  const dapiAddresses = (process.env.DAPI_ADDRESSES || '')
    .split(',')
    .map((address) => address.trim())
    .filter(Boolean);

  return new Dash.Client({
    ...(dapiAddresses.length > 0
      ? { dapiAddresses }
      : { seeds: getDAPISeeds() }),
    network: process.env.NETWORK,
    timeout: 25000,
    apps: {
      dpns: {
        contractId,
      },
    },
  });
}

module.exports = createClientWithoutWallet;
